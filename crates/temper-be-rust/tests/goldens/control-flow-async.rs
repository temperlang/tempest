use std::sync::{Arc, OnceLock, RwLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            crate::support::run_async(Arc::new(move || {
                let case_index4: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
                let b: Arc<RwLock<Option<temper_core::PromiseBuilder<Arc<String>>>>> =
                    Arc::new(RwLock::new(None));
                let b2: Arc<RwLock<Option<temper_core::PromiseBuilder<Arc<String>>>>> =
                    Arc::new(RwLock::new(None));
                let b3: Arc<RwLock<Option<temper_core::PromiseBuilder<Arc<String>>>>> =
                    Arc::new(RwLock::new(None));
                let b4: Arc<RwLock<Option<temper_core::PromiseBuilder<Arc<String>>>>> =
                    Arc::new(RwLock::new(None));
                let a: Arc<RwLock<Option<temper_core::PromiseBuilder<()>>>> =
                    Arc::new(RwLock::new(None));
                let b5: Arc<RwLock<Option<temper_core::PromiseBuilder<()>>>> =
                    Arc::new(RwLock::new(None));
                let b6: Arc<RwLock<Option<temper_core::PromiseBuilder<Arc<String>>>>> =
                    Arc::new(RwLock::new(None));
                let t: Arc<RwLock<Arc<String>>> = Arc::new(RwLock::new(Arc::new(String::new())));
                let awaited2: Arc<RwLock<Option<temper_core::Promise<Arc<String>>>>> =
                    Arc::new(RwLock::new(None));
                let t2: Arc<RwLock<Arc<String>>> = Arc::new(RwLock::new(Arc::new(String::new())));
                let awaited3: Arc<RwLock<Option<temper_core::Promise<Arc<String>>>>> =
                    Arc::new(RwLock::new(None));
                let t3: Arc<RwLock<Arc<String>>> = Arc::new(RwLock::new(Arc::new(String::new())));
                let awaited4: Arc<RwLock<Option<temper_core::Promise<Arc<String>>>>> =
                    Arc::new(RwLock::new(None));
                let t4: Arc<RwLock<Arc<String>>> = Arc::new(RwLock::new(Arc::new(String::new())));
                let awaited5: Arc<RwLock<Option<temper_core::Promise<Arc<String>>>>> =
                    Arc::new(RwLock::new(None));
                let t5: Arc<RwLock<Arc<String>>> = Arc::new(RwLock::new(Arc::new(String::new())));
                let awaited6: Arc<RwLock<Option<temper_core::Promise<Arc<String>>>>> =
                    Arc::new(RwLock::new(None));
                let awaited7: Arc<RwLock<Option<temper_core::Promise<()>>>> =
                    Arc::new(RwLock::new(None));
                let t6: Arc<RwLock<Arc<String>>> = Arc::new(RwLock::new(Arc::new(String::new())));
                let awaited8: Arc<RwLock<Option<temper_core::Promise<Arc<String>>>>> =
                    Arc::new(RwLock::new(None));
                temper_core::SafeGenerator::from_fn({
                    let b = b.clone();
                    let b2 = b2.clone();
                    let b3 = b3.clone();
                    let b4 = b4.clone();
                    let a = a.clone();
                    let b5 = b5.clone();
                    let b6 = b6.clone();
                    let case_index4 = case_index4.clone();
                    let t = t.clone();
                    let awaited2 = awaited2.clone();
                    let t2 = t2.clone();
                    let awaited3 = awaited3.clone();
                    let t3 = t3.clone();
                    let awaited4 = awaited4.clone();
                    let t4 = t4.clone();
                    let awaited5 = awaited5.clone();
                    let t5 = t5.clone();
                    let awaited6 = awaited6.clone();
                    let awaited7 = awaited7.clone();
                    let t6 = t6.clone();
                    let awaited8 = awaited8.clone();
                    Arc::new(move |generator2: temper_core::SafeGenerator<()>| loop {
                        let case_index_local2 = temper_core::read_locked(&case_index4);
                        *case_index4.write().unwrap() = -1;
                        if case_index_local2 == 0 {
                            *b.write().unwrap() = Some(temper_core::PromiseBuilder::new());
                            temper_core::read_locked(&b)
                                .as_ref()
                                .unwrap()
                                .complete(Arc::new("Completed synchronously".to_string()));
                            *awaited2.write().unwrap() =
                                Some(temper_core::read_locked(&b).as_ref().unwrap().promise());
                            *case_index4.write().unwrap() = 3;
                            temper_core::read_locked(&awaited2)
                                .as_ref()
                                .unwrap()
                                .on_ready(Arc::new({
                                    let g = generator2.clone();
                                    move || {
                                        g.next_safe();
                                    }
                                }));
                            return Some(());
                        } else if case_index_local2 == 1 {
                            panic!();
                        } else if case_index_local2 == 2 {
                            *b2.write().unwrap() = Some(temper_core::PromiseBuilder::new());
                            temper_core::read_locked(&b2)
                                .as_ref()
                                .unwrap()
                                .break_promise();
                            *case_index4.write().unwrap() = 6;
                        } else if case_index_local2 == 3 {
                            if (|| {
                                *t.write().unwrap() =
                                    temper_core::read_locked(&awaited2).as_ref().unwrap().get()?;
                                println!("{}", temper_core::read_locked(&t));
                                *case_index4.write().unwrap() = 2;
                                Ok::<_, temper_core::Error>(())
                            })()
                            .is_err()
                            {
                                {
                                    *case_index4.write().unwrap() = 1;
                                };
                            }
                        } else if case_index_local2 == 4 {
                            *t2.write().unwrap() = Arc::new("Broken synchronously".to_string());
                            *case_index4.write().unwrap() = 5;
                        } else if case_index_local2 == 5 {
                            println!("{}", temper_core::read_locked(&t2));
                            *case_index4.write().unwrap() = 10;
                        } else if case_index_local2 == 6 {
                            *awaited3.write().unwrap() =
                                Some(temper_core::read_locked(&b2).as_ref().unwrap().promise());
                            *case_index4.write().unwrap() = 7;
                            temper_core::read_locked(&awaited3)
                                .as_ref()
                                .unwrap()
                                .on_ready(Arc::new({
                                    let g = generator2.clone();
                                    move || {
                                        g.next_safe();
                                    }
                                }));
                            return Some(());
                        } else if case_index_local2 == 7 {
                            if (|| {
                                *t2.write().unwrap() =
                                    temper_core::read_locked(&awaited3).as_ref().unwrap().get()?;
                                *case_index4.write().unwrap() = 5;
                                Ok::<_, temper_core::Error>(())
                            })()
                            .is_err()
                            {
                                {
                                    *case_index4.write().unwrap() = 4;
                                };
                            }
                        } else if case_index_local2 == 8 {
                            panic!();
                        } else if case_index_local2 == 9 {
                            *b4.write().unwrap() = Some(temper_core::PromiseBuilder::new());
                            temper_core::read_locked(&b4)
                                .as_ref()
                                .unwrap()
                                .complete(Arc::new("Worth doing twice".to_string()));
                            *awaited5.write().unwrap() =
                                Some(temper_core::read_locked(&b4).as_ref().unwrap().promise());
                            *case_index4.write().unwrap() = 14;
                            temper_core::read_locked(&awaited5)
                                .as_ref()
                                .unwrap()
                                .on_ready(Arc::new({
                                    let g = generator2.clone();
                                    move || {
                                        g.next_safe();
                                    }
                                }));
                            return Some(());
                        } else if case_index_local2 == 10 {
                            *b3.write().unwrap() = Some(temper_core::PromiseBuilder::new());
                            crate::support::run_async({
                                let b3 = b3.clone();
                                Arc::new(move || {
                                    let case_index: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
                                    temper_core::SafeGenerator::from_fn({
                                        let b3 = b3.clone();
                                        let case_index = case_index.clone();
                                        Arc::new(move |_generator: temper_core::SafeGenerator<()>| {
                                            let case_index_local =
                                                temper_core::read_locked(&case_index);
                                            *case_index.write().unwrap() = -1;
                                            if case_index_local == 0 {
                                                temper_core::read_locked(&b3)
                                                    .as_ref()
                                                    .unwrap()
                                                    .complete(Arc::new(
                                                        "Completed asynchronously".to_string(),
                                                    ));
                                                None
                                            } else {
                                                None
                                            }
                                        })
                                    })
                                })
                            });
                            *awaited4.write().unwrap() =
                                Some(temper_core::read_locked(&b3).as_ref().unwrap().promise());
                            *case_index4.write().unwrap() = 11;
                            temper_core::read_locked(&awaited4)
                                .as_ref()
                                .unwrap()
                                .on_ready(Arc::new({
                                    let g = generator2.clone();
                                    move || {
                                        g.next_safe();
                                    }
                                }));
                            return Some(());
                        } else if case_index_local2 == 11 {
                            if (|| {
                                *t3.write().unwrap() =
                                    temper_core::read_locked(&awaited4).as_ref().unwrap().get()?;
                                println!("{}", temper_core::read_locked(&t3));
                                *case_index4.write().unwrap() = 9;
                                Ok::<_, temper_core::Error>(())
                            })()
                            .is_err()
                            {
                                {
                                    *case_index4.write().unwrap() = 8;
                                };
                            }
                        } else if case_index_local2 == 12 {
                            panic!();
                        } else if case_index_local2 == 13 {
                            *a.write().unwrap() = Some(temper_core::PromiseBuilder::new());
                            *b5.write().unwrap() = Some(temper_core::PromiseBuilder::new());
                            crate::support::run_async({
                                let a = a.clone();
                                let b5 = b5.clone();
                                Arc::new(move || {
                                    let case_index2: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
                                    let awaited: Arc<RwLock<Option<temper_core::Promise<()>>>> =
                                        Arc::new(RwLock::new(None));
                                    temper_core::SafeGenerator::from_fn({
                                        let a = a.clone();
                                        let b5 = b5.clone();
                                        let case_index2 = case_index2.clone();
                                        let awaited = awaited.clone();
                                        Arc::new(
                                            move |generator: temper_core::SafeGenerator<()>| loop {
                                                let case_index_local =
                                                    temper_core::read_locked(&case_index2);
                                                *case_index2.write().unwrap() = -1;
                                                if case_index_local == 0 {
                                                    *awaited.write().unwrap() = Some(
                                                        temper_core::read_locked(&b5)
                                                            .as_ref()
                                                            .unwrap()
                                                            .promise(),
                                                    );
                                                    *case_index2.write().unwrap() = 3;
                                                    temper_core::read_locked(&awaited)
                                                        .as_ref()
                                                        .unwrap()
                                                        .on_ready(Arc::new({
                                                            let g = generator.clone();
                                                            move || {
                                                                g.next_safe();
                                                            }
                                                        }));
                                                    return Some(());
                                                } else if case_index_local == 1 {
                                                    panic!();
                                                } else if case_index_local == 2 {
                                                    println!("After");
                                                    temper_core::read_locked(&a)
                                                        .as_ref()
                                                        .unwrap()
                                                        .complete(());
                                                    return None;
                                                } else if case_index_local == 3 {
                                                    if (|| {
                                                        temper_core::read_locked(&awaited)
                                                            .as_ref()
                                                            .unwrap()
                                                            .get()?;
                                                        *case_index2.write().unwrap() = 2;
                                                        Ok::<_, temper_core::Error>(())
                                                    })()
                                                    .is_err()
                                                    {
                                                        {
                                                            *case_index2.write().unwrap() = 1;
                                                        };
                                                    }
                                                } else {
                                                    return None;
                                                }
                                            },
                                        )
                                    })
                                })
                            });
                            crate::support::run_async({
                                let b5 = b5.clone();
                                Arc::new(move || {
                                    let case_index3: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
                                    temper_core::SafeGenerator::from_fn({
                                        let b5 = b5.clone();
                                        let case_index3 = case_index3.clone();
                                        Arc::new(move |_generator: temper_core::SafeGenerator<()>| {
                                            let case_index_local =
                                                temper_core::read_locked(&case_index3);
                                            *case_index3.write().unwrap() = -1;
                                            if case_index_local == 0 {
                                                println!("Before");
                                                temper_core::read_locked(&b5)
                                                    .as_ref()
                                                    .unwrap()
                                                    .complete(());
                                                None
                                            } else {
                                                None
                                            }
                                        })
                                    })
                                })
                            });
                            *awaited7.write().unwrap() =
                                Some(temper_core::read_locked(&a).as_ref().unwrap().promise());
                            *case_index4.write().unwrap() = 18;
                            temper_core::read_locked(&awaited7)
                                .as_ref()
                                .unwrap()
                                .on_ready(Arc::new({
                                    let g = generator2.clone();
                                    move || {
                                        g.next_safe();
                                    }
                                }));
                            return Some(());
                        } else if case_index_local2 == 14 {
                            'orelse: {
                                *t4.write().unwrap() = if let Ok(v) = temper_core::read_locked(&awaited5).as_ref().unwrap().get() {
                                    v
                                } else {
                                    break 'orelse;
                                };
                                println!("{}", temper_core::read_locked(&t4));
                                *awaited6.write().unwrap() =
                                    Some(temper_core::read_locked(&b4).as_ref().unwrap().promise());
                                *case_index4.write().unwrap() = 15;
                                temper_core::read_locked(&awaited6)
                                    .as_ref()
                                    .unwrap()
                                    .on_ready(Arc::new({
                                        let g = generator2.clone();
                                        move || {
                                            g.next_safe();
                                        }
                                    }));
                                return Some(());
                            }
                            {
                                *case_index4.write().unwrap() = 12;
                            };
                        } else if case_index_local2 == 15 {
                            if (|| {
                                *t5.write().unwrap() =
                                    temper_core::read_locked(&awaited6).as_ref().unwrap().get()?;
                                println!("{}", temper_core::read_locked(&t5));
                                *case_index4.write().unwrap() = 13;
                                Ok::<_, temper_core::Error>(())
                            })()
                            .is_err()
                            {
                                {
                                    *case_index4.write().unwrap() = 12;
                                };
                            }
                        } else if case_index_local2 == 16 {
                            panic!();
                        } else if case_index_local2 == 17 {
                            println!("What is your favourite colour?");
                            *b6.write().unwrap() = Some(temper_core::PromiseBuilder::new());
                            temper_core::read_locked(&b6)
                                .as_ref()
                                .unwrap()
                                .complete(Arc::new("Blue".to_string()));
                            temper_core::read_locked(&b6)
                                .as_ref()
                                .unwrap()
                                .complete(Arc::new("No, yell...".to_string()));
                            *awaited8.write().unwrap() =
                                Some(temper_core::read_locked(&b6).as_ref().unwrap().promise());
                            *case_index4.write().unwrap() = 21;
                            temper_core::read_locked(&awaited8)
                                .as_ref()
                                .unwrap()
                                .on_ready(Arc::new({
                                    let g = generator2.clone();
                                    move || {
                                        g.next_safe();
                                    }
                                }));
                            return Some(());
                        } else if case_index_local2 == 18 {
                            if (|| {
                                temper_core::read_locked(&awaited7).as_ref().unwrap().get()?;
                                *case_index4.write().unwrap() = 17;
                                Ok::<_, temper_core::Error>(())
                            })()
                            .is_err()
                            {
                                {
                                    *case_index4.write().unwrap() = 16;
                                };
                            }
                        } else if case_index_local2 == 19 {
                            *case_index4.write().unwrap() = 20;
                        } else if case_index_local2 == 21 {
                            if (|| {
                                *t6.write().unwrap() =
                                    temper_core::read_locked(&awaited8).as_ref().unwrap().get()?;
                                println!("{}", temper_core::read_locked(&t6));
                                *case_index4.write().unwrap() = 20;
                                Ok::<_, temper_core::Error>(())
                            })()
                            .is_err()
                            {
                                {
                                    *case_index4.write().unwrap() = 19;
                                };
                            }
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
