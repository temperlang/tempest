//! Gives the wasm build a 16 MiB stack, rather than the linker's default
//! 1 MiB, for the settle loop's and the interpreter's recursion, as the
//! native binaries run them on a thread with a large stack; and caps its
//! memory at 1 GiB, rather than wasm32's 4 GiB, so a runaway allocation,
//! as a REPL input's, traps rather than taking a phone's tab down with
//! it.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg-cdylib=-zstack-size=16777216");
        println!("cargo:rustc-link-arg-cdylib=--max-memory=1073741824");
    }
}
