use std::sync::{Arc, OnceLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            hello("directly");
            call_it(Arc::new(|a0: Arc<String>| hello(a0)));
            DelayedFunction::new(Arc::new(|a0: Arc<String>| hello(a0))).call_it();
            println!("Is 5 even? {}", is_even(5));
            println!("Is 6 even? {}", is_even(6));
            Ok(())
        })
        .clone()
}

pub(crate) fn hello(how: impl ToArcString) {
    let how = how.to_arc_string();
    println!("Called {how}!");
}

pub(crate) fn call_it(f: Arc<dyn Fn(Arc<String>) + Send + Sync>) {
    f(Arc::new("by a function".to_string()));
}

struct DelayedFunctionStruct {
    f: Arc<dyn Fn(Arc<String>) + Send + Sync>,
}

#[derive(Clone)]
pub(crate) struct DelayedFunction(Arc<DelayedFunctionStruct>);

impl DelayedFunction {
    pub fn new(f: Arc<dyn Fn(Arc<String>) + Send + Sync>) -> DelayedFunction {
        DelayedFunction(Arc::new(DelayedFunctionStruct { f }))
    }

    pub fn call_it(&self) {
        (self.0.f)(Arc::new("from a method".to_string()));
    }
}

temper_core::impl_any_value_trait!(DelayedFunction, []);

pub(crate) fn is_odd(i: i32) -> bool {
    if i == 0 {
        false
    } else {
        is_even(i.wrapping_sub(1))
    }
}

pub(crate) fn is_even(i: i32) -> bool {
    if i == 0 {
        true
    } else {
        is_odd(i.wrapping_sub(1))
    }
}
