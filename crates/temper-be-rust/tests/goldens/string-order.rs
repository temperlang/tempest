use std::sync::{Arc, OnceLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            println!("{}", show("｡", "😀"));
            println!("{}", show("😀", "｡"));
            println!("{}", show("", "𐀀"));
            println!("{}", show("￿", "􏿿"));
            println!("{}", show("a｡b", "a😀"));
            println!("{}", show("😀", "😁"));
            println!("{}", show("😀", "😀"));
            println!("{}", show("😀", "😀a"));
            println!("{}", show("", "😀"));
            println!("{}", show("", ""));
            println!("{}", show("z", "a"));
            println!("{}", show("abc", "abd"));
            println!("{} {} {}", cmp(2.5, 1.0), cmp(1.0, 2.5), cmp(1.5, 1.5));
            Ok(())
        })
        .clone()
}

pub(crate) fn show(a: impl ToArcString, b: impl ToArcString) -> Arc<String> {
    let a = a.to_arc_string();
    let b = b.to_arc_string();
    Arc::new(format!(
        "{} {} {} {} {}",
        a.as_str().cmp(b.as_str()) as i32,
        a < b,
        a <= b,
        a > b,
        a >= b
    ))
}

pub(crate) fn cmp(a: f64, b: f64) -> i32 {
    temper_core::float64::cmp(a, b)
}
