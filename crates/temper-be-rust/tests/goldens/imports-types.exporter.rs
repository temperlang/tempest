use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE.get_or_init(|| Ok(())).clone()
}

struct ExportedClassStruct {
    foo: i32,
}

#[derive(Clone)]
pub struct ExportedClass(Arc<ExportedClassStruct>);

impl ExportedClass {
    pub fn new(foo: i32) -> ExportedClass {
        ExportedClass(Arc::new(ExportedClassStruct { foo }))
    }

    pub fn foo(&self) -> i32 {
        self.0.foo
    }
}

temper_core::impl_any_value_trait!(ExportedClass, []);
