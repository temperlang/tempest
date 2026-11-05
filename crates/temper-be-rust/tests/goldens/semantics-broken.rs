use std::sync::{Arc, OnceLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| Err(temper_core::Error::new()))
        .clone()
}

pub(crate) fn undefined_plus_arg_inside() -> i32 {
    panic!()
}

pub(crate) fn pad_left(
    string: impl ToArcString,
    min_size: i32,
    pad: impl ToArcString,
) -> Arc<String> {
    let string = string.to_arc_string();
    let pad = pad.to_arc_string();
    let needed = min_size.wrapping_sub(temper_core::string::count_between(
        &string,
        0usize,
        string.len(),
    ));
    if needed <= 0 {
        return string.clone();
    }
    panic!();
}
