use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let empty_string = Arc::new(String::new());
            let foo = Arc::new("foo".to_string());
            println!(
                "{}",
                if empty_string.is_empty() {
                    "emptyString is empty"
                } else {
                    "emptyString is not empty"
                }
            );
            println!(
                "{}",
                if foo.is_empty() {
                    "foo is empty"
                } else {
                    "foo is not empty"
                }
            );
            Ok(())
        })
        .clone()
}
