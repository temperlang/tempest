use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            enclose(1);
            Ok(())
        })
        .clone()
}

pub(crate) fn call_it(i: i32, f: Arc<dyn Fn(i32) -> i32 + Send + Sync>) -> i32 {
    3i32.wrapping_mul(f(i))
}

pub(crate) fn enclose(i: i32) {
    let result = call_it(2, Arc::new(move |j: i32| i.wrapping_add(j)));
    println!("{result}");
}
