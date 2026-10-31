use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let empty_foo = Foo::new(None);
            let empty_bar = Bar::new(None);
            let non_empty_foo = Foo::new(Some(empty_bar.clone()));
            let non_empty_bar = Bar::new(Some(empty_foo.clone()));
            let stuffed_bar = Bar::new(Some(non_empty_foo.clone()));
            println!("emptyFoo.x has Foo -> {}", empty_foo.0.x.is_some());
            println!("nonEmptyFoo.x has Foo -> {}", non_empty_foo.0.x.is_some());
            Ok(())
        })
        .clone()
}

struct BarStruct {
    x: Option<Foo>,
}

#[derive(Clone)]
pub(crate) struct Bar(Arc<BarStruct>);

impl Bar {
    pub fn new(x: Option<Foo>) -> Bar {
        Bar(Arc::new(BarStruct { x }))
    }
}

temper_core::impl_any_value_trait!(Bar, []);

struct FooStruct {
    x: Option<Bar>,
}

#[derive(Clone)]
pub(crate) struct Foo(Arc<FooStruct>);

impl Foo {
    pub fn new(x: Option<Bar>) -> Foo {
        Foo(Arc::new(FooStruct { x }))
    }
}

temper_core::impl_any_value_trait!(Foo, []);
