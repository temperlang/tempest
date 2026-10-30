use std::sync::OnceLock;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            println!("1.min(3)");
            println!("{}", 1.min(3));
            println!("3.min(1)");
            println!("{}", 3.min(1));
            println!("1.max(3)");
            println!("{}", 1.max(3));
            println!("3.max(1)");
            println!("{}", 3.max(1));
            Ok(())
        })
        .clone()
}
