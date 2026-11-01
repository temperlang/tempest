use std::sync::{Arc, OnceLock, RwLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            println!("total {}", total(Arc::new(vec![1, 2, 3])));
            println!("forEach {}", total_for_each(Arc::new(vec![1, 2, 3])));
            println!("filter {}", total_filtered(Arc::new(vec![1, 2, 3])));
            println!(
                "nested {}",
                total_nested(Arc::new(vec![
                    Arc::new(vec![1, 2]),
                    Arc::new(vec![3]),
                    Arc::new(vec![]),
                ]))
            );
            println!(
                "firstOver {} {}",
                first_over(Arc::new(vec![1, 2, 3]), 1),
                first_over(Arc::new(vec![1, 2, 3]), 5)
            );
            Ok(())
        })
        .clone()
}

pub(crate) fn total(xs: impl temper_core::ToList<i32>) -> i32 {
    let xs = xs.to_list();
    let mut t: i32 = 0;
    {
        let length = temper_core::ListedTrait::len(&xs);
        let mut i: i32 = 0;
        while i < length {
            let x = temper_core::ListedTrait::get(&xs, i);
            i = i.wrapping_add(1);
            t = t.wrapping_add(x);
        }
    }
    t
}

pub(crate) fn total_for_each(xs: impl temper_core::ToList<i32>) -> i32 {
    let xs = xs.to_list();
    let mut t: i32 = 0;
    {
        let length = temper_core::ListedTrait::len(&xs);
        let mut i: i32 = 0;
        while i < length {
            let x = temper_core::ListedTrait::get(&xs, i);
            i = i.wrapping_add(1);
            t = t.wrapping_add(x);
        }
    }
    t
}

pub(crate) fn total_filtered(xs: impl temper_core::ToList<i32>) -> i32 {
    let xs = xs.to_list();
    let t: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
    temper_core::listed::filter(
        &xs,
        &(|x: i32| {
            *t.write().unwrap() = temper_core::read_locked(&t).wrapping_add(x);
            true
        }),
    );
    temper_core::read_locked(&t)
}

pub(crate) fn total_nested(xss: impl temper_core::ToList<temper_core::List<i32>>) -> i32 {
    let xss = xss.to_list();
    let mut t: i32 = 0;
    {
        let length = temper_core::ListedTrait::len(&xss);
        let mut i: i32 = 0;
        while i < length {
            let xs: temper_core::List<i32> = temper_core::ListedTrait::get(&xss, i);
            i = i.wrapping_add(1);
            {
                let length2 = temper_core::ListedTrait::len(&xs);
                let mut i2: i32 = 0;
                while i2 < length2 {
                    let x = temper_core::ListedTrait::get(&xs, i2);
                    i2 = i2.wrapping_add(1);
                    t = t.wrapping_add(x);
                }
            }
        }
    }
    t
}

pub(crate) fn first_over(xs: impl temper_core::ToList<i32>, n: i32) -> i32 {
    let xs = xs.to_list();
    {
        let length = temper_core::ListedTrait::len(&xs);
        let mut i: i32 = 0;
        while i < length {
            let x = temper_core::ListedTrait::get(&xs, i);
            i = i.wrapping_add(1);
            if x > n {
                return x;
            }
        }
    }
    -1
}
