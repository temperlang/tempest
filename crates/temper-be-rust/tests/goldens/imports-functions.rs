use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let mut a: f64 = 3.0;
            a = a;
            let b = a + 1.0;
            let values = temper_core::listed::join(
                &Arc::new(vec![a, b, norm(a, b)]),
                Arc::new(", ".to_string()),
                &(|it: f64| temper_core::float64::to_string(it)),
            );
            println!("{values} triangle");
            println!(
                "{}",
                temper_core::float64::to_string(something(Arc::new(vec![a, b])))
            );
            Ok(())
        })
        .clone()
}

pub fn norm(x: f64, y: f64) -> f64 {
    crate::exporter::norm_squared(x, y).sqrt()
}

pub fn something(vals: impl temper_core::ToListed<f64>) -> f64 {
    let vals = vals.to_listed();
    temper_core::listed::reduce_from(&*vals, 0.0, &crate::exporter::norm_squared)
}
