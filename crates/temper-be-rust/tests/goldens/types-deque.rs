use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            DEQUE
                .set(temper_core::deque::new())
                .unwrap_or_else(|_| panic!());
            check_empty();
            if !temper_core::deque::is_empty(&deque()) {
                println!("{}", temper_core::deque::remove_first(&deque()));
            }
            temper_core::deque::add(&deque(), Arc::new("foo".to_string()));
            temper_core::deque::add(&deque(), Arc::new("bar".to_string()));
            println!("{}", temper_core::deque::remove_first(&deque()));
            temper_core::deque::add(&deque(), Arc::new("baz".to_string()));
            println!("{}", temper_core::deque::remove_first(&deque()));
            temper_core::deque::add(&deque(), Arc::new("boo".to_string()));
            check_empty();
            while !temper_core::deque::is_empty(&deque()) {
                println!("{}", temper_core::deque::remove_first(&deque()));
            }
            check_empty();
            Ok(())
        })
        .clone()
}

static DEQUE: OnceLock<temper_core::Deque<Arc<String>>> = OnceLock::new();

pub(crate) fn deque() -> temper_core::Deque<Arc<String>> {
    crate::support::initialized(|| DEQUE.get()).clone()
}

pub(crate) fn check_empty() {
    println!(
        "{}",
        if temper_core::deque::is_empty(&deque()) {
            "Empty"
        } else {
            "Not empty"
        }
    );
}
