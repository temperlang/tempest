use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            NATIVE.set(2).unwrap_or_else(|_| panic!());
            Ok(())
        })
        .clone()
}

pub fn lambda(n: i32) -> i32 {
    n.wrapping_add(1)
}

static NATIVE: OnceLock<i32> = OnceLock::new();

pub fn native() -> i32 {
    *crate::support::initialized(|| NATIVE.get())
}

struct CheckedStruct {
    end: i32,
}

#[derive(Clone)]
pub struct Checked(Arc<CheckedStruct>);

impl Checked {
    pub fn new(end: i32) -> Checked {
        Checked(Arc::new(CheckedStruct { end }))
    }

    pub fn delete(&self) -> i32 {
        self.0.end.wrapping_mul(2)
    }

    pub fn local(&self) -> i32 {
        self.0.end.wrapping_add(native())
    }

    pub fn end(&self) -> i32 {
        self.0.end
    }
}

temper_core::impl_any_value_trait!(Checked, []);
