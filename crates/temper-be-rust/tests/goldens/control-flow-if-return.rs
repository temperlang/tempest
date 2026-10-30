use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let mut t = true;
            t = t;
            let f = !t;
            println!("explicit    true  -> {}", explicit(t));
            println!("explicit    false -> {}", explicit(f));
            println!("halfAndHalf true  -> {}", half_and_half(t));
            println!("halfAndHalf false -> {}", half_and_half(f));
            println!("implicit    true  -> {}", implicit(t));
            println!("implicit    false -> {}", implicit(f));
            Ok(())
        })
        .clone()
}

pub(crate) fn explicit(b: bool) -> Arc<String> {
    if b {
        return Arc::new("true".to_string());
    }
    Arc::new("false".to_string())
}

pub(crate) fn half_and_half(b: bool) -> Arc<String> {
    if b {
        return Arc::new("true".to_string());
    }
    Arc::new("false".to_string())
}

pub(crate) fn implicit(b: bool) -> Arc<String> {
    if b {
        Arc::new("true".to_string())
    } else {
        Arc::new("false".to_string())
    }
}
