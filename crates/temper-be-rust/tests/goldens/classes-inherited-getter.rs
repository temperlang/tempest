use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let c = C::new();
            println!("Goodbye, gotta run, {}!", c.place());
            Ok(())
        })
        .clone()
}

pub(crate) trait ITrait:
    temper_core::AsAnyValue + temper_core::AnyValueTrait + Send + Sync
{
    #[allow(dead_code)]
    fn clone_boxed(&self) -> I;

    fn place(&self) -> Arc<String> {
        Arc::new("Byron Bay".to_string())
    }
}

#[derive(Clone)]
pub(crate) struct I(Arc<dyn ITrait>);

impl I {
    pub fn new(selfish: impl ITrait + 'static) -> I {
        I(Arc::new(selfish))
    }
}

impl ITrait for I {
    fn clone_boxed(&self) -> I {
        ITrait::clone_boxed(&*self.0)
    }

    fn place(&self) -> Arc<String> {
        ITrait::place(&*self.0)
    }
}

temper_core::impl_any_value_trait_for_interface!(I);

impl std::ops::Deref for I {
    type Target = dyn ITrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

struct CStruct {}

#[derive(Clone)]
pub(crate) struct C(Arc<CStruct>);

impl C {
    pub fn new() -> C {
        C(Arc::new(CStruct {}))
    }
}

impl ITrait for C {
    fn clone_boxed(&self) -> I {
        I::new(self.clone())
    }
}

temper_core::impl_any_value_trait!(C, [I]);
