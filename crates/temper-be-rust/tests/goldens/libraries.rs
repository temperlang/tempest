use std::sync::{Arc, OnceLock, RwLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            CASING__FIRST
                .set(Casing::new("first"))
                .unwrap_or_else(|_| panic!());
            CASING__LAST
                .set(Casing::named("last"))
                .unwrap_or_else(|_| panic!());
            *TICKS.write().unwrap() = Some(0);
            EXTRAS.set(Arc::new(vec![1])).unwrap_or_else(|_| panic!());
            let grid = Grid::new(2, Arc::new(vec![1.0, 2.0, 3.0, 4.0]));
            println!(
                "{} {} {}",
                rows(grid.0.values.clone(), 2),
                temper_core::float64::to_string(half(3.0)),
                temper_core::float64::to_string(cube(8.0))
            );
            println!("{}", temper_core::float64::to_string(corner(grid.clone())));
            println!(
                "{}",
                temper_core::float64::to_string(total(grid.0.values.clone()))
            );
            let none = text(0);
            let some = count(3);
            println!(
                "{} {} {}",
                {
                    let part = none.clone();
                    part.clone().unwrap_or_else(|| Arc::new("null".to_string()))
                },
                {
                    let part2 = some;
                    if let Some(part2) = part2 {
                        Arc::new(part2.to_string())
                    } else {
                        Arc::new("null".to_string())
                    }
                },
                {
                    let part3 = {
                        let subject = maybe(0);
                        subject.as_ref().map(|subject| subject.0.ncols)
                    };
                    if let Some(part3) = part3 {
                        Arc::new(part3.to_string())
                    } else {
                        Arc::new("null".to_string())
                    }
                }
            );
            println!(
                "{} {} {}",
                maybe(1).is_some(),
                maybe(0).is_some(),
                Grid::parse("").is_none()
            );
            println!("{}", Casing::first());
            println!(
                "{}",
                temper_core::listed::join(
                    &mapped(grid.clone()),
                    Arc::new(",".to_string()),
                    &(|x: f64| temper_core::float64::to_string(x)),
                )
            );
            println!(
                "{}",
                temper_core::listed::join(
                    &cubes(grid.clone()),
                    Arc::new(",".to_string()),
                    &(|x: f64| temper_core::float64::to_string(x)),
                )
            );
            println!(
                "{} {} {}",
                {
                    let left = casing(1);
                    if let Some(left) = &left {
                        left.eq(Casing::first())
                    } else {
                        false
                    }
                },
                {
                    let left2 = casing(0);
                    if let Some(left2) = &left2 {
                        left2.eq(Casing::first())
                    } else {
                        false
                    }
                },
                !{
                    let left3 = casing(0);
                    let right = casing(0);
                    if let Some(left3) = &left3 {
                        if let Some(right) = &right {
                            left3.eq(right.clone())
                        } else {
                            false
                        }
                    } else {
                        right.is_none()
                    }
                }
            );
            println!("{} {}", captured(""), captured("2.5"));
            println!("{}", bidi().as_str() != "lookout");
            println!(
                "{} {} {}",
                word(1).as_str().cmp(word(2).as_str()) as i32,
                word(2).as_str().cmp(word(1).as_str()) as i32,
                word(2).as_str().cmp(word(3).as_str()) as i32
            );
            let named = word(2);
            println!(
                "{}",
                named == word(3) && format!("{named}!").as_str() != named.as_str()
            );
            let digits: temper_core::List<i32> = Arc::new(vec![1, 2, 4, 3]);
            println!(
                "{} {} {} {} {}",
                byte_at(digits.clone(), true, 1),
                byte_at(digits.clone(), false, 0),
                ascending(digits.clone()),
                first_drop(digits.clone()),
                size_with(digits.clone())
            );
            println!(
                "{} {} {} {}",
                tick_if(true),
                tick_if(false),
                ticked(),
                tick_while()
            );
            println!(
                "{} {} {} {} {} {} {}",
                shape_name(shape_of(1)),
                shape_name(shape_of(2)),
                temper_core::ListedTrait::len(&sizes(shape_of(1))),
                temper_core::ListedTrait::len(&sizes(shape_of(2))),
                {
                    let part4 = Dot::new(1).grown();
                    if let Some(part4) = part4 {
                        Arc::new(part4.to_string())
                    } else {
                        Arc::new("null".to_string())
                    }
                },
                {
                    let part5 = Dot::new(3).grown();
                    if let Some(part5) = part5 {
                        Arc::new(part5.to_string())
                    } else {
                        Arc::new("null".to_string())
                    }
                },
                Casing::last()
            );
            let carry = Carry::new(3);
            println!(
                "{} {} {} {} {} {} {}",
                carry.0.carrying,
                carry.0.state,
                Carry::new(0).0.carrying,
                {
                    let part6 = tagged(None::<Arc<String>>).0.label.clone();
                    part6
                        .clone()
                        .unwrap_or_else(|| Arc::new("null".to_string()))
                },
                {
                    let part7 = tagged(Some("x")).0.label.clone();
                    part7
                        .clone()
                        .unwrap_or_else(|| Arc::new("null".to_string()))
                },
                weight(42),
                weight(60)
            );
            Ok(())
        })
        .clone()
}

struct GridStruct {
    ncols: i32,
    values: temper_core::List<f64>,
}

#[derive(Clone)]
pub struct Grid(Arc<GridStruct>);

impl Grid {
    pub fn new(ncols: i32, values: impl temper_core::ToList<f64>) -> Grid {
        let values = values.to_list();
        Grid(Arc::new(GridStruct { ncols, values }))
    }

    pub fn get(&self, i: i32, j: i32) -> temper_core::Result<f64> {
        Ok(temper_core::ListedTrait::get(
            &self.0.values,
            i.wrapping_mul(self.0.ncols).wrapping_add(j),
        ))
    }

    pub fn parse(s: impl ToArcString) -> Option<Grid> {
        let s = s.to_arc_string();
        (|| Ok::<_, temper_core::Error>(Some(parse_grid(s.clone())?)))().unwrap_or(None)
    }

    pub fn ncols(&self) -> i32 {
        self.0.ncols
    }

    pub fn values(&self) -> temper_core::List<f64> {
        self.0.values.clone()
    }
}

temper_core::impl_any_value_trait!(Grid, []);

pub fn parse_grid(s: impl ToArcString) -> temper_core::Result<Grid> {
    let s = s.to_arc_string();
    if s.as_str() == "" {
        return Err(temper_core::Error::new());
    }
    Ok(Grid::new(
        1,
        Arc::new(vec![temper_core::string::to_float64(&s)?]),
    ))
}

struct CasingStruct {
    name: Arc<String>,
}

#[derive(Clone)]
pub struct Casing(Arc<CasingStruct>);

static CASING__FIRST: OnceLock<Casing> = OnceLock::new();

static CASING__LAST: OnceLock<Casing> = OnceLock::new();

impl Casing {
    #[allow(clippy::should_implement_trait)]
    pub fn eq(&self, other: Casing) -> bool {
        self.0.name == other.0.name
    }

    fn new(name: impl ToArcString) -> Casing {
        let name = name.to_arc_string();
        Casing(Arc::new(CasingStruct { name }))
    }

    pub fn to_string(&self) -> Arc<String> {
        self.0.name.clone()
    }

    pub fn first() -> Casing {
        crate::support::initialized(|| CASING__FIRST.get()).clone()
    }

    fn named(name: impl ToArcString) -> Casing {
        let name = name.to_arc_string();
        Casing::new(name.clone())
    }

    pub fn last() -> Casing {
        crate::support::initialized(|| CASING__LAST.get()).clone()
    }

    pub fn name(&self) -> Arc<String> {
        self.0.name.clone()
    }
}

impl std::fmt::Display for Casing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl PartialEq for Casing {
    fn eq(&self, other: &Casing) -> bool {
        Casing::eq(self, other.clone())
    }
}

temper_core::impl_any_value_trait!(Casing, []);

pub trait ShapeTrait: temper_core::AsAnyValue + temper_core::AnyValueTrait + Send + Sync {
    fn clone_boxed(&self) -> Shape;

    fn size(&self) -> i32;
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

    fn size(&self) -> i32 {
        ShapeTrait::size(&*self.0)
    }
}

temper_core::impl_any_value_trait_for_interface!(Shape);

impl std::ops::Deref for Shape {
    type Target = dyn ShapeTrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

struct DotStruct {
    size: i32,
}

#[derive(Clone)]
pub struct Dot(Arc<DotStruct>);

impl Dot {
    pub fn new(size: i32) -> Dot {
        Dot(Arc::new(DotStruct { size }))
    }

    pub fn bigger(&self) -> Option<Dot> {
        if self.0.size < 3 {
            Some(Dot::new(self.0.size.wrapping_add(1)))
        } else {
            None
        }
    }

    pub fn grown(&self) -> Option<i32> {
        let subject = self.bigger();
        subject.as_ref().map(|subject| subject.scaled(10))
    }

    pub fn scaled(&self, k: i32) -> i32 {
        self.0.size.wrapping_mul(k)
    }

    pub fn size(&self) -> i32 {
        self.0.size
    }
}

impl ShapeTrait for Dot {
    fn clone_boxed(&self) -> Shape {
        Shape::new(self.clone())
    }

    fn size(&self) -> i32 {
        self.size()
    }
}

temper_core::impl_any_value_trait!(Dot, [Shape]);

struct LineStruct {
    size: i32,
}

#[derive(Clone)]
pub struct Line(Arc<LineStruct>);

impl Line {
    pub fn new(size: i32) -> Line {
        Line(Arc::new(LineStruct { size }))
    }

    pub fn size(&self) -> i32 {
        self.0.size
    }
}

impl ShapeTrait for Line {
    fn clone_boxed(&self) -> Shape {
        Shape::new(self.clone())
    }

    fn size(&self) -> i32 {
        self.size()
    }
}

temper_core::impl_any_value_trait!(Line, [Shape]);

struct CarryStruct {
    carrying: bool,
    state: i32,
}

#[derive(Clone)]
pub struct Carry(Arc<CarryStruct>);

impl Carry {
    pub fn new(start: i32) -> Carry {
        let mut carrying: bool;
        let mut state: i32;
        carrying = false;
        state = 0;
        if start != 0 {
            carrying = true;
            state = start;
        }
        Carry(Arc::new(CarryStruct { carrying, state }))
    }

    pub fn carrying(&self) -> bool {
        self.0.carrying
    }

    pub fn state(&self) -> i32 {
        self.0.state
    }
}

temper_core::impl_any_value_trait!(Carry, []);

struct TaggedStruct {
    label: Option<Arc<String>>,
}

#[derive(Clone)]
pub struct Tagged(Arc<TaggedStruct>);

impl Tagged {
    pub fn new(label: Option<impl ToArcString>) -> Tagged {
        let label = label.map(|x| x.to_arc_string());
        Tagged(Arc::new(TaggedStruct { label }))
    }

    pub fn label(&self) -> Option<Arc<String>> {
        self.0.label.clone()
    }
}

temper_core::impl_any_value_trait!(Tagged, []);

pub(crate) fn tagged(label: Option<impl ToArcString>) -> Tagged {
    let label = label.map(|x| x.to_arc_string());
    Tagged::new(label.clone())
}

pub(crate) fn weight(c: i32) -> i32 {
    let base: i32 = if c == 42 { 0 } else { 6 };
    (if c > 50 { 1i32 } else { 2 })
        .wrapping_add(base)
        .wrapping_add(c)
}

pub(crate) fn shape_name(s: Shape) -> Arc<String> {
    let name = if temper_core::is::<Dot>(s.clone()) {
        Arc::new("dot".to_string())
    } else if temper_core::is::<Line>(s.clone()) {
        Arc::new("line".to_string())
    } else {
        panic!()
    };
    name.clone()
}

pub(crate) fn shape_of(n: i32) -> Shape {
    let shape = if n > 1 {
        Shape::new(Line::new(n))
    } else {
        Shape::new(Dot::new(n))
    };
    shape.clone()
}

pub(crate) fn sizes(s: Shape) -> temper_core::List<i32> {
    let found: temper_core::List<i32> = if temper_core::is::<Dot>(s.clone()) {
        Arc::new(vec![
            temper_core::cast::<Dot>(s.clone()).unwrap().0.size,
            temper_core::cast::<Dot>(s.clone()).unwrap().0.size,
        ])
    } else {
        Arc::new(vec![])
    };
    found.clone()
}

pub(crate) fn rows(values: impl temper_core::ToList<f64>, ncols: i32) -> i32 {
    let values = values.to_list();
    temper_core::int_div(temper_core::ListedTrait::len(&values), ncols).unwrap_or_else(|_| panic!())
}

pub(crate) fn scaled(grid: Grid) -> temper_core::Result<Grid> {
    if grid.0.ncols == 0 {
        return Err(temper_core::Error::new());
    }
    Ok(grid.clone())
}

pub(crate) fn cubes(grid: Grid) -> temper_core::List<f64> {
    (|| {
        let more: temper_core::List<f64> =
            temper_core::listed::map(&scaled(grid.clone())?.0.values, &(|x: f64| x * x * x));
        Ok::<_, temper_core::Error>(more.clone())
    })()
    .unwrap_or_else(|_| panic!())
}

pub(crate) fn each(values: impl temper_core::ToList<f64>, f: Arc<dyn Fn(f64) + Send + Sync>) {
    let values = values.to_list();
    let mut i: i32 = 0;
    while i < temper_core::ListedTrait::len(&values) {
        f(temper_core::ListedTrait::get(&values, i));
        i = i.wrapping_add(1);
    }
}

pub(crate) fn total(values: impl temper_core::ToList<f64>) -> f64 {
    let values = values.to_list();
    let sum: Arc<RwLock<f64>> = Arc::new(RwLock::new(0.0));
    each(values.clone(), {
        let sum = sum.clone();
        Arc::new(move |x: f64| {
            if (|| {
                let part = temper_core::float64::div(x, 2.0)?;
                *sum.write().unwrap() = temper_core::read_locked(&sum) + part;
                Ok::<_, temper_core::Error>(())
            })()
            .is_err()
            {
                panic!();
            }
        })
    });
    temper_core::read_locked(&sum)
}

pub(crate) fn half(x: f64) -> f64 {
    temper_core::float64::div(x, 2.0).unwrap_or_else(|_| panic!())
}

pub(crate) fn cube(x: f64) -> f64 {
    x * 0.25
}

pub(crate) fn corner(grid: Grid) -> f64 {
    grid.get(1, 1).unwrap_or(-1.0)
}

pub(crate) fn maybe(n: i32) -> Option<Grid> {
    if n > 0 {
        Some(Grid::new(n, Arc::new(vec![1.0])))
    } else {
        None
    }
}

pub(crate) fn mapped(grid: Grid) -> temper_core::List<f64> {
    temper_core::listed::map(
        &scaled(grid.clone()).unwrap_or_else(|_| panic!()).0.values,
        &(|x: f64| {
            if temper_core::float64::cmp(x, 1.5) > 0 {
                x * 2.0
            } else {
                x
            }
        }),
    )
}

pub(crate) fn byte_at(digits: impl temper_core::ToList<i32>, short: bool, c: i32) -> i32 {
    let digits = digits.to_list();
    if short {
        temper_core::ListedTrait::get(&digits, c).wrapping_mul(17)
    } else {
        temper_core::ListedTrait::get(&digits, 2i32.wrapping_mul(c))
            .wrapping_mul(16)
            .wrapping_add(temper_core::ListedTrait::get(
                &digits,
                2i32.wrapping_mul(c).wrapping_add(1),
            ))
    }
}

pub(crate) fn ascending(digits: impl temper_core::ToList<i32>) -> bool {
    let digits = digits.to_list();
    if temper_core::ListedTrait::len(&digits) > 1 {
        temper_core::ListedTrait::get(&digits, 0).wrapping_add(1)
            == temper_core::ListedTrait::get(&digits, 1)
    } else {
        false
    }
}

pub(crate) fn first_drop(digits: impl temper_core::ToList<i32>) -> i32 {
    let digits = digits.to_list();
    let mut i: i32 = 1;
    while i < temper_core::ListedTrait::len(&digits)
        && temper_core::ListedTrait::get(&digits, i.wrapping_sub(1))
            <= temper_core::ListedTrait::get(&digits, i)
    {
        i = i.wrapping_add(1);
    }
    i
}

static TICKS: RwLock<Option<i32>> = RwLock::new(None);

pub(crate) fn ticks() -> i32 {
    crate::support::initialized(|| *TICKS.read().unwrap())
}

pub(crate) fn tick() -> i32 {
    crate::support::initialized(|| {
        let mut ticks = TICKS.write().unwrap();
        *ticks = Some((*ticks)?.wrapping_add(1));
        Some(())
    });
    ticks()
}

pub(crate) fn tick_if(short: bool) -> i32 {
    if short {
        tick()
    } else {
        tick().wrapping_mul(10).wrapping_add(tick())
    }
}

pub(crate) fn ticked() -> bool {
    if ticks() > 0 {
        tick().wrapping_mul(10).wrapping_add(tick()) > 0
    } else {
        false
    }
}

pub(crate) fn tick_while() -> i32 {
    let mut i: i32 = 0;
    while i < 3 && tick() < tick() {
        i = i.wrapping_add(1);
    }
    i
}

static EXTRAS: OnceLock<temper_core::List<i32>> = OnceLock::new();

pub(crate) fn extras() -> temper_core::List<i32> {
    crate::support::initialized(|| EXTRAS.get()).clone()
}

pub fn size_with<T: Clone + Send + Sync + 'static>(items: impl temper_core::ToList<T>) -> i32 {
    let items = items.to_list();
    temper_core::ListedTrait::len(&items).wrapping_add(temper_core::ListedTrait::len(&extras()))
}

pub(crate) fn text(n: i32) -> Option<Arc<String>> {
    if n > 0 {
        Some(Arc::new("x".to_string()))
    } else {
        None
    }
}

pub(crate) fn count(n: i32) -> Option<i32> {
    if n > 0 {
        Some(n)
    } else {
        None
    }
}

pub(crate) fn casing(n: i32) -> Option<Casing> {
    if n > 0 {
        Some(Casing::first())
    } else {
        None
    }
}

pub(crate) fn captured(s: impl ToArcString) -> Arc<String> {
    let s = s.to_arc_string();
    let found = (|| Ok::<_, temper_core::Error>(Some(parse_grid(s.clone())?)))().unwrap_or(None);
    temper_core::listed::join(
        &temper_core::listed::map(&Arc::new(vec![1, 2]), &(|_x: i32| found.is_none())),
        Arc::new(",".to_string()),
        &(|b: bool| Arc::new(b.to_string())),
    )
}

pub(crate) fn bidi() -> Arc<String> {
    Arc::new("look\u{202e}out".to_string())
}

pub(crate) fn word(n: i32) -> Arc<String> {
    if n > 1 {
        Arc::new("b".to_string())
    } else {
        Arc::new("a".to_string())
    }
}

pub mod builders {
    use super::*;

    #[derive(Clone)]
    pub struct GridBuilder {
        pub ncols: i32,
        pub values: temper_core::List<f64>,
    }

    impl GridBuilder {
        pub fn build(self) -> Grid {
            Grid::new(self.ncols, self.values)
        }
    }
}
