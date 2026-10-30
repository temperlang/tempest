pub mod deep;

use std::sync::{OnceLock, RwLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            BASE.set(10).unwrap_or_else(|_| panic!());
            *CALLS.write().unwrap() = Some(0);
            println!("{}", crate::helpers::deep::greet("helpers"));
            Ok(())
        })
        .clone()
}

pub fn twice(x: i32) -> i32 {
    x.wrapping_mul(2)
}

static BASE: OnceLock<i32> = OnceLock::new();

pub fn base() -> i32 {
    *BASE.get().unwrap()
}

static CALLS: RwLock<Option<i32>> = RwLock::new(None);

pub fn calls() -> i32 {
    CALLS.read().unwrap().unwrap()
}

pub fn bump() -> i32 {
    {
        let mut calls = CALLS.write().unwrap();
        *calls = Some(calls.unwrap().wrapping_add(1));
    }
    calls()
}
