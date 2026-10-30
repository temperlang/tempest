use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE.get_or_init(|| Ok(())).clone()
}

pub trait StringableTrait:
    temper_core::AsAnyValue + temper_core::AnyValueTrait + Send + Sync
{
    fn clone_boxed(&self) -> Stringable;

    fn to_string(&self) -> Arc<String>;
}

#[derive(Clone)]
pub struct Stringable(Arc<dyn StringableTrait>);

impl Stringable {
    pub fn new(selfish: impl StringableTrait + 'static) -> Stringable {
        Stringable(Arc::new(selfish))
    }
}

impl StringableTrait for Stringable {
    fn clone_boxed(&self) -> Stringable {
        StringableTrait::clone_boxed(&*self.0)
    }

    fn to_string(&self) -> Arc<String> {
        StringableTrait::to_string(&*self.0)
    }
}

temper_core::impl_any_value_trait_for_interface!(Stringable);

impl std::ops::Deref for Stringable {
    type Target = dyn StringableTrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

struct CircleStruct {
    radius: i32,
}

#[derive(Clone)]
pub struct Circle(Arc<CircleStruct>);

impl Circle {
    pub fn new(radius: i32) -> Circle {
        Circle(Arc::new(CircleStruct { radius }))
    }

    pub fn to_string(&self) -> Arc<String> {
        Arc::new(format!("Circle of radius {}", self.0.radius))
    }

    pub fn radius(&self) -> i32 {
        self.0.radius
    }
}

impl std::fmt::Display for Circle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl StringableTrait for Circle {
    fn clone_boxed(&self) -> Stringable {
        Stringable::new(self.clone())
    }

    fn to_string(&self) -> Arc<String> {
        self.to_string()
    }
}

temper_core::impl_any_value_trait!(Circle, [Stringable]);

struct RectangleStruct {
    width: i32,
    height: i32,
}

#[derive(Clone)]
pub struct Rectangle(Arc<RectangleStruct>);

impl Rectangle {
    pub fn new(width: i32, height: i32) -> Rectangle {
        Rectangle(Arc::new(RectangleStruct { width, height }))
    }

    pub fn width(&self) -> i32 {
        self.0.width
    }

    pub fn height(&self) -> i32 {
        self.0.height
    }
}

temper_core::impl_any_value_trait!(Rectangle, []);

pub mod builders {
    use super::*;

    #[derive(Clone)]
    pub struct RectangleBuilder {
        pub width: i32,
        pub height: i32,
    }

    impl RectangleBuilder {
        pub fn build(self) -> Rectangle {
            Rectangle::new(self.width, self.height)
        }
    }
}
