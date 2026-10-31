use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            f(Arc::new(vec![2, 3]));
            f(Arc::new(vec![]));
            Ok(())
        })
        .clone()
}

pub(crate) fn f(b: impl temper_core::ToList<i32>) {
    let b = b.to_list();
    if temper_core::ListedTrait::is_empty(&b) {
        println!("empty");
    } else {
        println!("not empty");
    }
}
