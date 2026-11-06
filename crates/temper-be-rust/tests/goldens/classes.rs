use std::sync::{Arc, OnceLock, RwLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            RECT__UNIT.set(Rect::new(1, 1)).unwrap_or_else(|_| panic!());
            let r = Rect::new(2, 3);
            println!("{r}");
            println!("{}", r.area());
            let s = shape(Shape::new(r.clone()));
            println!("{}", s.describe());
            let q = shape(Shape::new(Square::new(4)));
            println!("{}", q.describe());
            println!("{}", q.area());
            println!("{}", r.scaled(2).scaled(3).area());
            println!("{}", Rect::unit().scaled(5));
            let c = Counter::zero();
            c.bump().bump();
            println!("{}", c.count());
            c.set_doubled(10);
            println!("{}", c.doubled());
            println!("{}", c.same(Counter::new().bump().bump()));
            println!(
                "{} {}",
                Point::new(1, 2) == Point::new(1, 2),
                Point::new(1, 2) != Point::new(2, 1)
            );
            println!(
                "{}",
                Holder::new(Loose::new(3), "a") == Holder::new(Loose::new(-3), "a")
            );
            println!(
                "{} {}",
                Version::new(1, 2).compare_to(Version::new(1, 3)) < 0,
                Version::new(2, 0).compare_to(Version::new(1, 9)) >= 0
            );
            println!(
                "{} {}",
                temper_core::is::<Square>(q.clone()),
                temper_core::is::<Square>(s.clone())
            );
            let sq = temper_core::cast::<Square>(q.clone()).ok_or_else(temper_core::Error::new)?;
            println!("{}", sq.area());
            if temper_core::is::<Rect>(s.clone()) {
                println!("{}", temper_core::cast::<Rect>(s.clone()).unwrap().0.w);
            }
            let _favorite = same(std::any::TypeId::of::<i64>());
            let mut named: i32 = 0;
            {
                let mut i: i32 = 0;
                while i < 6 {
                    type_named(i);
                    named = named.wrapping_add(1);
                    i = i.wrapping_add(1);
                }
            }
            println!("{named} types");
            let sql = Sql::new();
            sql.append_string(Some("it"));
            sql.append_string(None::<Arc<String>>);
            sql.append_int(3);
            sql.append_shape(Shape::new(Square::new(2)));
            sql.append_rect(Rect::new(1, 2));
            println!("{}", sql.accumulated());
            let seven: i32 = 7;
            println!("{}", {
                let accumulator = Sql::new();
                {
                    accumulator.append_safe("SELECT ");
                    accumulator.append_int(seven);
                    accumulator.append_safe(", ");
                    accumulator.append_string(Some("x"));
                    accumulator.append_safe(" FROM ");
                    accumulator.append_rect(Rect::new(2, 2));
                }
                accumulator.accumulated()
            });
            Ok(())
        })
        .clone()
}

pub trait ShapeTrait: temper_core::AsAnyValue + temper_core::AnyValueTrait + Send + Sync {
    fn clone_boxed(&self) -> Shape;

    fn area(&self) -> i32;

    fn describe(&self) -> Arc<String> {
        Arc::new(format!("shape of area {}", self.area()))
    }
}

#[derive(Clone)]
pub struct Shape(Arc<dyn ShapeTrait>);

impl Shape {
    pub fn new(selfish: impl ShapeTrait + 'static) -> Shape {
        Shape(Arc::new(selfish))
    }
}

impl ShapeTrait for Shape {
    fn clone_boxed(&self) -> Shape {
        ShapeTrait::clone_boxed(&*self.0)
    }

    fn area(&self) -> i32 {
        ShapeTrait::area(&*self.0)
    }

    fn describe(&self) -> Arc<String> {
        ShapeTrait::describe(&*self.0)
    }
}

temper_core::impl_any_value_trait_for_interface!(Shape);

impl std::ops::Deref for Shape {
    type Target = dyn ShapeTrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

struct RectStruct {
    w: i32,
    h: i32,
}

#[derive(Clone)]
pub struct Rect(Arc<RectStruct>);

static RECT__UNIT: OnceLock<Rect> = OnceLock::new();

impl Rect {
    pub fn new(w: i32, h: i32) -> Rect {
        Rect(Arc::new(RectStruct { w, h }))
    }

    pub fn unit() -> Rect {
        crate::support::initialized(|| RECT__UNIT.get()).clone()
    }

    pub fn area(&self) -> i32 {
        self.0.w.wrapping_mul(self.0.h)
    }

    pub fn to_string(&self) -> Arc<String> {
        Arc::new(format!("Rect({}, {})", self.0.w, self.0.h))
    }

    pub fn scaled(&self, k: i32) -> Rect {
        Rect::new(self.0.w.wrapping_mul(k), self.0.h.wrapping_mul(k))
    }

    pub fn w(&self) -> i32 {
        self.0.w
    }

    pub fn h(&self) -> i32 {
        self.0.h
    }
}

impl std::fmt::Display for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl ShapeTrait for Rect {
    fn clone_boxed(&self) -> Shape {
        Shape::new(self.clone())
    }

    fn area(&self) -> i32 {
        self.area()
    }
}

temper_core::impl_any_value_trait!(Rect, [Shape]);

struct SquareStruct {
    side: i32,
}

#[derive(Clone)]
pub struct Square(Arc<SquareStruct>);

impl Square {
    pub fn new(side: i32) -> Square {
        Square(Arc::new(SquareStruct { side }))
    }

    pub fn area(&self) -> i32 {
        self.0.side.wrapping_mul(self.0.side)
    }

    pub fn describe(&self) -> Arc<String> {
        Arc::new(format!("square of side {}", self.0.side))
    }
}

impl ShapeTrait for Square {
    fn clone_boxed(&self) -> Shape {
        Shape::new(self.clone())
    }

    fn area(&self) -> i32 {
        self.area()
    }

    fn describe(&self) -> Arc<String> {
        self.describe()
    }
}

temper_core::impl_any_value_trait!(Square, [Shape]);

struct CounterStruct {
    count: RwLock<i32>,
    hidden: RwLock<i32>,
}

#[derive(Clone)]
pub(crate) struct Counter(Arc<CounterStruct>);

impl Counter {
    pub fn new() -> Counter {
        Counter(Arc::new(CounterStruct {
            count: RwLock::new(0),
            hidden: RwLock::new(0),
        }))
    }

    pub fn doubled(&self) -> i32 {
        self.count().wrapping_mul(2)
    }

    pub fn set_doubled(&self, value: i32) {
        *self.0.count.write().unwrap() = value.wrapping_sub(self.count());
    }

    pub fn bump(&self) -> Counter {
        *self.0.count.write().unwrap() = self.count().wrapping_add(1);
        *self.0.hidden.write().unwrap() = self.hidden().wrapping_add(1);
        self.clone()
    }

    pub fn same(&self, other: Counter) -> bool {
        self.hidden() == other.hidden()
    }

    pub fn zero() -> Counter {
        Counter::new()
    }

    pub fn count(&self) -> i32 {
        *self.0.count.read().unwrap()
    }

    fn hidden(&self) -> i32 {
        *self.0.hidden.read().unwrap()
    }
}

temper_core::impl_any_value_trait!(Counter, []);

struct PointStruct {
    x: i32,
    y: i32,
}

#[derive(Clone)]
pub(crate) struct Point(Arc<PointStruct>);

impl Point {
    pub fn new(x: i32, y: i32) -> Point {
        Point(Arc::new(PointStruct { x, y }))
    }

    pub fn eq(&self, other: Point) -> bool {
        if self.0.x == other.0.x {
            self.0.y == other.0.y
        } else {
            false
        }
    }
}

impl PartialEq for Point {
    fn eq(&self, other: &Point) -> bool {
        Point::eq(self, other.clone())
    }
}

temper_core::impl_any_value_trait!(Point, []);

struct LooseStruct {
    n: i32,
}

#[derive(Clone)]
pub(crate) struct Loose(Arc<LooseStruct>);

impl Loose {
    pub fn new(n: i32) -> Loose {
        Loose(Arc::new(LooseStruct { n }))
    }

    pub fn eq(&self, other: Loose) -> bool {
        if self.0.n == other.0.n {
            true
        } else {
            self.0.n == other.0.n.wrapping_neg()
        }
    }
}

impl PartialEq for Loose {
    fn eq(&self, other: &Loose) -> bool {
        Loose::eq(self, other.clone())
    }
}

temper_core::impl_any_value_trait!(Loose, []);

struct HolderStruct {
    loose: Loose,
    tag: Arc<String>,
}

#[derive(Clone)]
pub(crate) struct Holder(Arc<HolderStruct>);

impl Holder {
    pub fn new(loose: Loose, tag: impl ToArcString) -> Holder {
        let tag = tag.to_arc_string();
        Holder(Arc::new(HolderStruct { loose, tag }))
    }

    pub fn eq(&self, other: Holder) -> bool {
        if self.0.loose == other.0.loose {
            self.0.tag == other.0.tag
        } else {
            false
        }
    }
}

impl PartialEq for Holder {
    fn eq(&self, other: &Holder) -> bool {
        Holder::eq(self, other.clone())
    }
}

temper_core::impl_any_value_trait!(Holder, []);

struct VersionStruct {
    major: i32,
    minor: i32,
}

#[derive(Clone)]
pub(crate) struct Version(Arc<VersionStruct>);

impl Version {
    pub fn new(major: i32, minor: i32) -> Version {
        Version(Arc::new(VersionStruct { major, minor }))
    }

    pub fn compare_to(&self, other: Version) -> i32 {
        if self.0.major != other.0.major {
            self.0.major.cmp(&other.0.major) as i32
        } else {
            self.0.minor.cmp(&other.0.minor) as i32
        }
    }
}

temper_core::impl_any_value_trait!(Version, []);

pub(crate) fn shape(x: Shape) -> Shape {
    x.clone()
}

pub(crate) fn type_named(n: i32) -> std::any::TypeId {
    if n == 0 {
        std::any::TypeId::of::<()>()
    } else if n == 1 {
        std::any::TypeId::of::<bool>()
    } else if n == 2 {
        std::any::TypeId::of::<i32>()
    } else if n == 3 {
        std::any::TypeId::of::<i64>()
    } else if n == 4 {
        std::any::TypeId::of::<f64>()
    } else {
        std::any::TypeId::of::<Arc<String>>()
    }
}

pub(crate) fn same(t: std::any::TypeId) -> std::any::TypeId {
    t
}

struct SqlStruct {
    parts: temper_core::ListBuilder<Arc<String>>,
}

#[derive(Clone)]
pub(crate) struct Sql(Arc<SqlStruct>);

impl Sql {
    pub fn new() -> Sql {
        Sql(Arc::new(SqlStruct {
            parts: temper_core::listed::new_builder(),
        }))
    }

    pub fn append_safe(&self, s: impl ToArcString) {
        let s = s.to_arc_string();
        temper_core::listed::add(&self.0.parts, s.clone(), None);
    }

    pub fn append_string(&self, x: Option<impl ToArcString>) {
        let x = x.map(|x| x.to_arc_string());
        temper_core::listed::add(
            &self.0.parts,
            if let Some(x) = &x {
                Arc::new(format!("'{x}'"))
            } else {
                Arc::new("NULL".to_string())
            },
            None,
        );
    }

    pub fn append_int(&self, x: i32) {
        temper_core::listed::add(&self.0.parts, Arc::new(x.to_string()), None);
    }

    pub fn append_shape(&self, x: Shape) {
        temper_core::listed::add(&self.0.parts, Arc::new(format!("shape {}", x.area())), None);
    }

    pub fn append_rect(&self, x: Rect) {
        temper_core::listed::add(&self.0.parts, Arc::new(format!("rect {x}")), None);
    }

    pub fn accumulated(&self) -> Arc<String> {
        temper_core::listed::join(
            &self.0.parts,
            Arc::new(String::new()),
            &(|p: Arc<String>| p.clone()),
        )
    }
}

temper_core::impl_any_value_trait!(Sql, []);

pub mod builders {
    use super::*;

    #[derive(Clone)]
    pub struct RectBuilder {
        pub w: i32,
        pub h: i32,
    }

    impl RectBuilder {
        pub fn build(self) -> Rect {
            Rect::new(self.w, self.h)
        }
    }
}
