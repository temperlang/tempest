use std::sync::{Arc, OnceLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            SIMPLE__SIMON
                .set(Simple::new("foo"))
                .unwrap_or_else(|_| panic!());
            SIMPLE__VALUE
                .set(Simple::simon().0.name.clone())
                .unwrap_or_else(|_| panic!());
            SIMPLE__OTHER.set(22).unwrap_or_else(|_| panic!());
            println!("Simple.simon.name is {}", Simple::simon().0.name);
            println!("Simple.value is {}", Simple::value());
            println!("Simple.other is {}", Simple::other());
            C::say_hi();
            I::say_bye();
            Ok(())
        })
        .clone()
}

struct SimpleStruct {
    name: Arc<String>,
}

#[derive(Clone)]
pub(crate) struct Simple(Arc<SimpleStruct>);

static SIMPLE__SIMON: OnceLock<Simple> = OnceLock::new();

static SIMPLE__VALUE: OnceLock<Arc<String>> = OnceLock::new();

static SIMPLE__OTHER: OnceLock<i32> = OnceLock::new();

impl Simple {
    pub fn new(name: impl ToArcString) -> Simple {
        let name = name.to_arc_string();
        Simple(Arc::new(SimpleStruct { name }))
    }

    pub fn simon() -> Simple {
        SIMPLE__SIMON.get().unwrap().clone()
    }

    pub fn value() -> Arc<String> {
        SIMPLE__VALUE.get().unwrap().clone()
    }

    pub fn other() -> i32 {
        *SIMPLE__OTHER.get().unwrap()
    }
}

temper_core::impl_any_value_trait!(Simple, []);

struct CStruct {}

#[derive(Clone)]
pub(crate) struct C(Arc<CStruct>);

impl C {
    #[allow(dead_code)]
    pub fn new() -> C {
        C(Arc::new(CStruct {}))
    }

    pub fn say_hi() {
        println!("{}", C::compose("C", "'hi.'"));
    }

    fn compose(subject: impl ToArcString, message: impl ToArcString) -> Arc<String> {
        let subject = subject.to_arc_string();
        let message = message.to_arc_string();
        Arc::new(format!("{subject} says {message}"))
    }
}

temper_core::impl_any_value_trait!(C, []);

pub(crate) trait ITrait:
    temper_core::AsAnyValue + temper_core::AnyValueTrait + Send + Sync
{
    #[allow(dead_code)]
    fn clone_boxed(&self) -> I;
}

#[derive(Clone)]
pub(crate) struct I(Arc<dyn ITrait>);

impl I {
    #[allow(dead_code)]
    pub fn new(selfish: impl ITrait + 'static) -> I {
        I(Arc::new(selfish))
    }

    pub fn say_bye() {
        println!("I says 'bye.'");
    }
}

impl ITrait for I {
    fn clone_boxed(&self) -> I {
        ITrait::clone_boxed(&*self.0)
    }
}

temper_core::impl_any_value_trait_for_interface!(I);

impl std::ops::Deref for I {
    type Target = dyn ITrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
