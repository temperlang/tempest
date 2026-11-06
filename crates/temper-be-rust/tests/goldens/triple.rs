use std::sync::OnceLock;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            println!("triple(14) = {}", triple(14));
            Ok(())
        })
        .clone()
}

pub(crate) fn triple(n: i32) -> i32 {
    n.wrapping_mul(3)
}
