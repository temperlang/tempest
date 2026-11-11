//! The web page's functions that the wasm build (`temper-wasm`) imports,
//! from the import module `env`, as a browser's tab offers what an
//! operating system would:
//!
//! - `host_log(ptr, len)`: a line a REPL input logged, as UTF-8 at
//!   `ptr` in the module's memory, as it's logged, so a long run shows
//!   its output while it runs.
//! - `host_now() -> f64`: the time, in milliseconds since the Unix
//!   epoch, as JavaScript's `Date.now()`, for std's `temporal`, since
//!   `std::time::SystemTime::now()` panics on wasm32-unknown-unknown.
//!
//! Hand-written declarations, as there are no bindings for a page's own
//! functions. Elsewhere than wasm, nothing is imported: [`log`] does
//! nothing, and [`now_ms`] reads the system clock.

/// Gives the page a line an input logged.
#[cfg(target_arch = "wasm32")]
pub fn log(line: &str) {
    // SAFETY: the page only reads `len` bytes at `ptr`, which `line`
    // holds for the call.
    unsafe { imports::host_log(line.as_ptr(), line.len()) }
}

/// Elsewhere than wasm, there's no page to give a line to.
#[cfg(not(target_arch = "wasm32"))]
pub fn log(_line: &str) {}

/// Milliseconds since the Unix epoch, from the page's clock.
#[cfg(target_arch = "wasm32")]
pub fn now_ms() -> f64 {
    // SAFETY: takes nothing and returns a number.
    unsafe { imports::host_now() }
}

/// Milliseconds since the Unix epoch, from the system clock, or 0
/// before it.
#[cfg(not(target_arch = "wasm32"))]
pub fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
}

#[cfg(target_arch = "wasm32")]
mod imports {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        pub fn host_log(ptr: *const u8, len: usize);
        pub fn host_now() -> f64;
    }
}
