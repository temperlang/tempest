use std::sync::{Arc, OnceLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let something = Something::new("Hello, World!");
            something.say_message();
            Ok(())
        })
        .clone()
}

pub(crate) fn say(message: impl ToArcString) {
    let message = message.to_arc_string();
    println!("{message}");
}

struct SomethingStruct {
    message: Arc<String>,
}

#[derive(Clone)]
pub(crate) struct Something(Arc<SomethingStruct>);

impl Something {
    pub fn new(message: impl ToArcString) -> Something {
        let message = message.to_arc_string();
        Something(Arc::new(SomethingStruct { message }))
    }

    pub fn say_message(&self) {
        say(self.0.message.clone());
    }
}

temper_core::impl_any_value_trait!(Something, []);
