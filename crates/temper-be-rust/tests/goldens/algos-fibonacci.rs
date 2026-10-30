use std::sync::OnceLock;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let mut zero: i32 = 0;
            zero = zero;
            println!("fib(1)={}", fib(zero.wrapping_add(1)));
            println!("fib(10)={}", fib(zero.wrapping_add(10)));
            println!("fib(20)={}", fib(zero.wrapping_add(20)));
            Ok(())
        })
        .clone()
}

pub(crate) fn fib(mut i: i32) -> i32 {
    let mut a: i32 = 0;
    let mut b: i32 = 1;
    while i > 0 {
        let c = a.wrapping_add(b);
        a = b;
        b = c;
        i = i.wrapping_sub(1);
    }
    a
}
