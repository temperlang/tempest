use std::sync::{Arc, OnceLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            report(Some("hi"));
            report(None::<Arc<String>>);
            println!("{}", calc_rows(3, 6)?);
            println!("{}", count_below("abc").unwrap_or(-1));
            branch(0, 2);
            branch(2, 0);
            Ok(())
        })
        .clone()
}

pub(crate) fn more(x: Option<i32>) -> temper_core::Result<i32> {
    Err(temper_core::Error::new())
}

pub(crate) fn something(x: Option<impl ToArcString>) -> temper_core::Result<Arc<String>> {
    let x = x.map(|x| x.to_arc_string());
    if let Some(x) = &x {
        Ok(x.clone())
    } else {
        Err(temper_core::Error::new())
    }
}

pub(crate) fn report(x: Option<impl ToArcString>) {
    let x = x.map(|x| x.to_arc_string());
    let message = something(x.clone()).unwrap_or_else(|_| Arc::new("null".to_string()));
    let suffix = (|| {
        Ok::<_, temper_core::Error>(Arc::new(
            more(Some(
                (|| {
                    x.clone().ok_or_else(temper_core::Error::new)?;
                    Ok::<_, temper_core::Error>(1)
                })()
                .unwrap_or(0),
            ))?
            .to_string(),
        ))
    })()
    .unwrap_or_else(|_| Arc::new(String::new()));
    println!("{message}{suffix}");
}

pub(crate) fn calc_rows(cols: i32, total: i32) -> temper_core::Result<i32> {
    println!("calcRows({cols}, {total})");
    if temper_core::int_rem(total, cols)? != 0 {
        return Err(temper_core::Error::new());
    }
    temper_core::int_div(total, cols)
}

pub(crate) fn count_below(s: impl ToArcString) -> temper_core::Result<i32> {
    let s = s.to_arc_string();
    let mut i: i32 = 0;
    while i < temper_core::string::to_int(&s, None)? {
        i = i.wrapping_add(1);
    }
    Ok(i)
}

pub(crate) fn branch(a: i32, b: i32) {
    let c = if a == 0 {
        if b == 0 {
            Arc::new("00".to_string())
        } else if b == 1 {
            Arc::new("01".to_string())
        } else if b == 2 {
            Arc::new("02".to_string())
        } else {
            Arc::new("0x".to_string())
        }
    } else if a == 1 {
        Arc::new("1x".to_string())
    } else if a == 2 {
        Arc::new("2x".to_string())
    } else if a == 3 {
        Arc::new("3x".to_string())
    } else {
        Arc::new("xx".to_string())
    };
    println!("branch({a}, {b}) -> {c}");
}
