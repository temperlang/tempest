//! Child processes, each in a process group of its own, so what it
//! starts in turn goes with it:
//!
//! - [`own_group`] starts a command in a new group, which on Linux gets
//!   SIGKILL when tempest dies (`PR_SET_PDEATHSIG`).
//! - [`Group`] registers a child's group, killed when the `Group` is
//!   dropped, or when tempest gets SIGINT, SIGTERM, or SIGHUP; tempest
//!   then ends as the signal would have ended it.
//! - [`limit_data`] caps what a command may write, on 64-bit Linux
//!   (`RLIMIT_DATA`).
//!
//! Elsewhere than Unix, these do nothing: a command runs in tempest's
//! group, and only it can be killed, by [`std::process::Child::kill`].

use std::process::{Child, Command};

/// Starts a command in a process group of its own, which on Linux gets
/// SIGKILL when tempest dies, or fails to start if tempest died while
/// starting it. It's when the thread that starts it ends, strictly, but
/// tools wait for what they start. Nothing elsewhere than Unix.
pub fn own_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
        #[cfg(target_os = "linux")]
        linux::die_with_parent(command);
    }
    #[cfg(not(unix))]
    let _ = command;
}

/// Has a command lower its `RLIMIT_DATA` to `bytes` where it's higher,
/// on 64-bit Linux; nothing elsewhere.
pub fn limit_data(command: &mut Command, bytes: u64) {
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    linux::limit_data(command, bytes);
    #[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
    let _ = (command, bytes);
}

/// A child's process group, registered so a signal to tempest kills it,
/// and killed when dropped. Up to 64 are registered at once; past that,
/// only the parent-death signal, on Linux, and the `Group`'s drop kill
/// one.
pub struct Group {
    #[cfg(unix)]
    slot: Option<unix::Slot>,
}

impl Group {
    /// Registers the group of a child started by a command that
    /// [`own_group`] set up, which is the child's id, as it leads it.
    #[cfg_attr(not(unix), allow(unused_variables))]
    pub fn register(child: &Child) -> Group {
        Group {
            #[cfg(unix)]
            slot: unix::register(child.id()),
        }
    }

    /// Kills every process in the group. After the child's been waited
    /// for, its id could, in principle, start another group, but Linux
    /// hands out ids in turn, so not so soon.
    pub fn kill(&self) {
        #[cfg(unix)]
        if let Some(slot) = &self.slot {
            unix::kill_group(slot.group);
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
    //! The groups registered, and the signal handler that kills them.

    use std::sync::Once;
    use std::sync::atomic::{AtomicI32, Ordering};

    /// The process groups running, by their ids, 0 for a free slot,
    /// which the signal handler reads; tools may run on several threads
    /// at once, as in backends' tests.
    static GROUPS: [AtomicI32; 64] = [const { AtomicI32::new(0) }; 64];

    /// A registered group, its slot freed when dropped.
    pub struct Slot {
        index: usize,
        pub group: i32,
    }

    impl Drop for Slot {
        fn drop(&mut self) {
            GROUPS[self.index].store(0, Ordering::SeqCst);
        }
    }

    pub fn register(id: u32) -> Option<Slot> {
        install_handlers();
        let group = i32::try_from(id).ok()?;
        let index = GROUPS.iter().position(|slot| {
            slot.compare_exchange(0, group, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        })?;
        Some(Slot { index, group })
    }

    /// Sends SIGKILL to every process in a group; async-signal-safe.
    pub fn kill_group(group: i32) {
        // SAFETY: kill takes plain integers and touches no memory of
        // ours; it's async-signal-safe, so the handler may call this.
        unsafe { libc::kill(-group, libc::SIGKILL) };
    }

    /// On SIGINT, SIGTERM, and SIGHUP, kills every registered group,
    /// then ends tempest as the signal would have, unless tempest was
    /// set to ignore it.
    fn install_handlers() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let handler: extern "C" fn(libc::c_int) = on_signal;
            for sig in [libc::SIGHUP, libc::SIGINT, libc::SIGTERM] {
                // SAFETY: the handler only reads atomics and makes
                // async-signal-safe calls (kill, signal, raise).
                let old = unsafe { libc::signal(sig, handler as libc::sighandler_t) };
                if old == libc::SIG_IGN {
                    // SAFETY: back to ignoring it, as it was.
                    unsafe { libc::signal(sig, libc::SIG_IGN) };
                }
            }
        });
    }

    extern "C" fn on_signal(sig: libc::c_int) {
        for slot in &GROUPS {
            let group = slot.load(Ordering::SeqCst);
            if group > 0 {
                kill_group(group);
            }
        }
        // SAFETY: the default action, set with an async-signal-safe
        // call, for the signal raised next.
        unsafe { libc::signal(sig, libc::SIG_DFL) };
        // SAFETY: async-signal-safe; the signal, blocked while this
        // handler runs, comes once it returns, with its default action.
        unsafe { libc::raise(sig) };
    }
}

#[cfg(target_os = "linux")]
mod linux {
    //! The parent-death signal and the memory limit, Linux's own, set
    //! in the child between `fork` and `exec`.

    use std::io;
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    pub fn die_with_parent(command: &mut Command) {
        let parent = libc::pid_t::try_from(std::process::id()).unwrap_or(0);
        // SAFETY: what runs in the child, `die_now_or_with`, makes only
        // async-signal-safe calls and doesn't allocate.
        unsafe { command.pre_exec(move || die_now_or_with(parent)) };
    }

    /// In the child: SIGKILL when `parent` dies, or an error now if it
    /// already has.
    fn die_now_or_with(parent: libc::pid_t) -> io::Result<()> {
        // SAFETY: PR_SET_PDEATHSIG takes one integer argument, as
        // given, and touches no memory of ours.
        unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL as libc::c_ulong) };
        // SAFETY: getppid has no preconditions.
        if unsafe { libc::getppid() } != parent {
            // An error without a message, which would allocate; no one
            // is left to read it.
            return Err(io::ErrorKind::Other.into());
        }
        Ok(())
    }

    #[cfg(target_pointer_width = "64")]
    pub fn limit_data(command: &mut Command, bytes: u64) {
        // SAFETY: what runs in the child, `lower_data_limit`, makes only
        // async-signal-safe calls (getrlimit, setrlimit) and doesn't
        // allocate.
        unsafe {
            command.pre_exec(move || {
                lower_data_limit(bytes);
                Ok(())
            })
        };
    }

    /// Lowers this process's `RLIMIT_DATA` to `bytes`, where it's higher.
    #[cfg(target_pointer_width = "64")]
    fn lower_data_limit(bytes: u64) {
        let mut limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: a pointer to a live `rlimit`, for getrlimit to fill.
        if unsafe { libc::getrlimit(libc::RLIMIT_DATA, &mut limit) } != 0 {
            return;
        }
        let lower = libc::rlimit {
            rlim_cur: limit.rlim_cur.min(bytes),
            rlim_max: limit.rlim_max,
        };
        // SAFETY: a pointer to a live `rlimit`, which setrlimit reads.
        unsafe { libc::setrlimit(libc::RLIMIT_DATA, &lower) };
    }
}
