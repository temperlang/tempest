use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE.get_or_init(|| Ok(())).clone()
}

struct HiStruct<T: Clone + Send + Sync + 'static> {
    ha: Arc<dyn Fn(T) -> T + Send + Sync>,
}

#[derive(Clone)]
pub struct Hi<T: Clone + Send + Sync + 'static>(Arc<HiStruct<T>>);

impl<T: Clone + Send + Sync + 'static> Hi<T> {
    pub fn new(ha: Arc<dyn Fn(T) -> T + Send + Sync>) -> Hi<T> {
        Hi(Arc::new(HiStruct { ha }))
    }

    pub fn ha(&self) -> Arc<dyn Fn(T) -> T + Send + Sync> {
        self.0.ha.clone()
    }
}

temper_core::impl_any_value_trait!(Hi<T>, []);

pub fn make_hi<T: Clone + Send + Sync + 'static>(ha: Arc<dyn Fn(T) -> T + Send + Sync>) -> Hi<T> {
    Hi::new(ha.clone())
}

pub fn new_int_hi(j: i32) -> Hi<i32> {
    Hi::new(Arc::new(move |i: i32| i.wrapping_add(j)))
}

pub fn make_int_hi(j: i32) -> Hi<i32> {
    make_hi(Arc::new(move |i: i32| i.wrapping_add(j)))
}
