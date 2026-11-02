use std::sync::{Arc, OnceLock, RwLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            *F_CALLS.write().unwrap() = Some(0);
            println!("fCalls={}", f_calls());
            println!("g(   )={}", g(None));
            println!("fCalls={}", f_calls());
            println!("g(123)={}", g(Some(123)));
            println!("fCalls={}", f_calls());
            println!("g(null)={}", g(no_inline()));
            println!("fCalls={}", f_calls());
            Thing::new().say(Some("hi"), None::<Arc<String>>);
            Thing::new().say(None::<Arc<String>>, Some("there"));
            Thing::new().say(Some("by"), Some("e"));
            Thing::say_static("yall", None::<Arc<String>>);
            *INKED.write().unwrap() = Some(0);
            println!("maybeInc() -> {}", {
                let subject = maybe_inc();
                subject.unwrap_or(-1)
            });
            println!("inked: {}", inked());
            println!("lengthish \"ab\": {}", chain_more(Some("ab")));
            println!("lengthish null: {}", chain_more(None::<Arc<String>>));
            println!("inked: {}", inked());
            println!(
                "maybeEmpty \"\": {}",
                maybe_empty(Some(StringHolder::new("")))
            );
            println!("maybeEmpty null: {}", maybe_empty(None));
            println!("inked: {}", inked());
            Ok(())
        })
        .clone()
}

static F_CALLS: RwLock<Option<i32>> = RwLock::new(None);

pub(crate) fn f_calls() -> i32 {
    crate::support::initialized(|| *F_CALLS.read().unwrap())
}

pub(crate) fn f() -> i32 {
    crate::support::initialized(|| {
        let mut f_calls = F_CALLS.write().unwrap();
        *f_calls = Some((*f_calls)?.wrapping_add(1));
        Some(())
    });
    42
}

pub(crate) fn g(x: Option<i32>) -> i32 {
    x.unwrap_or_else(f)
}

pub(crate) fn no_inline() -> Option<i32> {
    println!("noInline");
    None
}

struct ThingStruct {}

#[derive(Clone)]
pub(crate) struct Thing(Arc<ThingStruct>);

impl Thing {
    pub fn new() -> Thing {
        Thing(Arc::new(ThingStruct {}))
    }

    pub fn say(&self, a: Option<impl ToArcString>, b: Option<impl ToArcString>) {
        let a = a.map(|x| x.to_arc_string());
        let b = b.map(|x| x.to_arc_string());
        let a = a.unwrap_or_else(|| Arc::new(String::new()));
        let b = b.unwrap_or_else(|| Arc::new(String::new()));
        println!("saying {a}{b}");
    }

    pub fn say_static(a: impl ToArcString, b: Option<impl ToArcString>) {
        let a = a.to_arc_string();
        let b = b.map(|x| x.to_arc_string());
        let b = b.unwrap_or_else(|| Arc::new(String::new()));
        println!("static {a}{b}");
    }
}

temper_core::impl_any_value_trait!(Thing, []);

static INKED: RwLock<Option<i32>> = RwLock::new(None);

pub(crate) fn inked() -> i32 {
    crate::support::initialized(|| *INKED.read().unwrap())
}

struct StringHolderStruct {
    string: Arc<String>,
}

#[derive(Clone)]
pub(crate) struct StringHolder(Arc<StringHolderStruct>);

impl StringHolder {
    pub fn new(string: impl ToArcString) -> StringHolder {
        let string = string.to_arc_string();
        StringHolder(Arc::new(StringHolderStruct { string }))
    }
}

temper_core::impl_any_value_trait!(StringHolder, []);

pub(crate) fn maybe_empty(a: Option<StringHolder>) -> Arc<String> {
    crate::support::initialized(|| {
        let mut inked = INKED.write().unwrap();
        *inked = Some((*inked)?.wrapping_add(1));
        Some(())
    });
    let subject = {
        let subject = {
            let subject = a.as_ref().map(|a| a.0.string.clone());
            subject.as_ref().map(|subject| subject.is_empty())
        };
        subject.map(|subject| Arc::new(subject.to_string()))
    };
    subject
        .clone()
        .unwrap_or_else(|| Arc::new("null".to_string()))
}

pub(crate) fn maybe_length(a: Option<impl ToArcString>) -> Option<i32> {
    let a = a.map(|x| x.to_arc_string());
    crate::support::initialized(|| {
        let mut inked = INKED.write().unwrap();
        *inked = Some((*inked)?.wrapping_add(1));
        Some(())
    });
    a.as_ref()
        .map(|a| temper_core::string::count_between(a, 0usize, a.len()))
}

pub(crate) fn maybe_inc() -> Option<i32> {
    if inked() == 10 {
        None
    } else {
        crate::support::initialized(|| {
            let mut inked = INKED.write().unwrap();
            *inked = Some((*inked)?.wrapping_add(1));
            Some(())
        });
        Some(inked())
    }
}

pub(crate) fn chain_more(a: Option<impl ToArcString>) -> Arc<String> {
    let a = a.map(|x| x.to_arc_string());
    Arc::new(
        {
            let subject = {
                let subject = maybe_length(a.clone());
                subject.map(|subject| subject.max(0))
            };
            subject.unwrap_or(-1)
        }
        .to_string(),
    )
}
