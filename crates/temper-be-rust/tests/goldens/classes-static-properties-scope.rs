use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            SIMPLE__VALUE
                .set(Arc::new("foo".to_string()))
                .unwrap_or_else(|_| panic!());
            SIMPLE__OTHER
                .set(Arc::new("bar".to_string()))
                .unwrap_or_else(|_| panic!());
            println!("Simple.value is {}", read_simple_value());
            println!("Simple.other is {}", read_simple_other());
            Ok(())
        })
        .clone()
}

struct SimpleStruct {}

#[derive(Clone)]
pub(crate) struct Simple(Arc<SimpleStruct>);

static SIMPLE__VALUE: OnceLock<Arc<String>> = OnceLock::new();

static SIMPLE__OTHER: OnceLock<Arc<String>> = OnceLock::new();

impl Simple {
    #[allow(dead_code)]
    pub fn new() -> Simple {
        Simple(Arc::new(SimpleStruct {}))
    }

    pub fn value() -> Arc<String> {
        SIMPLE__VALUE.get().unwrap().clone()
    }

    pub fn other() -> Arc<String> {
        SIMPLE__OTHER.get().unwrap().clone()
    }
}

temper_core::impl_any_value_trait!(Simple, []);

pub(crate) fn read_simple_value() -> Arc<String> {
    Simple::value()
}

pub(crate) fn read_simple_other() -> Arc<String> {
    Simple::other()
}
