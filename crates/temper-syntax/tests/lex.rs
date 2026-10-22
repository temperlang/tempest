//! Lexer tests over the cases in `tests/*.cases`.
//!
//! Each case is a header line `=== name`, or `=== name (markdown)` for a
//! `.temper.md` source, then the input text exactly, then a `--- tokens` line
//! and the expected tokens, one per line in the format of
//! `lex::write_token_lines`. The input ends with the newline before
//! `--- tokens`, so an input that ends in a newline has a blank line there.
//!
//! - `cases_lex_as_expected` checks each case's tokens. To accept new or
//!   changed output, run with `UPDATE_SNAPSHOTS=1` and review the diff.
//! - `mutated_cases_keep_lexer_properties` lexes randomly edited copies of
//!   the cases and checks properties that hold for any text. Set
//!   `LEX_FUZZ_SEED` and `LEX_FUZZ_ROUNDS` (edits per case) to vary it.
//!
//! `compat/check_lex.py` checks the same cases against Kotlin Temper, and
//! `compat/fuzz_lex.py` compares edited copies against it.

mod common;

use common::{Rng, case_files, check_cases, env_number, mutate};
use temper_syntax::lex::{SourceKind, Synthetic, Token, TokenKind, lex, write_token_lines};

#[test]
fn cases_lex_as_expected() {
    check_cases("tests", "tokens", |case| {
        let mut actual = String::new();
        write_token_lines(&case.input, &lex(&case.input, case.kind), &mut actual);
        actual
    });
}

#[test]
fn mutated_cases_keep_lexer_properties() {
    let seed = env_number("LEX_FUZZ_SEED", 1);
    let rounds = env_number("LEX_FUZZ_ROUNDS", 40);
    let mut rng = Rng(seed.max(1));
    for (path, cases) in case_files("tests", "tokens") {
        for case in cases {
            let context = format!("{} case {:?}", path.display(), case.name);
            check_both(&case.input, &context);
            for round in 0..rounds {
                let text = mutate(&case.input, common::PIECES, &mut rng);
                check_both(&text, &format!("{context}, seed {seed} round {round}"));
            }
        }
    }
}

// Properties.

/// Checks both source kinds, since any text is valid input to either.
fn check_both(text: &str, context: &str) {
    for kind in [SourceKind::Standalone, SourceKind::Markdown] {
        let tokens = lex(text, kind);
        if let Err(problem) = check_properties(text, &tokens) {
            panic!("{context} as {kind:?}: {problem}\ninput: {text:?}");
        }
        assert!(lex(text, kind) == tokens, "{context}: lexing again differs");
    }
}

/// Tokens from the source tile it exactly, in order, none empty. Synthetic
/// tokens are zero-width at the current position, except paragraph
/// comments, which lie within the prose comment before them.
fn check_properties(text: &str, tokens: &[Token]) -> Result<(), String> {
    let mut pos = 0;
    let mut last_source: Option<Token> = None;
    for (i, &token) in tokens.iter().enumerate() {
        let at = || format!("token {i} {token:?}");
        if token.start > token.end || token.end as usize > text.len() {
            return Err(format!("{} is out of bounds", at()));
        }
        if token.synthetic.is_some() {
            if token.start == token.end {
                if token.start != pos {
                    return Err(format!("{} isn't at position {pos}", at()));
                }
            } else {
                let inside = last_source.is_some_and(|prose| {
                    prose.kind == TokenKind::Comment
                        && prose.start <= token.start
                        && token.end <= prose.end
                });
                if token.synthetic != Some(Synthetic::Paragraph) || !inside {
                    return Err(format!("{} has width but isn't a paragraph comment", at()));
                }
            }
            continue;
        }
        if token.start != pos {
            return Err(format!("{} doesn't start at position {pos}", at()));
        }
        if token.start == token.end {
            return Err(format!("{} is empty but not synthetic", at()));
        }
        if text.get(token.range()).is_none() {
            return Err(format!("{} doesn't span whole characters", at()));
        }
        pos = token.end;
        last_source = Some(token);
    }
    if pos as usize != text.len() {
        return Err(format!("tokens end at {pos} of {}", text.len()));
    }
    Ok(())
}
