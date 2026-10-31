use std::sync::{Arc, OnceLock, RwLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let label = Label::new(None::<Arc<String>>);
            println!("{}", label.shown());
            *label.0.text.write().unwrap() = Some(Arc::new("hi".to_string()));
            println!("{}", label.shown());
            println!(
                "{} {} {}",
                same_float(Some(1.5), Some(1.5)),
                same_float(None, Some(0.0)),
                same_float(None, None)
            );
            println!(
                "{} {}",
                same_long(Some(2), Some(2)),
                same_long(Some(2), None)
            );
            println!(
                "{} {} {}",
                longer(Some("41")),
                longer(None::<Arc<String>>),
                longer(Some("x"))
            );
            println!(
                "{}",
                must(Some("ok")).unwrap_or_else(|_| Arc::new("bad".to_string()))
            );
            println!("{} {}", tried("7"), tried("x"));
            Ok(())
        })
        .clone()
}

struct LabelStruct {
    text: RwLock<Option<Arc<String>>>,
}

#[derive(Clone)]
pub struct Label(Arc<LabelStruct>);

impl Label {
    pub fn new(text: Option<impl ToArcString>) -> Label {
        let text = text.map(|x| x.to_arc_string());
        Label(Arc::new(LabelStruct {
            text: RwLock::new(text),
        }))
    }

    pub fn shown(&self) -> Arc<String> {
        let t = self.text();
        t.clone().unwrap_or_else(|| Arc::new("(none)".to_string()))
    }

    pub fn text(&self) -> Option<Arc<String>> {
        self.0.text.read().unwrap().clone()
    }

    pub fn set_text(&self, value: Option<Arc<String>>) {
        *self.0.text.write().unwrap() = value;
    }
}

temper_core::impl_any_value_trait!(Label, []);

pub(crate) fn same_float(a: Option<f64>, b: Option<f64>) -> bool {
    a.zip(b).map_or(a.is_none() && b.is_none(), |(a, b)| {
        temper_core::float64::cmp(a, b) == 0
    })
}

pub(crate) fn same_long(a: Option<i64>, b: Option<i64>) -> bool {
    a == b
}

pub(crate) fn must(s: Option<impl ToArcString>) -> temper_core::Result<Arc<String>> {
    let s = s.map(|x| x.to_arc_string());
    s.clone().ok_or_else(temper_core::Error::new)
}

pub(crate) fn longer(s: Option<impl ToArcString>) -> i32 {
    let s = s.map(|x| x.to_arc_string());
    (|| {
        let t = must(s.clone())?;
        let n = temper_core::string::to_int(&t, None)?;
        Ok::<_, temper_core::Error>(n.wrapping_add(1))
    })()
    .unwrap_or(0)
}

pub(crate) fn tried(s: impl ToArcString) -> i32 {
    let s = s.to_arc_string();
    {
        println!("trying {s}");
        temper_core::string::to_int(&s, None)
    }
    .unwrap_or(-1)
}
