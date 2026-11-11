//! Files on Unix, beyond std: opening a file relative to a directory's
//! descriptor, so the directory can't be swapped between checking it
//! and opening the file in it, and the user that checks are made for.
//! Nothing on other platforms.

#[cfg(unix)]
pub use unix::{effective_uid, open_at};

#[cfg(unix)]
mod unix {
    use std::ffi::CStr;
    use std::fs::File;
    use std::io;
    use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};

    /// The process's effective user ID (`geteuid`).
    pub fn effective_uid() -> u32 {
        // SAFETY: geteuid only reads the process's effective user ID.
        unsafe { libc::geteuid() }
    }

    /// Opens `name` in the directory `dir` (`openat`), with `open`'s
    /// `flags`, and `mode` for a file it creates.
    pub fn open_at(
        dir: impl AsFd,
        name: &CStr,
        flags: libc::c_int,
        mode: libc::mode_t,
    ) -> io::Result<File> {
        let dir = dir.as_fd().as_raw_fd();
        // SAFETY: a descriptor borrowed for the call and a NUL-terminated
        // name; `mode` goes as the variadic argument, promoted to an
        // unsigned int, as C would.
        let fd = unsafe { libc::openat(dir, name.as_ptr(), flags, libc::c_uint::from(mode)) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: openat just returned this descriptor, open, and
        // nothing else owns it.
        Ok(File::from(unsafe { OwnedFd::from_raw_fd(fd) }))
    }
}
