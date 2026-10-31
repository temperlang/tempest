use std::sync::{Arc, OnceLock, RwLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            GREETING
                .set(Arc::new("hi".to_string()))
                .unwrap_or_else(|_| panic!());
            *COUNT.write().unwrap() = Some(0);
            LIMIT.set(3).unwrap_or_else(|_| panic!());
            *TOTAL.write().unwrap() = Some(10);
            *TICKS.write().unwrap() = Some(0);
            let mut local: i32 = 1;
            local = local.wrapping_add(limit());
            println!("{}", report());
            println!("{}", report());
            println!("local {local}");
            Ok(())
        })
        .clone()
}

static GREETING: OnceLock<Arc<String>> = OnceLock::new();

pub fn greeting() -> Arc<String> {
    crate::support::initialized(|| GREETING.get()).clone()
}

static COUNT: RwLock<Option<i32>> = RwLock::new(None);

pub(crate) fn count() -> i32 {
    crate::support::initialized(|| *COUNT.read().unwrap())
}

static LIMIT: OnceLock<i32> = OnceLock::new();

pub(crate) fn limit() -> i32 {
    *crate::support::initialized(|| LIMIT.get())
}

static TOTAL: RwLock<Option<i32>> = RwLock::new(None);

pub fn total() -> i32 {
    crate::support::initialized(|| *TOTAL.read().unwrap())
}

static TICKS: RwLock<Option<i32>> = RwLock::new(None);

pub fn ticks() -> i32 {
    crate::support::initialized(|| *TICKS.read().unwrap())
}

pub(crate) fn bump() -> i32 {
    crate::support::initialized(|| {
        let mut count = COUNT.write().unwrap();
        *count = Some((*count)?.wrapping_add(1));
        Some(())
    });
    *TOTAL.write().unwrap() = Some(total().wrapping_add(count()));
    count()
}

pub fn tick() {
    crate::support::initialized(|| {
        let mut ticks = TICKS.write().unwrap();
        *ticks = Some((*ticks)?.wrapping_add(1));
        Some(())
    });
}

pub fn report() -> Arc<String> {
    Arc::new(format!(
        "{} {} of {} {}",
        greeting(),
        bump(),
        limit(),
        total()
    ))
}
