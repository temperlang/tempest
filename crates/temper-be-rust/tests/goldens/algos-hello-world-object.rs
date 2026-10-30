use std::sync::{Arc, OnceLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let c = C::new("Hello", "World");
            println!("{}, {}!", c.0.x, c.0.y);
            c.echo("Hello World, again!");
            Ok(())
        })
        .clone()
}

struct CStruct {
    x: Arc<String>,
    y: Arc<String>,
}

#[derive(Clone)]
pub(crate) struct C(Arc<CStruct>);

impl C {
    pub fn new(x: impl ToArcString, y: impl ToArcString) -> C {
        let x = x.to_arc_string();
        let y = y.to_arc_string();
        C(Arc::new(CStruct { x, y }))
    }

    pub fn echo(&self, input: impl ToArcString) {
        let input = input.to_arc_string();
        println!("{input}");
    }
}

temper_core::impl_any_value_trait!(C, []);
