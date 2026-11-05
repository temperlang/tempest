use std::sync::{Arc, OnceLock, RwLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            f(Arc::new(move || {
                let case_index: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
                let i: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
                temper_core::SafeGenerator::from_fn({
                    let i = i.clone();
                    let case_index = case_index.clone();
                    Arc::new(move |_generator: temper_core::SafeGenerator<()>| loop {
                        let case_index_local = temper_core::read_locked(&case_index);
                        *case_index.write().unwrap() = -1;
                        if case_index_local == 0 {
                            *i.write().unwrap() = 0;
                            *case_index.write().unwrap() = 1;
                        } else if case_index_local == 1 {
                            if true {
                                *case_index.write().unwrap() = 2;
                            } else {
                                *case_index.write().unwrap() = 3;
                            }
                        } else if case_index_local == 2 {
                            println!("generated {}", temper_core::read_locked(&i));
                            *i.write().unwrap() = temper_core::read_locked(&i).wrapping_add(1);
                            *case_index.write().unwrap() = 4;
                            return Some(());
                        } else if case_index_local == 4 {
                            *case_index.write().unwrap() = 1;
                        } else {
                            return None;
                        }
                    })
                })
            }));
            Ok(())
        })
        .clone()
}

pub(crate) fn f(body: Arc<dyn Fn() -> temper_core::SafeGenerator<()> + Send + Sync>) {
    let generator: temper_core::SafeGenerator<()> = body();
    println!("f0");
    generator.next_safe();
    println!("f1");
    generator.next_safe();
    println!("f2");
    generator.next_safe();
    println!("f3");
}
