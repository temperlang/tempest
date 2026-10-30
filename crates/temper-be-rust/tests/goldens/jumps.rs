use std::sync::OnceLock;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            println!("{}", search(5));
            println!("{}", odds(9));
            Ok(())
        })
        .clone()
}

pub(crate) fn search(n: i32) -> i32 {
    let mut found: i32 = -1;
    let mut i: i32 = 0;
    'outer: while i < n {
        let mut j: i32 = 0;
        loop {
            if j >= n {
                break;
            }
            if i.wrapping_mul(j) == 6 {
                found = i.wrapping_mul(10).wrapping_add(j);
                break 'outer;
            }
            j = j.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    found
}

pub(crate) fn odds(n: i32) -> i32 {
    let mut k: i32 = 0;
    let mut sum: i32 = 0;
    loop {
        'body: {
            k = k.wrapping_add(1);
            if k & 1 == 0 {
                break 'body;
            }
            sum = sum.wrapping_add(k);
        }
        if k >= n {
            break;
        }
    }
    sum
}
