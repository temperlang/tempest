//! Ids into a program's tables, and source spans.

macro_rules! id {
    ($($(#[doc = $doc:literal])* $name:ident;)*) => {$(
        $(#[doc = $doc])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u32);

        impl $name {
            /// No entry.
            pub const NONE: Self = Self(u32::MAX);

            pub fn index(self) -> usize {
                self.0 as usize
            }

            pub fn is_none(self) -> bool {
                self == Self::NONE
            }
        }
    )*};
}

id! {
    /// A node in [`Nodes`](crate::Nodes). A child's id is always lower
    /// than its parent's.
    NodeId;
    /// A declaration in [`DeclTable`](crate::DeclTable).
    DeclId;
    /// An interned string in [`Strings`](crate::Strings).
    StrId;
    /// A type in [`TypeTable`](crate::TypeTable).
    TypeId;
    /// A source file in [`Program::files`](crate::Program::files).
    FileId;
}

/// Where something came from: a file and a byte range, end exclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    /// No position: for programs read from text without spans.
    pub const UNKNOWN: Span = Span {
        file: FileId::NONE,
        start: 0,
        end: 0,
    };

    pub fn is_known(self) -> bool {
        !self.file.is_none()
    }
}

impl Default for Span {
    fn default() -> Self {
        Span::UNKNOWN
    }
}

/// A source file that spans point into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    /// Relative to the library's root.
    pub path: String,
    /// Of the text, to match it later.
    pub hash: [u8; 16],
}
