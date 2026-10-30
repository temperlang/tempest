use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let c = C::new();
            println!("Hello, {}!", c.place());
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

    pub fn place(&self) -> Arc<String> {
        Arc::new("Hilo, HI".to_string())
    }
}

temper_core::impl_any_value_trait!(C, []);
