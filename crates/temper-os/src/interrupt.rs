//! Ctrl-C, as a flag for the interpreter rather than the end of the
//! process: SIGINT on Unix, the console's Ctrl-C event on Windows, and
//! on Windows, Ctrl-Break too. While a Ctrl-C is still unhandled,
//! another one goes to the platform's default: on Unix, the next ends
//! tempest, and on Windows, that one does, for when tempest itself is
//! stuck. Elsewhere, nothing is installed, and the flag stays unset.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

static FLAG: OnceLock<Arc<AtomicBool>> = OnceLock::new();
/// Ctrl-Cs since the flag was last handled.
static PRESSES: AtomicU32 = AtomicU32::new(0);

/// The flag Ctrl-C sets, with the handler installed (again, on Unix,
/// where something else, as a line editor, may have taken SIGINT).
pub fn install() -> Arc<AtomicBool> {
    let flag = FLAG
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone();
    platform::install();
    flag
}

/// After a Ctrl-C is handled: the next ones count from none, and the
/// handler is back if a second one took it away.
pub fn reset() {
    if PRESSES.swap(0, Ordering::Relaxed) > 1 {
        platform::install();
    }
}

/// What the handler does: sets the flag, and says whether it's the
/// first press since the last was handled; if not, the platform's
/// default takes over. It only touches atomics, so a signal handler may
/// call it.
fn pressed() -> bool {
    if let Some(flag) = FLAG.get() {
        flag.store(true, Ordering::Relaxed);
    }
    PRESSES.fetch_add(1, Ordering::Relaxed) == 0
}

#[cfg(unix)]
mod platform {
    extern "C" fn on_sigint(_: libc::c_int) {
        if !super::pressed() {
            // SAFETY: `signal` is async-signal-safe; this restores the
            // default for the next SIGINT.
            unsafe { libc::signal(libc::SIGINT, libc::SIG_DFL) };
        }
    }

    pub fn install() {
        let handler: extern "C" fn(libc::c_int) = on_sigint;
        // SAFETY: the handler only touches atomics and calls `signal`,
        // which are async-signal-safe.
        unsafe { libc::signal(libc::SIGINT, handler as libc::sighandler_t) };
    }
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::System::Console::{
        CTRL_BREAK_EVENT, CTRL_C_EVENT, SetConsoleCtrlHandler,
    };
    use windows_sys::core::BOOL;

    extern "system" fn on_ctrl(kind: u32) -> BOOL {
        // Ctrl-Break as Ctrl-C (it alone can be sent to one process
        // group, as tests do). Handled unless it's the third in a row,
        // or another event, such as closing the console.
        let key = kind == CTRL_C_EVENT || kind == CTRL_BREAK_EVENT;
        (key && super::pressed()) as BOOL
    }

    /// Once: the handler stays, passing on what it doesn't handle.
    pub fn install() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        let handler: unsafe extern "system" fn(u32) -> BOOL = on_ctrl;
        ONCE.call_once(|| {
            // SAFETY: registers a handler that only touches atomics, and
            // stays valid, as a plain function, for the process's life.
            unsafe { SetConsoleCtrlHandler(Some(handler), 1) };
        });
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    pub fn install() {}
}
