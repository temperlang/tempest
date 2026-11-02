use std::sync::OnceLock;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let counter = Counter::new();
            counter.add(2);
            counter.add(3);
            println!("counter.total(): {}", counter.total());
            println!("new Doubler().twice(5): {}", Doubler::new().twice(5));
            Ok(())
        })
        .clone()
}

pub use crate::_connected::Counter;
pub use crate::_connected::Doubler;
