use std::sync::{Arc, OnceLock, RwLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            println!("{}", Foo::new("initial").bar());
            let foo = Foo::new("initial");
            foo.set_bar("next");
            println!("{}", foo.bar());
            let bar = Bar::new("initial");
            bar.using_set("next");
            println!("{}", bar.bar());
            let what = Whatever::new(ConcreteWhatever::new(4));
            what.set_something(5);
            println!("something set to {}", what.something());
            Ok(())
        })
        .clone()
}

struct FooStruct {
    backing: RwLock<Arc<String>>,
}

#[derive(Clone)]
pub(crate) struct Foo(Arc<FooStruct>);

impl Foo {
    pub fn new(backing: impl ToArcString) -> Foo {
        let backing = backing.to_arc_string();
        Foo(Arc::new(FooStruct {
            backing: RwLock::new(backing),
        }))
    }

    pub fn bar(&self) -> Arc<String> {
        self.backing()
    }

    pub fn set_bar(&self, value: impl ToArcString) {
        let value = value.to_arc_string();
        *self.0.backing.write().unwrap() = value.clone();
    }

    fn backing(&self) -> Arc<String> {
        self.0.backing.read().unwrap().clone()
    }
}

temper_core::impl_any_value_trait!(Foo, []);

struct BarStruct {
    backing: RwLock<Arc<String>>,
}

#[derive(Clone)]
pub(crate) struct Bar(Arc<BarStruct>);

impl Bar {
    pub fn new(backing: impl ToArcString) -> Bar {
        let backing = backing.to_arc_string();
        Bar(Arc::new(BarStruct {
            backing: RwLock::new(backing),
        }))
    }

    pub fn bar(&self) -> Arc<String> {
        self.backing()
    }

    fn set_bar(&self, value: impl ToArcString) {
        let value = value.to_arc_string();
        *self.0.backing.write().unwrap() = value.clone();
    }

    pub fn using_set(&self, value: impl ToArcString) {
        let value = value.to_arc_string();
        self.set_bar(value.clone());
    }

    fn backing(&self) -> Arc<String> {
        self.0.backing.read().unwrap().clone()
    }
}

temper_core::impl_any_value_trait!(Bar, []);

pub trait WhateverTrait:
    temper_core::AsAnyValue + temper_core::AnyValueTrait + Send + Sync
{
    fn clone_boxed(&self) -> Whatever;

    fn something(&self) -> i32;

    fn set_something(&self, blah: i32);
}

#[derive(Clone)]
pub struct Whatever(Arc<dyn WhateverTrait>);

impl Whatever {
    pub fn new(selfish: impl WhateverTrait + 'static) -> Whatever {
        Whatever(Arc::new(selfish))
    }
}

impl WhateverTrait for Whatever {
    fn clone_boxed(&self) -> Whatever {
        WhateverTrait::clone_boxed(&*self.0)
    }

    fn something(&self) -> i32 {
        WhateverTrait::something(&*self.0)
    }

    fn set_something(&self, blah: i32) {
        WhateverTrait::set_something(&*self.0, blah)
    }
}

temper_core::impl_any_value_trait_for_interface!(Whatever);

impl std::ops::Deref for Whatever {
    type Target = dyn WhateverTrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

struct ConcreteWhateverStruct {
    something: RwLock<i32>,
}

#[derive(Clone)]
pub struct ConcreteWhatever(Arc<ConcreteWhateverStruct>);

impl ConcreteWhatever {
    pub fn new(something: i32) -> ConcreteWhatever {
        ConcreteWhatever(Arc::new(ConcreteWhateverStruct {
            something: RwLock::new(something),
        }))
    }

    pub fn something(&self) -> i32 {
        *self.0.something.read().unwrap()
    }

    pub fn set_something(&self, value: i32) {
        *self.0.something.write().unwrap() = value;
    }
}

impl WhateverTrait for ConcreteWhatever {
    fn clone_boxed(&self) -> Whatever {
        Whatever::new(self.clone())
    }

    fn something(&self) -> i32 {
        self.something()
    }

    fn set_something(&self, blah: i32) {
        self.set_something(blah)
    }
}

temper_core::impl_any_value_trait!(ConcreteWhatever, [Whatever]);
