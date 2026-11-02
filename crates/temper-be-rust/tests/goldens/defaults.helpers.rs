use std::sync::{Arc, OnceLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE.get_or_init(|| Ok(())).clone()
}

pub fn wrap(
    text: impl ToArcString,
    open: Option<impl ToArcString>,
    close: Option<impl ToArcString>,
) -> Arc<String> {
    let text = text.to_arc_string();
    let open = open.map(|x| x.to_arc_string());
    let close = close.map(|x| x.to_arc_string());
    let open = open.unwrap_or_else(|| Arc::new("[".to_string()));
    let close = close.unwrap_or_else(|| open.clone());
    Arc::new(format!("{open}{text}{close}"))
}

struct PadStruct {
    left: i32,
    right: i32,
}

#[derive(Clone)]
pub struct Pad(Arc<PadStruct>);

impl Pad {
    pub fn new(left: Option<i32>, right: Option<i32>) -> Pad {
        let left = left.unwrap_or(1);
        let right = right.unwrap_or(2);
        Pad(Arc::new(PadStruct { left, right }))
    }

    pub fn apply(&self, text: impl ToArcString, fill: Option<impl ToArcString>) -> Arc<String> {
        let text = text.to_arc_string();
        let fill = fill.map(|x| x.to_arc_string());
        let fill = fill.unwrap_or_else(|| Arc::new(".".to_string()));
        let mut out = text.clone();
        {
            let mut i: i32 = 0;
            while i < self.0.left {
                out = Arc::new(format!("{fill}{out}"));
                i = i.wrapping_add(1);
            }
        }
        {
            let mut i: i32 = 0;
            while i < self.0.right {
                out = Arc::new(format!("{out}{fill}"));
                i = i.wrapping_add(1);
            }
        }
        out.clone()
    }

    pub fn left(&self) -> i32 {
        self.0.left
    }

    pub fn right(&self) -> i32 {
        self.0.right
    }
}

temper_core::impl_any_value_trait!(Pad, []);

pub mod builders {
    use super::*;

    #[derive(Clone)]
    pub struct PadBuilder {}

    #[derive(Clone)]
    pub struct PadOptions {
        left: Option<i32>,
        right: Option<i32>,
    }

    impl PadOptions {
        pub fn new(_selfish: PadBuilder) -> Self {
            Self {
                left: None,
                right: None,
            }
        }

        pub fn left(mut self, left: i32) -> Self {
            self.left = Some(left);
            self
        }

        pub fn right(mut self, right: i32) -> Self {
            self.right = Some(right);
            self
        }

        pub fn build(self) -> Pad {
            Pad::new(self.left, self.right)
        }
    }

    impl PadBuilder {
        pub fn build(self) -> Pad {
            self.options().build()
        }

        pub fn options(self) -> PadOptions {
            PadOptions::new(self)
        }
    }
}
