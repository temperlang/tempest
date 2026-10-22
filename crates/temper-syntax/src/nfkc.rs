//! Whether text is in Unicode NFKC, as Kotlin Temper requires of
//! identifiers (`LexicalDefinitions.isIdentifierNormal`): a word token
//! that isn't is an error, and so is a quoted name, ``nym`...` ``.
//!
//! This checks rather than normalizes, which needs much less data. Code
//! points that NFKC changes on their own never appear in NFKC text; with
//! none of those, NFKC is NFC, and NFC's quick check settles the rest
//! unless a mark that may compose with what precedes it appears. Only then
//! does it normalize, to NFC, and compare. The tables, in `nfkc_tables`,
//! come from the JDK, as Kotlin Temper's answers do; see
//! compat/gen_nfkc.py for regenerating them.
//!
//! Covered: all of Unicode as the JDK knows it ([`UNICODE_VERSION`]),
//! checked against the JDK on the cases in tests/nfkc.txt (and once on
//! 200,000 random ones). Like Kotlin, not covered: TR 31's NFKC
//! modifications for identifiers, and code points newer than the JDK's
//! Unicode, which count as unchanged. A newer JDK may answer differently;
//! regenerate the tables when Kotlin Temper's JDK moves.

use crate::id_parts;
use crate::nfkc_tables::{CCC_STARTS, CCC_VALUES, COMPOSITES, NOT_NFKC, SECONDS};

pub use crate::nfkc_tables::UNICODE_VERSION;

/// Whether `text` is unchanged by NFKC.
pub fn is_nfkc(text: &str) -> bool {
    if text.is_ascii() {
        return true;
    }
    let mut last_ccc = 0;
    let mut maybe = false;
    for c in text.chars() {
        if c.is_ascii() {
            last_ccc = 0;
            continue;
        }
        if id_parts::contains(NOT_NFKC, c) {
            return false;
        }
        let ccc = ccc(c);
        if ccc != 0 && last_ccc > ccc {
            return false;
        }
        maybe = maybe || may_compose_back(c);
        last_ccc = ccc;
    }
    !maybe || nfc(text).eq(text.chars())
}

/// The canonical combining class of `c`.
fn ccc(c: char) -> u8 {
    CCC_VALUES[CCC_STARTS.partition_point(|&start| start <= c as u32) - 1]
}

/// Whether `c` may compose with what precedes it: NFC's quick check
/// "maybe".
fn may_compose_back(c: char) -> bool {
    let c = c as u32;
    (V_BASE..V_BASE + V_COUNT).contains(&c)
        || (T_BASE + 1..T_BASE + T_COUNT).contains(&c)
        || SECONDS.binary_search(&c).is_ok()
}

/// `text` in NFC, for text without code points NFKC changes on their own:
/// decomposed, put in canonical order and composed again (UAX #15).
fn nfc(text: &str) -> impl Iterator<Item = char> {
    let mut chars = Vec::new();
    for c in text.chars() {
        decompose(c as u32, &mut chars);
    }
    // Canonical order: a stable sort of each run of marks by class.
    let mut start = 0;
    for i in 0..=chars.len() {
        if i == chars.len() || ccc_of(chars[i]) == 0 {
            chars[start..i].sort_by_key(|&c| ccc_of(c));
            start = i + 1;
        }
    }
    // Compose each mark with the last starter if nothing between blocks it.
    let mut out: Vec<u32> = Vec::with_capacity(chars.len());
    let mut starter: Option<usize> = None;
    let mut last_ccc: Option<u8> = None;
    for c in chars {
        let ccc = ccc_of(c);
        if let Some(s) = starter {
            let blocked = last_ccc.is_some_and(|last| last == 0 || last >= ccc);
            if !blocked {
                if let Some(composite) = compose(out[s], c) {
                    out[s] = composite;
                    continue;
                }
            }
        }
        if ccc == 0 {
            starter = Some(out.len());
            last_ccc = None;
        } else {
            last_ccc = Some(ccc);
        }
        out.push(c);
    }
    out.into_iter().filter_map(char::from_u32)
}

fn ccc_of(c: u32) -> u8 {
    char::from_u32(c).map_or(0, ccc)
}

/// Appends the canonical decomposition of `c`.
fn decompose(c: u32, out: &mut Vec<u32>) {
    if (S_BASE..S_BASE + S_COUNT).contains(&c) {
        let index = c - S_BASE;
        out.push(L_BASE + index / (V_COUNT * T_COUNT));
        out.push(V_BASE + index % (V_COUNT * T_COUNT) / T_COUNT);
        if index % T_COUNT != 0 {
            out.push(T_BASE + index % T_COUNT);
        }
        return;
    }
    match COMPOSITES.binary_search_by_key(&c, |&[composite, _, _]| composite) {
        Ok(i) => {
            let [_, first, second] = COMPOSITES[i];
            decompose(first, out);
            out.push(second);
        }
        Err(_) => out.push(c),
    }
}

/// The primary composite of a pair, if any.
fn compose(first: u32, second: u32) -> Option<u32> {
    if (L_BASE..L_BASE + L_COUNT).contains(&first) && (V_BASE..V_BASE + V_COUNT).contains(&second) {
        return Some(S_BASE + ((first - L_BASE) * V_COUNT + second - V_BASE) * T_COUNT);
    }
    let lv = (S_BASE..S_BASE + S_COUNT).contains(&first) && (first - S_BASE) % T_COUNT == 0;
    if lv && (T_BASE + 1..T_BASE + T_COUNT).contains(&second) {
        return Some(first + second - T_BASE);
    }
    COMPOSITES.iter().find(|&&[_, f, s]| f == first && s == second).map(|&[composite, _, _]| composite)
}

// Hangul syllables compose by arithmetic (Unicode's chapter 3.12).
const S_BASE: u32 = 0xAC00;
const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const L_COUNT: u32 = 19;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;
const S_COUNT: u32 = L_COUNT * V_COUNT * T_COUNT;

#[cfg(test)]
mod tests {
    use super::is_nfkc;

    #[test]
    fn examples() {
        assert!(is_nfkc("abc"));
        assert!(is_nfkc("caf\u{e9}"));
        assert!(!is_nfkc("cafe\u{301}"));
        // Unicode's FAQ example: four spellings of one identifier, of which
        // only one is in NFKC.
        assert!(!is_nfkc("\u{3d3}"));
        assert!(!is_nfkc("\u{3d2}\u{301}"));
        assert!(is_nfkc("\u{38e}"));
        assert!(!is_nfkc("\u{3a5}\u{301}"));
        // Hangul: a syllable, and the jamo it's made of.
        assert!(is_nfkc("\u{d55c}"));
        assert!(!is_nfkc("\u{1112}\u{1161}\u{11ab}"));
        assert!(!is_nfkc("\u{fb01}"));
    }
}
