use std::sync::OnceLock;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            println!("Checkpoint 0");
            'orelse: {
                let _a: i32;
                break 'orelse;
            }
            yay();
            println!("Checkpoint 1");
            {
                let _a2: i32 = 1;
                yay();
            };
            println!("Checkpoint 2");
            {
                let a3: i32 = 1;
                meh();
                if a3 == 1 {
                    meh();
                } else {
                    meh();
                }
            }
            println!("Checkpoint 3");
            {
                let mut _a4: i32 = 1;
                _a4 = 2;
                yay()
            };
            println!("Checkpoint 4");
            if (|| {
                f(0)?;
                yay();
                Ok::<_, temper_core::Error>(())
            })()
            .is_err()
            {
                boo();
            }
            meh();
            println!("Checkpoint 5");
            if (|| {
                g(0)?;
                yay();
                Ok::<_, temper_core::Error>(())
            })()
            .is_err()
            {
                boo();
            }
            meh();
            println!("Checkpoint 6");
            if (|| {
                i(0)?;
                yay();
                Ok::<_, temper_core::Error>(())
            })()
            .is_err()
            {
                boo();
            }
            meh();
            Ok(())
        })
        .clone()
}

pub(crate) fn yay() {
    println!("yay");
}

pub(crate) fn meh() {
    println!("meh");
}

pub(crate) fn boo() {
    println!("boo");
}

pub(crate) fn f(x: i32) -> temper_core::Result<i32> {
    Ok(x)
}

pub(crate) fn g(x: i32) -> temper_core::Result<i32> {
    Ok(x)
}

pub(crate) fn i(x: i32) -> temper_core::Result<i32> {
    Ok(x)
}
