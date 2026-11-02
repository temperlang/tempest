use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let ls: temper_core::List<i32> = Arc::new(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
            println!(
                "All  {}",
                temper_core::listed::join(
                    &ls,
                    Arc::new(", ".to_string()),
                    &(|x: i32| Arc::new(x.to_string())),
                )
            );
            println!(
                "Even {}",
                temper_core::listed::join(
                    &temper_core::listed::filter(&ls, &(|x: i32| x & 1 == 0)),
                    Arc::new(", ".to_string()),
                    &(|x: i32| Arc::new(x.to_string())),
                )
            );
            println!(
                "Odd  {}",
                temper_core::listed::join(
                    &temper_core::listed::filter(&ls, &(|x: i32| x & 1 != 0)),
                    Arc::new(", ".to_string()),
                    &(|x: i32| Arc::new(x.to_string())),
                )
            );
            println!(
                "Neg  {}",
                temper_core::listed::join(
                    &temper_core::listed::map(&ls, &(|x: i32| x.wrapping_neg())),
                    Arc::new(", ".to_string()),
                    &(|x: i32| Arc::new(x.to_string())),
                )
            );
            let strs: temper_core::List<Arc<String>> =
                temper_core::string::split("1 ; 2 ; 3", Arc::new(" ; ".to_string()));
            println!(
                "Splt {}",
                temper_core::listed::join(
                    &strs,
                    Arc::new(", ".to_string()),
                    &(|x: Arc<String>| x.clone()),
                )
            );
            Ok(())
        })
        .clone()
}
