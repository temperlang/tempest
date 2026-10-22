//! A wasm entry point for Temper, for a browser playground and for measuring
//! how big Temper is as wasm (`compat/measure_size.py`).
//!
//! For now it parses source and returns the parse tree and the grammar
//! step's tree as text. JavaScript calls
//! `alloc` for a buffer, writes UTF-8 source into it, calls `parse`, and reads
//! the result from `output_ptr` and `output_len`:
//!
//! ```js
//! const bytes = new TextEncoder().encode(source);
//! const ptr = exports.alloc(bytes.length);
//! new Uint8Array(exports.memory.buffer, ptr, bytes.length).set(bytes);
//! exports.parse(ptr, bytes.length, markdown ? 1 : 0);
//! const out = new Uint8Array(exports.memory.buffer, exports.output_ptr(), exports.output_len());
//! console.log(new TextDecoder().decode(out));
//! ```

use std::cell::RefCell;
use temper_syntax::ast::build;
use temper_syntax::{lex::SourceKind, parse::parse as parse_source, print::write_ast, print::write_tree};

thread_local! {
    static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUTPUT: RefCell<String> = const { RefCell::new(String::new()) };
}

/// A buffer of `len` bytes for the next input, replacing the last one.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    INPUT.with_borrow_mut(|input| {
        *input = vec![0; len];
        input.as_mut_ptr()
    })
}

/// Parses the input buffer as UTF-8 Temper source, or literate markdown if
/// `markdown` is nonzero, and keeps the parse tree and then the grammar
/// step's tree as text for `output_ptr`, with a blank line between.
/// Invalid UTF-8 is replaced rather than rejected.
#[unsafe(no_mangle)]
pub extern "C" fn parse(ptr: *const u8, len: usize, markdown: u32) {
    let kind = if markdown != 0 { SourceKind::Markdown } else { SourceKind::Standalone };
    INPUT.with_borrow(|input| {
        // Only our own buffer from `alloc` is accepted.
        let bytes = if ptr == input.as_ptr() && len <= input.len() { &input[..len] } else { &[][..] };
        let source = String::from_utf8_lossy(bytes);
        let parsed = parse_source(&source, kind);
        OUTPUT.with_borrow_mut(|out| {
            out.clear();
            write_tree(&parsed, 100, out);
            out.push('\n');
            write_ast(&build(&parsed), 100, out);
        });
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn output_ptr() -> *const u8 {
    OUTPUT.with_borrow(|out| out.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn output_len() -> usize {
    OUTPUT.with_borrow(|out| out.len())
}
