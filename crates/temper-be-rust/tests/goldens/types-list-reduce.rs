use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let vals: temper_core::List<i32> = Arc::new(vec![2, 3, 4]);
            let sum = temper_core::listed::reduce(&vals, &(|sum: i32, n: i32| sum.wrapping_add(n)));
            let poor_reverse_join = temper_core::listed::reduce_from(
                &vals,
                Arc::new(String::new()),
                &(|text: Arc<String>, n: i32| Arc::new(format!("{n}{text}"))),
            );
            println!("Reductions: {sum} {poor_reverse_join}");
            println!(
                "All even: {}",
                all(vals.clone(), Arc::new(move |n: i32| n.wrapping_rem(2) == 0))
            );
            println!(
                "All positive: {}",
                all(vals.clone(), Arc::new(move |n: i32| n > 0))
            );
            Ok(())
        })
        .clone()
}

pub(crate) fn all<T: Clone + Send + Sync + 'static>(
    items: impl temper_core::ToListed<T>,
    check: Arc<dyn Fn(T) -> bool + Send + Sync>,
) -> bool {
    let items = items.to_listed();
    temper_core::listed::reduce_from(
        &*items,
        true,
        &(|result: bool, it: T| {
            if result {
                check(it.clone())
            } else {
                false
            }
        }),
    )
}
