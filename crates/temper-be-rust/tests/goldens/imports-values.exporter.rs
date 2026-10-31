use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let mut a = temper_core::ListedTrait::get(&foo(), 0);
            a = a;
            MAGIC_NUMBER.set(a).unwrap_or_else(|_| panic!());
            Ok(())
        })
        .clone()
}

pub(crate) fn foo() -> temper_core::List<i32> {
    Arc::new(vec![21i32.wrapping_add(21), 42])
}

static MAGIC_NUMBER: OnceLock<i32> = OnceLock::new();

pub fn magic_number() -> i32 {
    *MAGIC_NUMBER.get().unwrap()
}
