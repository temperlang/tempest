use std::sync::OnceLock;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let mut a: i32 = 1;
            a = a;
            let c = times_magic(a);
            println!("number = {c}");
            Ok(())
        })
        .clone()
}

pub fn times_magic(n: i32) -> i32 {
    n.wrapping_mul(crate::exporter::magic_number())
}
