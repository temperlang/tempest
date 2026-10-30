use std::sync::OnceLock;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            println!("both {}", both(2));
            Ok(())
        })
        .clone()
}

pub fn both(n: i32) -> i32 {
    modules::sum(n)
        .wrapping_add(modules::helpers::twice(n))
        .wrapping_add(modules::helpers::base())
}
