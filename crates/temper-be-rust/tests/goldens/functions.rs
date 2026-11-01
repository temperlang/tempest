use std::sync::{Arc, OnceLock, RwLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let counter = make_counter(10);
            counter();
            println!("count {}", counter());
            let other = make_counter(0);
            println!("other {} count {}", other(), counter());
            let add_two = twice(Arc::new(add_one));
            println!("twice {} {}", add_two(5), apply_to(Arc::new(add_one), 1));
            println!(
                "triple {} {}",
                apply_to(Arc::new(Scaler::triple), 4),
                twice(Arc::new(Scaler::triple))(2)
            );
            let ns: temper_core::List<i32> = Arc::new(vec![5, 2, 8, 1, 4]);
            let doubled: temper_core::List<i32> =
                temper_core::listed::map(&ns, &(|n: i32| n.wrapping_mul(2)));
            let big: temper_core::List<i32> =
                temper_core::listed::filter(&doubled, &(|n: i32| n > 4));
            println!(
                "{}",
                temper_core::listed::join(
                    &big,
                    Arc::new(", ".to_string()),
                    &(|n: i32| Arc::new(n.to_string())),
                )
            );
            let order: temper_core::List<i32> =
                temper_core::listed::sorted(&ns, &(|a: i32, b: i32| a.wrapping_sub(b)));
            println!(
                "{}",
                temper_core::listed::join(
                    &order,
                    Arc::new(" ".to_string()),
                    &(|n: i32| Arc::new(n.to_string())),
                )
            );
            let mut total: i32 = 0;
            {
                let length = temper_core::ListedTrait::len(&ns);
                let mut i2: i32 = 0;
                while i2 < length {
                    let n = temper_core::ListedTrait::get(&ns, i2);
                    i2 = i2.wrapping_add(1);
                    total = total.wrapping_add(n);
                }
            }
            println!("total {total}");
            let button = Button::new("hits", make_counter(0));
            button.press();
            println!("{}", button.press());
            let makers: temper_core::ListBuilder<Arc<dyn Fn() -> i32 + Send + Sync>> =
                temper_core::listed::new_builder();
            {
                let mut i: i32 = 0;
                while i < 3 {
                    let k = i.wrapping_mul(10);
                    temper_core::listed::add(&makers, Arc::new(move || k), None);
                    i = i.wrapping_add(1);
                }
            }
            println!(
                "{}",
                temper_core::listed::join(
                    &makers,
                    Arc::new(" ".to_string()),
                    &(|c: Arc<dyn Fn() -> i32 + Send + Sync>| Arc::new(c().to_string())),
                )
            );
            Ok(())
        })
        .clone()
}

pub(crate) fn make_counter(start: i32) -> Arc<dyn Fn() -> i32 + Send + Sync> {
    let count: Arc<RwLock<i32>> = Arc::new(RwLock::new(start));
    {
        let count = count.clone();
        Arc::new(move || {
            *count.write().unwrap() = temper_core::read_locked(&count).wrapping_add(1);
            temper_core::read_locked(&count)
        })
    }
}

pub(crate) fn twice(
    f: Arc<dyn Fn(i32) -> i32 + Send + Sync>,
) -> Arc<dyn Fn(i32) -> i32 + Send + Sync> {
    {
        let f = f.clone();
        Arc::new(move |x: i32| f(f(x)))
    }
}

pub(crate) fn apply_to(f: Arc<dyn Fn(i32) -> i32 + Send + Sync>, x: i32) -> i32 {
    f(x)
}

pub(crate) fn add_one(x: i32) -> i32 {
    x.wrapping_add(1)
}

struct ScalerStruct {}

#[derive(Clone)]
pub(crate) struct Scaler(Arc<ScalerStruct>);

impl Scaler {
    #[allow(dead_code)]
    pub fn new() -> Scaler {
        Scaler(Arc::new(ScalerStruct {}))
    }

    pub fn triple(x: i32) -> i32 {
        x.wrapping_mul(3)
    }
}

temper_core::impl_any_value_trait!(Scaler, []);

struct ButtonStruct {
    label: Arc<String>,
    on_press: Arc<dyn Fn() -> i32 + Send + Sync>,
}

#[derive(Clone)]
pub(crate) struct Button(Arc<ButtonStruct>);

impl Button {
    pub fn new(label: impl ToArcString, on_press: Arc<dyn Fn() -> i32 + Send + Sync>) -> Button {
        let label = label.to_arc_string();
        Button(Arc::new(ButtonStruct { label, on_press }))
    }

    pub fn press(&self) -> Arc<String> {
        Arc::new(format!("{} {}", self.0.label, (self.0.on_press)()))
    }
}

temper_core::impl_any_value_trait!(Button, []);
