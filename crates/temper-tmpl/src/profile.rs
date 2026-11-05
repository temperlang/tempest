//! Profiles: what a target handles natively (notes/tmpl/profiles.md).
//!
//! Only the features subset A's kinds need, so far; the either-or
//! choices (failure, function types, module init, module `var`s) come
//! with the passes that read them.

/// A set of features a target keeps natively.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Features(pub u32);

impl Features {
    pub const NONE: Features = Features(0);
    /// Statements inside expressions: `BlockValue`.
    pub const STATEMENT_EXPRESSIONS: Features = Features(1);
    pub const ALL: Features = Features(1);

    pub const fn with(self, other: Features) -> Features {
        Features(self.0 | other.0)
    }

    pub fn contains(self, other: Features) -> bool {
        self.0 & other.0 == other.0
    }
}

/// What a target handles natively, which lowering and validation read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub native: Features,
    /// How generator literals are written ([`crate::coroutines`]).
    pub coroutines: crate::coroutines::CoroutineStrategy,
}

impl Profile {
    /// Everything high-level: what starting tmpl may hold.
    pub fn full() -> Profile {
        Profile {
            native: Features::ALL,
            coroutines: crate::coroutines::CoroutineStrategy::TranslateToGenerator,
        }
    }
}
