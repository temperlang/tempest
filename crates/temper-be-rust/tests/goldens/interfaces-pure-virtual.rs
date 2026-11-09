use std::sync::{Arc, OnceLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let greeter = Greeter::new(HiGreeter::new());
            greeter.greet();
            let box_wrapper: Wrapper<Box> = Wrapper::new(BoxWrapper::new());
            let int_wrapper: Wrapper<i32> = Wrapper::new(IntWrapper::new());
            println!("{}", box_wrapper.wrap(Box::new("box")).0.label);
            println!("{}", int_wrapper.wrap(41));
            Ok(())
        })
        .clone()
}

pub(crate) trait GreeterTrait:
    temper_core::AsAnyValue + temper_core::AnyValueTrait + Send + Sync
{
    #[allow(dead_code)]
    fn clone_boxed(&self) -> Greeter;

    fn greet(&self);
}

#[derive(Clone)]
pub(crate) struct Greeter(Arc<dyn GreeterTrait>);

impl Greeter {
    pub fn new(selfish: impl GreeterTrait + 'static) -> Greeter {
        Greeter(Arc::new(selfish))
    }
}

impl GreeterTrait for Greeter {
    fn clone_boxed(&self) -> Greeter {
        GreeterTrait::clone_boxed(&*self.0)
    }

    fn greet(&self) {
        GreeterTrait::greet(&*self.0)
    }
}

temper_core::impl_any_value_trait_for_interface!(Greeter);

impl std::ops::Deref for Greeter {
    type Target = dyn GreeterTrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

struct HiGreeterStruct {}

#[derive(Clone)]
pub(crate) struct HiGreeter(Arc<HiGreeterStruct>);

impl HiGreeter {
    pub fn new() -> HiGreeter {
        HiGreeter(Arc::new(HiGreeterStruct {}))
    }

    pub fn greet(&self) {
        println!("Hi, virtually!");
    }
}

impl GreeterTrait for HiGreeter {
    fn clone_boxed(&self) -> Greeter {
        Greeter::new(self.clone())
    }

    fn greet(&self) {
        self.greet()
    }
}

temper_core::impl_any_value_trait!(HiGreeter, [Greeter]);

pub(crate) trait WrapperTrait<T: Clone + Send + Sync + 'static>:
    temper_core::AsAnyValue + temper_core::AnyValueTrait + Send + Sync
{
    #[allow(dead_code)]
    fn clone_boxed(&self) -> Wrapper<T>;

    fn wrap(&self, item: T) -> T;
}

#[derive(Clone)]
pub(crate) struct Wrapper<T: Clone + Send + Sync + 'static>(Arc<dyn WrapperTrait<T>>);

impl<T: Clone + Send + Sync + 'static> Wrapper<T> {
    pub fn new(selfish: impl WrapperTrait<T> + 'static) -> Wrapper<T> {
        Wrapper(Arc::new(selfish))
    }
}

impl<T: Clone + Send + Sync + 'static> WrapperTrait<T> for Wrapper<T> {
    fn clone_boxed(&self) -> Wrapper<T> {
        WrapperTrait::clone_boxed(&*self.0)
    }

    fn wrap(&self, item: T) -> T {
        WrapperTrait::wrap(&*self.0, item)
    }
}

temper_core::impl_any_value_trait_for_interface!(Wrapper<T>);

impl<T: Clone + Send + Sync + 'static> std::ops::Deref for Wrapper<T> {
    type Target = dyn WrapperTrait<T>;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

struct BoxStruct {
    label: Arc<String>,
}

#[derive(Clone)]
pub(crate) struct Box(Arc<BoxStruct>);

impl Box {
    pub fn new(label: impl ToArcString) -> Box {
        let label = label.to_arc_string();
        Box(Arc::new(BoxStruct { label }))
    }
}

temper_core::impl_any_value_trait!(Box, []);

struct BoxWrapperStruct {}

#[derive(Clone)]
pub(crate) struct BoxWrapper(Arc<BoxWrapperStruct>);

impl BoxWrapper {
    pub fn new() -> BoxWrapper {
        BoxWrapper(Arc::new(BoxWrapperStruct {}))
    }

    pub fn wrap(&self, mut item: Box) -> Box {
        item = Box::new(Arc::new(format!("[{}]", item.0.label)));
        item.clone()
    }
}

impl WrapperTrait<Box> for BoxWrapper {
    fn clone_boxed(&self) -> Wrapper<Box> {
        Wrapper::new(self.clone())
    }

    fn wrap(&self, item: Box) -> Box {
        self.wrap(item)
    }
}

temper_core::impl_any_value_trait!(BoxWrapper, [Wrapper<Box>]);

struct IntWrapperStruct {}

#[derive(Clone)]
pub(crate) struct IntWrapper(Arc<IntWrapperStruct>);

impl IntWrapper {
    pub fn new() -> IntWrapper {
        IntWrapper(Arc::new(IntWrapperStruct {}))
    }

    pub fn wrap(&self, item: i32) -> i32 {
        item.wrapping_add(1)
    }
}

impl WrapperTrait<i32> for IntWrapper {
    fn clone_boxed(&self) -> Wrapper<i32> {
        Wrapper::new(self.clone())
    }

    fn wrap(&self, item: i32) -> i32 {
        self.wrap(item)
    }
}

temper_core::impl_any_value_trait!(IntWrapper, [Wrapper<i32>]);
