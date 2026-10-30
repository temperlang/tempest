use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let c = C::new();
            println!("method1(10)={}", c.method1(10));
            println!("method2(10)={}", c.method2(10));
            Ok(())
        })
        .clone()
}

struct CStruct {}

#[derive(Clone)]
pub(crate) struct C(Arc<CStruct>);

impl C {
    pub fn new() -> C {
        C(Arc::new(CStruct {}))
    }

    fn helper(&self, x: i32) -> i32 {
        x.wrapping_add(1)
    }

    pub fn method1(&self, x: i32) -> i32 {
        self.helper(x.wrapping_mul(2))
    }

    pub fn method2(&self, x: i32) -> i32 {
        self.helper(x.wrapping_mul(3))
    }
}

temper_core::impl_any_value_trait!(C, []);
