//! The processes that backends' tools start ([`crate::tools::output`] and
//! [`crate::tools::program`]), kept from outliving tempest or running
//! forever. Each starts in a process group of its own, so what it starts
//! in turn (cargo's test binaries, `dotnet run`'s program) goes with it:
//!
//! - **A time limit.** A command that runs past its limit
//!   ([`set_timeout`], `TEMPEST_TIMEOUT`, else [`DEFAULT_TIMEOUT`]) has
//!   its group killed, and fails, "timed out after N s". `tempest run`'s
//!   program has no limit unless one is given, as it may be a server.
//! - **Nothing left behind.** Once a command ends, or times out, what's
//!   left of its group is killed, as are the groups still running if
//!   tempest gets SIGINT, SIGTERM, or SIGHUP; then tempest ends as the
//!   signal would have ended it.
//! - **Linux:** each process gets SIGKILL when tempest dies
//!   (`PR_SET_PDEATHSIG`), even by SIGKILL, which no handler sees; what
//!   *it* started outlives it then, as the parent-death signal isn't
//!   inherited. Commands that ask ([`limit_memory`]) may write at most
//!   `TEMPEST_MEMORY_LIMIT` MiB, 4096 by default (`RLIMIT_DATA`).
//!
//! On Windows, a command that times out is killed, but not what it
//! started, and nothing dies with tempest yet (a Job Object would do
//! both).
//!
//! Standard input stays tempest's for `program`; as the program's group
//! isn't the terminal's foreground one, a program that read from the
//! terminal would stop, but Temper has no way to read it.

use std::io::Read;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Each tool command's time limit unless `--timeout` or
/// `TEMPEST_TIMEOUT` says otherwise: ten minutes.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);

/// Memory a command that asks for a limit may write, in MiB, unless
/// `TEMPEST_MEMORY_LIMIT` says otherwise.
pub const DEFAULT_MEMORY_LIMIT_MIB: u64 = 4096;

/// The limit set by [`set_timeout`], if it was called.
static TIMEOUT: OnceLock<Option<Duration>> = OnceLock::new();

/// Sets the time limit for each command tools start, `None` for none, in
/// place of `TEMPEST_TIMEOUT`'s, as `--timeout` does; the first call
/// counts.
pub fn set_timeout(limit: Option<Duration>) {
    let _ = TIMEOUT.set(limit);
}

/// A time limit as `--timeout` and `TEMPEST_TIMEOUT` give it: whole
/// seconds above 0, or `none`; or what it should be.
pub fn parse_seconds(text: &str) -> Result<Option<Duration>, String> {
    whole(text)
        .map(|n| n.map(Duration::from_secs))
        .ok_or_else(|| format!("whole seconds above 0, or none, not {text}"))
}

/// A memory limit as `TEMPEST_MEMORY_LIMIT` gives it: MiB above 0, or
/// `none`; or what it should be.
pub fn parse_mib(text: &str) -> Result<Option<u64>, String> {
    whole(text).ok_or_else(|| format!("MiB above 0, or none, not {text}"))
}

/// A whole number above 0, or `none`.
fn whole(text: &str) -> Option<Option<u64>> {
    match text {
        "none" => Some(None),
        _ if text.bytes().all(|b| b.is_ascii_digit()) => {
            text.parse::<u64>().ok().filter(|&n| n > 0).map(Some)
        }
        _ => None,
    }
}

/// The time limit given, by [`set_timeout`] or else `TEMPEST_TIMEOUT`
/// (where it's valid), if any was.
fn given_timeout() -> Option<Option<Duration>> {
    if let Some(limit) = TIMEOUT.get() {
        return Some(*limit);
    }
    let text = std::env::var("TEMPEST_TIMEOUT").ok()?;
    parse_seconds(&text).ok()
}

/// A tool command's time limit.
fn tool_timeout() -> Option<Duration> {
    given_timeout().unwrap_or(Some(DEFAULT_TIMEOUT))
}

/// A program's time limit, none unless one was given.
fn program_timeout() -> Option<Duration> {
    given_timeout().flatten()
}

/// Limits how much memory a command may write, on Linux, to
/// `TEMPEST_MEMORY_LIMIT` MiB (where it's valid), else
/// [`DEFAULT_MEMORY_LIMIT_MIB`]; `none` lifts it. Only for commands whose
/// writable memory is what they use: not the JVM's, .NET's, or Node's,
/// which set aside more than they need, nor a compiler's.
///
/// It's `RLIMIT_DATA`, not `RLIMIT_AS`: address space reserved but not
/// yet writable, as glibc's malloc arenas and thread stacks, doesn't
/// count, so a limit that stops a runaway doesn't fail a run that's
/// fine. A limit already lower stays.
pub fn limit_memory(command: &mut Command) -> &mut Command {
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    {
        let mib = match std::env::var("TEMPEST_MEMORY_LIMIT") {
            Ok(text) => parse_mib(&text).unwrap_or(Some(DEFAULT_MEMORY_LIMIT_MIB)),
            Err(_) => Some(DEFAULT_MEMORY_LIMIT_MIB),
        };
        if let Some(mib) = mib {
            let bytes = mib.saturating_mul(1 << 20);
            use std::os::unix::process::CommandExt;
            // SAFETY: getrlimit and setrlimit are async-signal-safe, and
            // nothing here allocates.
            unsafe {
                command.pre_exec(move || {
                    linux::lower_data_limit(bytes);
                    Ok(())
                });
            }
        }
    }
    command
}

/// Runs a command, its output captured and its standard input empty, as
/// [`Command::output`] does, within a tool command's time limit.
pub fn output(command: &mut Command, what: &str) -> Result<Output, String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let (status, stdout, stderr, limit) = run(command, what, tool_timeout())?;
    let output = Output {
        status,
        stdout,
        stderr,
    };
    match limit {
        None => Ok(output),
        // With the end of what it printed, if anything.
        Some(limit) => {
            let printed = crate::tools::tool_output(&output);
            let printed = printed.trim_end();
            match printed.is_empty() {
                true => Err(timed_out(limit)),
                false => Err(format!("{}\n{printed}", timed_out(limit))),
            }
        }
    }
}

/// Runs a command with tempest's standard input, output, and error, as
/// [`Command::status`] does, with no time limit unless one was given;
/// whether it succeeded.
pub fn program(command: &mut Command, what: &str) -> Result<bool, String> {
    match run(command, what, program_timeout())? {
        (status, _, _, None) => Ok(status.success()),
        (_, _, _, Some(limit)) => Err(timed_out(limit)),
    }
}

fn timed_out(limit: Duration) -> String {
    format!("timed out after {} s", limit.as_secs())
}

/// Runs a command in a process group of its own until it ends or runs out
/// of time, killing what's left of the group either way: its status, what
/// it wrote to piped output, and its limit, if it ran out.
#[allow(clippy::type_complexity)]
fn run(
    command: &mut Command,
    what: &str,
    limit: Option<Duration>,
) -> Result<(ExitStatus, Vec<u8>, Vec<u8>, Option<Duration>), String> {
    #[cfg(unix)]
    unix::own_group(command);
    let mut child = command
        .spawn()
        .map_err(|e| format!("couldn't run {what}: {e}"))?;
    let group = Group::new(&child);
    let read = |pipe: Option<_>| {
        pipe.map(|mut pipe: Box<dyn Read + Send>| {
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                let _ = pipe.read_to_end(&mut bytes);
                bytes
            })
        })
    };
    let stdout = read(child.stdout.take().map(|p| Box::new(p) as _));
    let stderr = read(child.stderr.take().map(|p| Box::new(p) as _));
    let waited = wait(&mut child, &group, limit);
    // What it started and left running goes too, so the pipes close.
    drop(group);
    let join = |reader: Option<std::thread::JoinHandle<Vec<u8>>>| {
        reader.map_or_else(Vec::new, |r| r.join().unwrap_or_default())
    };
    let (stdout, stderr) = (join(stdout), join(stderr));
    let (status, out_of_time) = waited.map_err(|e| format!("couldn't run {what}: {e}"))?;
    Ok((
        status,
        stdout,
        stderr,
        out_of_time.then_some(limit).flatten(),
    ))
}

/// Waits for a child, polling, sooner at first, so short commands end
/// promptly, and killing its group if it runs past `limit`: its status,
/// and whether it ran out of time.
fn wait(
    child: &mut Child,
    group: &Group,
    limit: Option<Duration>,
) -> std::io::Result<(ExitStatus, bool)> {
    let deadline = limit.map(|l| Instant::now() + l);
    let mut pause = Duration::from_millis(1);
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok((status, false));
        }
        let now = Instant::now();
        if let Some(deadline) = deadline {
            if now >= deadline {
                group.kill();
                let _ = child.kill();
                return child.wait().map(|status| (status, true));
            }
            pause = pause.min(deadline - now);
        }
        std::thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(50));
    }
}

/// A child's process group, registered so a signal to tempest kills it,
/// and killed when dropped.
struct Group {
    #[cfg(unix)]
    slot: Option<unix::Slot>,
}

impl Group {
    #[cfg_attr(not(unix), allow(unused_variables))]
    fn new(child: &Child) -> Group {
        Group {
            #[cfg(unix)]
            slot: unix::register(child.id()),
        }
    }

    fn kill(&self) {
        #[cfg(unix)]
        if let Some(slot) = &self.slot {
            slot.kill();
        }
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(unix)]
mod unix {
    //! Process groups, and the signal handler that kills them.

    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use std::sync::Once;
    use std::sync::atomic::{AtomicI32, Ordering};

    const SIGHUP: i32 = 1;
    const SIGINT: i32 = 2;
    const SIGKILL: i32 = 9;
    const SIGTERM: i32 = 15;
    const SIG_DFL: usize = 0;
    const SIG_IGN: usize = 1;

    unsafe extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
        fn signal(sig: i32, handler: usize) -> usize;
        fn raise(sig: i32) -> i32;
    }

    /// The process groups running, by their ids, 0 for a free slot,
    /// which the signal handler reads; tools may run on several threads
    /// at once, as in backends' tests. A child past the 64th isn't
    /// registered, so only the parent-death signal, on Linux, and its
    /// own end, kill its group.
    static GROUPS: [AtomicI32; 64] = [const { AtomicI32::new(0) }; 64];

    /// Starts a command in a process group of its own, which on Linux
    /// gets SIGKILL when tempest dies.
    pub fn own_group(command: &mut Command) {
        command.process_group(0);
        #[cfg(target_os = "linux")]
        super::linux::die_with_parent(command);
    }

    /// A registered process group.
    pub struct Slot {
        index: usize,
        group: i32,
    }

    impl Slot {
        /// Kills every process in the group. After the child's been
        /// waited for, its id could, in principle, start another group,
        /// but Linux hands out ids in turn, so not so soon.
        pub fn kill(&self) {
            // SAFETY: kill has no memory effects.
            unsafe { kill(-self.group, SIGKILL) };
        }
    }

    impl Drop for Slot {
        fn drop(&mut self) {
            GROUPS[self.index].store(0, Ordering::SeqCst);
        }
    }

    /// Registers a child's group, which is its id, as it leads it.
    pub fn register(id: u32) -> Option<Slot> {
        install_handlers();
        let group = i32::try_from(id).ok()?;
        let index = GROUPS.iter().position(|slot| {
            slot.compare_exchange(0, group, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        })?;
        Some(Slot { index, group })
    }

    /// On SIGINT, SIGTERM, and SIGHUP, kills every registered group,
    /// then ends tempest as the signal would have, unless tempest was
    /// set to ignore it.
    fn install_handlers() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            for sig in [SIGHUP, SIGINT, SIGTERM] {
                // SAFETY: the handler only calls async-signal-safe
                // functions and reads atomics.
                unsafe {
                    if signal(sig, on_signal as usize) == SIG_IGN {
                        signal(sig, SIG_IGN);
                    }
                }
            }
        });
    }

    extern "C" fn on_signal(sig: i32) {
        for slot in &GROUPS {
            let group = slot.load(Ordering::SeqCst);
            if group > 0 {
                // SAFETY: kill is async-signal-safe.
                unsafe { kill(-group, SIGKILL) };
            }
        }
        // SAFETY: async-signal-safe; the signal, blocked while this
        // handler runs, comes once it returns, with its default action.
        unsafe {
            signal(sig, SIG_DFL);
            raise(sig);
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    //! The parent-death signal and the memory limit, Linux's own.

    use std::os::unix::process::CommandExt;
    use std::process::Command;

    const PR_SET_PDEATHSIG: i32 = 1;
    const SIGKILL: i32 = 9;
    #[cfg(target_pointer_width = "64")]
    const RLIMIT_DATA: i32 = 2;

    /// `struct rlimit`, whose `rlim_t` is 64 bits wide on 64-bit Linux.
    #[cfg(target_pointer_width = "64")]
    #[repr(C)]
    struct Rlimit {
        cur: u64,
        max: u64,
    }

    unsafe extern "C" {
        fn prctl(option: i32, ...) -> i32;
        fn getppid() -> i32;
        #[cfg(target_pointer_width = "64")]
        fn getrlimit(resource: i32, limit: *mut Rlimit) -> i32;
        #[cfg(target_pointer_width = "64")]
        fn setrlimit(resource: i32, limit: *const Rlimit) -> i32;
    }

    /// Has the command get SIGKILL when tempest dies, or right away if
    /// tempest died while starting it. It's when the thread that starts
    /// it ends, strictly, but tools wait for what they start.
    pub fn die_with_parent(command: &mut Command) {
        let parent = i32::try_from(std::process::id()).unwrap_or(0);
        // SAFETY: prctl and getppid are async-signal-safe, and nothing
        // here allocates.
        unsafe {
            command.pre_exec(move || {
                prctl(PR_SET_PDEATHSIG, SIGKILL as std::ffi::c_ulong);
                if getppid() != parent {
                    return Err(std::io::Error::other("tempest ended"));
                }
                Ok(())
            });
        }
    }

    /// Lowers this process's `RLIMIT_DATA` to `bytes`, where it's higher.
    #[cfg(target_pointer_width = "64")]
    pub fn lower_data_limit(bytes: u64) {
        let mut limit = Rlimit { cur: 0, max: 0 };
        // SAFETY: the pointers are to a live, properly laid out struct.
        unsafe {
            if getrlimit(RLIMIT_DATA, &mut limit) != 0 {
                return;
            }
            let cur = limit.cur.min(bytes);
            let max = limit.max;
            setrlimit(RLIMIT_DATA, &Rlimit { cur, max });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_mib, parse_seconds};
    use std::time::Duration;

    #[test]
    fn parses_limits() {
        assert_eq!(parse_seconds("90"), Ok(Some(Duration::from_secs(90))));
        assert_eq!(parse_seconds("none"), Ok(None));
        for wrong in ["0", "-5", "+5", "1.5", "lots", ""] {
            let why = format!("whole seconds above 0, or none, not {wrong}");
            assert_eq!(parse_seconds(wrong), Err(why));
        }
        assert_eq!(parse_mib("2048"), Ok(Some(2048)));
    }

    /// A command past its limit is killed, with what it started, and
    /// fails, saying so, with what it printed.
    #[cfg(unix)]
    #[test]
    fn times_out() {
        let started = std::time::Instant::now();
        let mut command = std::process::Command::new("sh");
        command
            .args(["-c", "echo started; sleep 30 & sleep 30; wait"])
            .stdout(std::process::Stdio::piped());
        let limit = Some(Duration::from_millis(300));
        let (_, stdout, _, out_of_time) = super::run(&mut command, "sh", limit).expect("sh runs");
        assert_eq!(out_of_time, limit);
        assert_eq!(String::from_utf8_lossy(&stdout), "started\n");
        // The background sleep held the pipe open; it's gone too.
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}
