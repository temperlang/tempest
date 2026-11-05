use std::sync::{Arc, OnceLock, RwLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let t: temper_core::SafeGenerator<()> = ticks(
                2,
                Arc::new(move |i: i32| {
                    println!("tick {i}");
                }),
            );
            t.next_safe();
            t.next_safe();
            t.next_safe();
            C.set(Counter::new()).unwrap_or_else(|_| panic!());
            GATE_A
                .set(temper_core::PromiseBuilder::new())
                .unwrap_or_else(|_| panic!());
            GATE_B
                .set(temper_core::PromiseBuilder::new())
                .unwrap_or_else(|_| panic!());
            crate::support::run_async(Arc::new(move || {
                let case_index: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
                let k: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
                let awaited: Arc<RwLock<Option<temper_core::Promise<i32>>>> =
                    Arc::new(RwLock::new(None));
                temper_core::SafeGenerator::from_fn({
                    let k = k.clone();
                    let case_index = case_index.clone();
                    let awaited = awaited.clone();
                    Arc::new(move |generator: temper_core::SafeGenerator<()>| loop {
                        let case_index_local = temper_core::read_locked(&case_index);
                        *case_index.write().unwrap() = -1;
                        if case_index_local == 0 {
                            c().bump("A", 10000);
                            *case_index.write().unwrap() = 3;
                        } else if case_index_local == 1 {
                            *k.write().unwrap() = -1;
                            *case_index.write().unwrap() = 2;
                        } else if case_index_local == 2 {
                            c().bump(
                                Arc::new(format!("A{}", temper_core::read_locked(&k))),
                                10000,
                            );
                            return None;
                        } else if case_index_local == 3 {
                            *awaited.write().unwrap() = Some(gate_a().promise());
                            *case_index.write().unwrap() = 4;
                            crate::support::awake_upon(
                                temper_core::read_locked(&awaited).unwrap(),
                                generator.clone(),
                            );
                            return Some(());
                        } else if case_index_local == 4 {
                            if (|| {
                                *k.write().unwrap() =
                                    temper_core::read_locked(&awaited).as_ref().unwrap().get()?;
                                *case_index.write().unwrap() = 2;
                                Ok::<_, temper_core::Error>(())
                            })()
                            .is_err()
                            {
                                {
                                    *case_index.write().unwrap() = 1;
                                };
                            }
                        } else {
                            return None;
                        }
                    })
                })
            }));
            crate::support::run_async(Arc::new(move || {
                let case_index: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
                let k: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
                let awaited: Arc<RwLock<Option<temper_core::Promise<i32>>>> =
                    Arc::new(RwLock::new(None));
                temper_core::SafeGenerator::from_fn({
                    let k = k.clone();
                    let case_index = case_index.clone();
                    let awaited = awaited.clone();
                    Arc::new(move |generator: temper_core::SafeGenerator<()>| loop {
                        let case_index_local = temper_core::read_locked(&case_index);
                        *case_index.write().unwrap() = -1;
                        if case_index_local == 0 {
                            c().bump("B", 10000);
                            *case_index.write().unwrap() = 3;
                        } else if case_index_local == 1 {
                            *k.write().unwrap() = -1;
                            *case_index.write().unwrap() = 2;
                        } else if case_index_local == 2 {
                            c().bump(
                                Arc::new(format!("B{}", temper_core::read_locked(&k))),
                                10000,
                            );
                            return None;
                        } else if case_index_local == 3 {
                            *awaited.write().unwrap() = Some(gate_b().promise());
                            *case_index.write().unwrap() = 4;
                            crate::support::awake_upon(
                                temper_core::read_locked(&awaited).unwrap(),
                                generator.clone(),
                            );
                            return Some(());
                        } else if case_index_local == 4 {
                            if (|| {
                                *k.write().unwrap() =
                                    temper_core::read_locked(&awaited).as_ref().unwrap().get()?;
                                *case_index.write().unwrap() = 2;
                                Ok::<_, temper_core::Error>(())
                            })()
                            .is_err()
                            {
                                {
                                    *case_index.write().unwrap() = 1;
                                };
                            }
                        } else {
                            return None;
                        }
                    })
                })
            }));
            crate::support::run_async(Arc::new(move || {
                let case_index: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
                temper_core::SafeGenerator::from_fn({
                    let case_index = case_index.clone();
                    Arc::new(move |_generator: temper_core::SafeGenerator<()>| {
                        let case_index_local = temper_core::read_locked(&case_index);
                        *case_index.write().unwrap() = -1;
                        if case_index_local == 0 {
                            println!("opener: calling complete");
                            gate_a().complete(1);
                            gate_b().complete(2);
                            println!("opener: complete returned");
                            None
                        } else {
                            None
                        }
                    })
                })
            }));
            Ok(())
        })
        .clone()
}

pub fn later(n: i32) -> temper_core::Promise<i32> {
    let pb: temper_core::PromiseBuilder<i32> = temper_core::PromiseBuilder::new();
    crate::support::run_async({
        let pb = pb.clone();
        Arc::new(move || {
            let case_index: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
            temper_core::SafeGenerator::from_fn({
                let pb = pb.clone();
                let case_index = case_index.clone();
                Arc::new(move |_generator: temper_core::SafeGenerator<()>| {
                    let case_index_local = temper_core::read_locked(&case_index);
                    *case_index.write().unwrap() = -1;
                    if case_index_local == 0 {
                        pb.complete(n.wrapping_mul(2));
                        None
                    } else {
                        None
                    }
                })
            })
        })
    });
    pb.promise()
}

pub(crate) fn generated(
    body: Arc<dyn Fn() -> temper_core::SafeGenerator<()> + Send + Sync>,
) -> temper_core::SafeGenerator<()> {
    body()
}

pub fn ticks(n: i32, each: Arc<dyn Fn(i32) + Send + Sync>) -> temper_core::SafeGenerator<()> {
    generated({
        let each = each.clone();
        Arc::new(move || {
            let case_index: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
            let i: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
            temper_core::SafeGenerator::from_fn({
                let each = each.clone();
                let i = i.clone();
                let case_index = case_index.clone();
                Arc::new(move |_generator: temper_core::SafeGenerator<()>| loop {
                    let case_index_local = temper_core::read_locked(&case_index);
                    *case_index.write().unwrap() = -1;
                    if case_index_local == 0 {
                        *i.write().unwrap() = 0;
                        *case_index.write().unwrap() = 1;
                    } else if case_index_local == 1 {
                        if temper_core::read_locked(&i) < n {
                            *case_index.write().unwrap() = 2;
                        } else {
                            *case_index.write().unwrap() = 3;
                        }
                    } else if case_index_local == 2 {
                        each(temper_core::read_locked(&i));
                        *case_index.write().unwrap() = 4;
                        return Some(());
                    } else if case_index_local == 4 {
                        *i.write().unwrap() = temper_core::read_locked(&i).wrapping_add(1);
                        *case_index.write().unwrap() = 1;
                    } else {
                        return None;
                    }
                })
            })
        })
    })
}

struct CounterStruct {
    n: RwLock<i32>,
}

#[derive(Clone)]
pub(crate) struct Counter(Arc<CounterStruct>);

impl Counter {
    pub fn new() -> Counter {
        Counter(Arc::new(CounterStruct { n: RwLock::new(0) }))
    }

    pub fn bump(&self, tag: impl ToArcString, times: i32) {
        let tag = tag.to_arc_string();
        println!("{tag} start");
        {
            let mut i: i32 = 0;
            while i < times {
                let before = self.n();
                *self.0.n.write().unwrap() = before.wrapping_add(1);
                i = i.wrapping_add(1);
            }
        }
        println!("{tag} done n={}", self.n());
    }

    pub fn n(&self) -> i32 {
        *self.0.n.read().unwrap()
    }
}

temper_core::impl_any_value_trait!(Counter, []);

static C: OnceLock<Counter> = OnceLock::new();

pub(crate) fn c() -> Counter {
    crate::support::initialized(|| C.get()).clone()
}

static GATE_A: OnceLock<temper_core::PromiseBuilder<i32>> = OnceLock::new();

pub(crate) fn gate_a() -> temper_core::PromiseBuilder<i32> {
    crate::support::initialized(|| GATE_A.get()).clone()
}

static GATE_B: OnceLock<temper_core::PromiseBuilder<i32>> = OnceLock::new();

pub(crate) fn gate_b() -> temper_core::PromiseBuilder<i32> {
    crate::support::initialized(|| GATE_B.get()).clone()
}
