use std::sync::OnceLock;
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            say_hello("World");
            Ok(())
        })
        .clone()
}

pub(crate) fn say_hello(όνομα: impl ToArcString) {
    let όνομα = όνομα.to_arc_string();
    println!("Hello, {όνομα}!");
}
