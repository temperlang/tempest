//! The terminal: ANSI escape codes, which Windows' console takes once
//! its virtual terminal processing is on, and, for tests on Windows,
//! sending Ctrl-Break to a process group.

/// Whether standard output takes ANSI escape codes, turning on the
/// console's virtual terminal processing if it's off.
#[cfg(windows)]
pub fn enable_ansi() -> bool {
    windows::enable_ansi()
}

/// Whether standard output takes ANSI escape codes: elsewhere than
/// Windows, they're there already.
#[cfg(not(windows))]
pub fn enable_ansi() -> bool {
    true
}

/// Sends Ctrl-Break to a console process group, as tests do to check
/// that it stops the input running (Ctrl-C can't be sent to one process
/// alone); whether it was sent. Ctrl events go to processes sharing the
/// sender's console; without one, as under some CI services, this
/// process attaches to the group leader's.
#[cfg(windows)]
pub fn send_ctrl_break(group: u32) -> bool {
    windows::send_ctrl_break(group)
}

#[cfg(windows)]
mod windows {
    use windows_sys::Win32::System::Console::{
        AttachConsole, CTRL_BREAK_EVENT, ENABLE_VIRTUAL_TERMINAL_PROCESSING, FreeConsole,
        GenerateConsoleCtrlEvent, GetConsoleMode, GetStdHandle, STD_OUTPUT_HANDLE, SetConsoleMode,
    };

    pub fn enable_ansi() -> bool {
        // SAFETY: takes a constant and returns a handle, or null.
        let out = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        if out.is_null() {
            return false;
        }
        let mut mode = 0u32;
        // SAFETY: this process's own standard output handle, and a valid
        // pointer for the mode; the call fails if it isn't a console.
        if unsafe { GetConsoleMode(out, &mut mode) } == 0 {
            return false;
        }
        if mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING != 0 {
            return true;
        }
        // SAFETY: the same console handle, and a plain mode value.
        unsafe { SetConsoleMode(out, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0 }
    }

    pub fn send_ctrl_break(group: u32) -> bool {
        if send(group) {
            return true;
        }
        // SAFETY: detaches this process from its console, if any; its
        // standard handles may stop working, which tests don't mind.
        unsafe { FreeConsole() };
        // SAFETY: a plain process id; it fails if there's no such
        // process or console.
        let attached = unsafe { AttachConsole(group) } != 0;
        attached && send(group)
    }

    fn send(group: u32) -> bool {
        // SAFETY: plain values; it fails if there's no such group.
        unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, group) != 0 }
    }
}
