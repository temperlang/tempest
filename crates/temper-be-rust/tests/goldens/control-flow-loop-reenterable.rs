use std::sync::OnceLock;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let mut i: i32 = 0;
            while i < 2 {
                let x = i;
                println!("{x}");
                i = i.wrapping_add(1);
            }
            Ok(())
        })
        .clone()
}
