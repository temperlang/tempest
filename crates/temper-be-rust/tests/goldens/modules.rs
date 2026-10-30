use std::sync::OnceLock;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let calls2 = crate::helpers::calls();
            BASE2.set(1).unwrap_or_else(|_| panic!());
            println!("{}", crate::helpers::deep::greet("modules"));
            println!("sum {}", sum(3));
            crate::helpers::bump();
            crate::helpers::bump();
            println!("calls {calls2}");
            println!("calls now {}", crate::helpers::bump());
            Ok(())
        })
        .clone()
}

static BASE2: OnceLock<i32> = OnceLock::new();

pub(crate) fn base2() -> i32 {
    *BASE2.get().unwrap()
}

pub fn sum(n: i32) -> i32 {
    crate::helpers::twice(n)
        .wrapping_add(crate::helpers::base())
        .wrapping_add(base2())
}
