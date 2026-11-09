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
            println!("nested      true  false -> {}", nested(t, f));
            println!("nested      false true  -> {}", nested(f, t));
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

pub(crate) fn nested(a: bool, b: bool) -> Arc<String> {
    let mut result = Arc::new("neither".to_string());
    if a {
        if b {
            result = Arc::new("both".to_string());
        }
    } else {
        result = Arc::new("not a".to_string());
    }
    result.clone()
}
