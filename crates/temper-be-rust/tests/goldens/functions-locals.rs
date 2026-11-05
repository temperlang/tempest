use std::sync::{Arc, OnceLock, RwLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            constant_capture(Arc::new(vec![1, 2, 3]), 1);
            constant_capture_nested(Arc::new(vec![1, 2, 3]), 1);
            mutable_capture(Arc::new(vec![1, 2, 3]));
            mutable_capture_nested(Arc::new(vec![1, 2, 3]));
            thresh(Arc::new(vec![4, 5, 6]), 4);
            thresh2(Arc::new(vec![4, 5, 6]), 4);
            self_recursion(Arc::new(vec![1, 2, 3]));
            self_recursion_nested(Arc::new(vec![1, 2, 3]));
            mutual_recursion(Arc::new(vec![1, 2, 3]));
            let hub: Hub<i32> = Hub::new();
            let _items: temper_core::ListBuilder<i32> = gather_handlers(hub.clone());
            hub.action_performed(1);
            Ok(())
        })
        .clone()
}

pub(crate) fn show(x: impl temper_core::ToListed<i32>) -> Arc<String> {
    let x = x.to_listed();
    let s: temper_core::ListBuilder<Arc<String>> = temper_core::listed::new_builder();
    temper_core::listed::add(&s, Arc::new("[".to_string()), None);
    let mut c = Arc::new(String::new());
    {
        let mut i: i32 = 0;
        while i < temper_core::ListedTrait::len(&*x) {
            temper_core::listed::add(&s, c.clone(), None);
            temper_core::listed::add(
                &s,
                Arc::new(temper_core::ListedTrait::get(&*x, i).to_string()),
                None,
            );
            c = Arc::new(", ".to_string());
            i = i.wrapping_add(1);
        }
    }
    temper_core::listed::add(&s, Arc::new("]".to_string()), None);
    temper_core::listed::join(&s, Arc::new(String::new()), &(|v: Arc<String>| v.clone()))
}

pub(crate) fn nest_block<T: Clone + Send + Sync + 'static>(
    b: Arc<dyn Fn() -> T + Send + Sync>,
) -> T {
    b()
}

pub(crate) fn nest_block_void(b: Arc<dyn Fn() + Send + Sync>) {
    b();
}

pub(crate) fn constant_capture(x: impl temper_core::ToListed<i32>, a: i32) {
    let x = x.to_listed();
    let b = a.wrapping_mul(1);
    let out: temper_core::List<i32> = temper_core::listed::map(&*x, &(|n: i32| n.wrapping_add(b)));
    println!("constantCapture = {}", show(out.clone()));
}

pub(crate) fn constant_capture_nested(x: impl temper_core::ToListed<i32>, a: i32) {
    let x = x.to_listed();
    let b = a.wrapping_mul(1);
    let out: temper_core::List<i32> = nest_block({
        let x = x.clone();
        Arc::new(move || temper_core::listed::map(&*x, &(|n: i32| n.wrapping_add(b))))
    });
    println!("constantCaptureNested = {}", show(out.clone()));
}

pub(crate) fn mutable_capture(x: impl temper_core::ToListed<i32>) {
    let x = x.to_listed();
    let sum: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
    let out: temper_core::List<i32> = temper_core::listed::map(
        &*x,
        &(|n: i32| {
            *sum.write().unwrap() = temper_core::read_locked(&sum).wrapping_add(n);
            temper_core::read_locked(&sum)
        }),
    );
    println!(
        "mutableCapture = {}; sum={}",
        show(out.clone()),
        temper_core::read_locked(&sum)
    );
}

pub(crate) fn mutable_capture_nested(x: impl temper_core::ToListed<i32>) {
    let x = x.to_listed();
    let sum: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
    let out: temper_core::List<i32> = nest_block({
        let x = x.clone();
        let sum = sum.clone();
        Arc::new(move || {
            temper_core::listed::map(
                &*x,
                &(|n: i32| {
                    *sum.write().unwrap() = temper_core::read_locked(&sum).wrapping_add(n);
                    temper_core::read_locked(&sum)
                }),
            )
        })
    });
    println!(
        "mutableCaptureNested = {}; sum={}",
        show(out.clone()),
        temper_core::read_locked(&sum)
    );
}

pub fn thresh(values: impl temper_core::ToListed<i32>, value: i32) {
    let values = values.to_listed();
    let value: Arc<RwLock<i32>> = Arc::new(RwLock::new(value));
    *value.write().unwrap() = temper_core::read_locked(&value).wrapping_add(1);
    let result: temper_core::List<i32> =
        temper_core::listed::filter(&*values, &(|it: i32| it > temper_core::read_locked(&value)));
    println!("thresh length = {}", temper_core::ListedTrait::len(&result));
}

pub fn thresh2(values: impl temper_core::ToListed<i32>, value: i32) {
    let values = values.to_listed();
    let value: Arc<RwLock<i32>> = Arc::new(RwLock::new(value));
    *value.write().unwrap() = temper_core::read_locked(&value).wrapping_add(1);
    let result: temper_core::List<i32> =
        temper_core::listed::filter(&*values, &(|it: i32| it > temper_core::read_locked(&value)));
    println!(
        "thresh2 length = {}",
        temper_core::ListedTrait::len(&result)
    );
}

pub(crate) fn self_recursion(x: impl temper_core::ToList<i32>) {
    let x = x.to_list();
    #[derive(Clone)]
    struct MapperGroup {
        x: temper_core::List<i32>,
    }
    impl MapperGroup {
        fn mapper(&self, i: i32, out: temper_core::ListBuilder<i32>) {
            let x = self.x.clone();
            if i < temper_core::ListedTrait::len(&x) {
                temper_core::listed::add(
                    &out,
                    temper_core::ListedTrait::get(&x, i).wrapping_mul(2),
                    None,
                );
                self.mapper(i.wrapping_add(1), out.clone());
            }
        }
    }
    let mapper_group = MapperGroup { x: x.clone() };
    let mapper =
        Arc::new(move |a0: i32, a1: temper_core::ListBuilder<i32>| mapper_group.mapper(a0, a1));
    let out: temper_core::ListBuilder<i32> = temper_core::listed::new_builder();
    mapper(0, out.clone());
    println!("selfRecursion = {}", show(out.clone()));
}

pub(crate) fn self_recursion_nested(x: impl temper_core::ToList<i32>) {
    let x = x.to_list();
    #[derive(Clone)]
    struct MapperGroup {
        x: temper_core::List<i32>,
    }
    impl MapperGroup {
        fn mapper(&self, i: i32, out: temper_core::ListBuilder<i32>) {
            let x = self.x.clone();
            nest_block_void({
                let x = x.clone();
                let mapper = {
                    let group = self.clone();
                    Arc::new(move |a0: i32, a1: temper_core::ListBuilder<i32>| group.mapper(a0, a1))
                };
                let out = out.clone();
                Arc::new(move || {
                    if i < temper_core::ListedTrait::len(&x) {
                        temper_core::listed::add(
                            &out,
                            temper_core::ListedTrait::get(&x, i).wrapping_mul(2),
                            None,
                        );
                        mapper(i.wrapping_add(1), out.clone());
                    }
                })
            });
        }
    }
    let mapper_group = MapperGroup { x: x.clone() };
    let mapper =
        Arc::new(move |a0: i32, a1: temper_core::ListBuilder<i32>| mapper_group.mapper(a0, a1));
    let out2: temper_core::ListBuilder<i32> = temper_core::listed::new_builder();
    mapper(0, out2.clone());
    println!("selfRecursionNested = {}", show(out2.clone()));
}

pub(crate) fn mutual_recursion(x: impl temper_core::ToList<i32>) {
    let x = x.to_list();
    #[derive(Clone)]
    struct WalkerGroup {
        x: temper_core::List<i32>,
    }
    impl WalkerGroup {
        fn walker(&self, i: i32, out: temper_core::ListBuilder<i32>) {
            let x = self.x.clone();
            if i < temper_core::ListedTrait::len(&x) {
                self.putter(i, out.clone());
            }
        }

        fn putter(&self, i: i32, out: temper_core::ListBuilder<i32>) {
            let x = self.x.clone();
            temper_core::listed::add(
                &out,
                temper_core::ListedTrait::get(&x, i).wrapping_mul(2),
                None,
            );
            self.walker(i.wrapping_add(1), out.clone());
        }
    }
    let walker_group = WalkerGroup { x: x.clone() };
    let walker =
        Arc::new(move |a0: i32, a1: temper_core::ListBuilder<i32>| walker_group.walker(a0, a1));
    let out: temper_core::ListBuilder<i32> = temper_core::listed::new_builder();
    walker(0, out.clone());
    println!("mutualRecursion = {}", show(out.clone()));
}

struct HubStruct<T: Clone + Send + Sync + 'static> {
    handlers: temper_core::ListBuilder<Arc<dyn Fn(T) + Send + Sync>>,
}

#[derive(Clone)]
pub(crate) struct Hub<T: Clone + Send + Sync + 'static>(Arc<HubStruct<T>>);

impl<T: Clone + Send + Sync + 'static> Hub<T> {
    pub fn new() -> Hub<T> {
        Hub(Arc::new(HubStruct {
            handlers: temper_core::listed::new_builder(),
        }))
    }

    pub fn on_action(&self, handler: Arc<dyn Fn(T) + Send + Sync>) {
        temper_core::listed::add(&self.0.handlers, handler.clone(), None);
    }

    pub fn action_performed(&self, item: T) {
        let mut i: i32 = 0;
        while i < temper_core::ListedTrait::len(&self.0.handlers) {
            temper_core::ListedTrait::get(&self.0.handlers, i)(item.clone());
            i = i.wrapping_add(1);
        }
    }
}

temper_core::impl_any_value_trait!(Hub<T>, []);

pub(crate) fn gather_handlers(hub: Hub<i32>) -> temper_core::ListBuilder<i32> {
    let items: temper_core::ListBuilder<i32> = temper_core::listed::new_builder();
    {
        let mut i: i32 = 0;
        while i < 3 {
            let i2 = i;
            hub.on_action({
                let items = items.clone();
                Arc::new(move |item: i32| {
                    temper_core::listed::add(&items, item.wrapping_add(i2), None);
                })
            });
            i = i.wrapping_add(1);
        }
    }
    items.clone()
}
