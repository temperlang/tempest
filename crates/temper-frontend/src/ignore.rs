//! `.gitignore` files, which say what finding libraries skips
//! ([`project`](crate::project)), read as git reads them
//! (git-scm.com/docs/gitignore, "Pattern format"): blank lines and `#`
//! comments are nothing; `!` re-includes what an earlier pattern
//! excluded; a trailing `/` matches only directories; a `/` at the start
//! or in the middle anchors a pattern to its file's directory, where
//! otherwise it matches a name at any depth; `*`, `?`, and `[a-z]` match
//! within a name, and `**` across directories; `\` escapes. The last
//! pattern that matches decides, a deeper file's after a shallower one's.
//! As in git, nothing under an excluded directory is looked at, so a
//! `!` can't re-include it.

/// One pattern of an ignore file, seen from where the scan started.
#[derive(Clone, Debug)]
struct Rule {
    /// Its text, without its `!`, anchoring `/`, or trailing `/`.
    glob: Vec<u8>,
    negated: bool,
    dir_only: bool,
    anchored: bool,
}

/// An ignore file's rules, with how to reach its directory from the
/// scan's paths: the scan's paths start with `inside` (a directory
/// below the scan's start, or nothing), which goes, and `prefix` (the
/// path from the file's directory to the start, for one above it) goes
/// before what's left.
#[derive(Clone, Debug)]
pub struct IgnoreFile {
    inside: String,
    prefix: String,
    rules: Vec<Rule>,
}

impl IgnoreFile {
    /// An ignore file's text, for a file whose directory is `inside`,
    /// as the scan names it, or `""` for the start, or above it by
    /// `prefix` (as `"lib/src"` for a start two levels below it).
    pub fn parse(text: &str, inside: &str, prefix: &str) -> IgnoreFile {
        let rules = text.lines().filter_map(rule).collect();
        IgnoreFile {
            inside: inside.trim_end_matches('/').to_string(),
            prefix: prefix.trim_matches('/').to_string(),
            rules,
        }
    }

    /// Whether its patterns say to skip `path`, as the scan names it, or
    /// `None` if none matches.
    pub fn decides(&self, path: &str, is_dir: bool) -> Option<bool> {
        let rest = match self.inside.as_str() {
            "" => path,
            inside => path.strip_prefix(inside)?.strip_prefix('/')?,
        };
        let full = match self.prefix.as_str() {
            "" => rest.to_string(),
            prefix => format!("{prefix}/{rest}"),
        };
        let name = full.rsplit('/').next().unwrap_or(&full);
        (self.rules.iter().rev())
            .find(|r| {
                (is_dir || !r.dir_only)
                    && match r.anchored {
                        true => glob(&r.glob, full.as_bytes()),
                        false => glob(&r.glob, name.as_bytes()),
                    }
            })
            .map(|r| !r.negated)
    }
}

/// Whether the ignore files, outermost first, say to skip `path`.
pub fn ignored(files: &[IgnoreFile], path: &str, is_dir: bool) -> bool {
    (files.iter().rev())
        .find_map(|f| f.decides(path, is_dir))
        .unwrap_or(false)
}

/// A line's rule, if it has one.
fn rule(line: &str) -> Option<Rule> {
    // Trailing spaces go, unless escaped.
    let mut line = line.trim_end_matches(['\r', '\n']);
    while line.ends_with(' ') && !line.ends_with("\\ ") {
        line = &line[..line.len() - 1];
    }
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (negated, line) = match line.strip_prefix('!') {
        Some(rest) => (true, rest),
        None => (false, line),
    };
    let (dir_only, line) = match line.strip_suffix('/') {
        Some(rest) => (true, rest),
        None => (false, line),
    };
    // A `/` anywhere left anchors it; one at the start goes.
    let anchored = line.contains('/');
    let line = line.strip_prefix('/').unwrap_or(line);
    (!line.is_empty()).then(|| Rule {
        glob: line.as_bytes().to_vec(),
        negated,
        dir_only,
        anchored,
    })
}

/// Whether a gitignore glob matches all of `s`: `*` and `?` within a
/// name, `[…]` one of a set (`!` or `^` first for the rest), `**/`,
/// `/**/`, and `/**` across directories, and `\` escaping.
fn glob(p: &[u8], s: &[u8]) -> bool {
    match p {
        [] => s.is_empty(),
        // `**/` at the start or after a `/`: any number of directories.
        [b'*', b'*', b'/', rest @ ..] => {
            (0..=s.len()).any(|i| (i == 0 || s[i - 1] == b'/') && glob(rest, &s[i..]))
        }
        // A trailing `**`: everything.
        [b'*', b'*'] => true,
        [b'*', rest @ ..] => (0..=s.len())
            .take_while(|&i| i == 0 || s[i - 1] != b'/')
            .any(|i| glob(rest, &s[i..])),
        [b'?', rest @ ..] => matches!(s, [c, tail @ ..] if *c != b'/' && glob(rest, tail)),
        [b'[', rest @ ..] => match (class(rest), s) {
            (Some((set, after)), [c, tail @ ..]) if *c != b'/' && set(*c) => glob(after, tail),
            (Some(_), _) => false,
            // An unclosed `[` is itself.
            (None, [b'[', tail @ ..]) => glob(rest, tail),
            (None, _) => false,
        },
        [b'\\', c, rest @ ..] => matches!(s, [d, tail @ ..] if d == c && glob(rest, tail)),
        [c, rest @ ..] => matches!(s, [d, tail @ ..] if d == c && glob(rest, tail)),
    }
}

/// A `[…]` set, its `[` gone: what it matches, and the pattern after
/// its `]`, or `None` if it isn't closed.
fn class(p: &[u8]) -> Option<(impl Fn(u8) -> bool + '_, &[u8])> {
    let (negated, body) = match p {
        [b'!' | b'^', rest @ ..] => (true, rest),
        _ => (false, p),
    };
    // A `]` first is one of the set.
    let close = 1 + body.get(1..)?.iter().position(|&c| c == b']')?;
    let (set, after) = (&body[..close], &body[close + 1..]);
    let has = move |c: u8| {
        let mut i = 0;
        while i < set.len() {
            if i + 2 < set.len() && set[i + 1] == b'-' {
                if (set[i]..=set[i + 2]).contains(&c) {
                    return true;
                }
                i += 3;
            } else {
                if set[i] == c {
                    return true;
                }
                i += 1;
            }
        }
        false
    };
    Some((move |c| has(c) != negated, after))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skips(text: &str, path: &str, is_dir: bool) -> bool {
        ignored(&[IgnoreFile::parse(text, "", "")], path, is_dir)
    }

    #[test]
    fn patterns_match_as_git_reads_them() {
        // A directory pattern, at any depth, only for directories.
        assert!(skips("build/\n", "build", true));
        assert!(skips("build/\n", "bench/build", true));
        assert!(!skips("build/\n", "build", false));
        // Anchored to the file's directory.
        assert!(skips("/out\n", "out", true));
        assert!(!skips("/out\n", "a/out", true));
        assert!(skips("a/b\n", "a/b", false));
        assert!(!skips("a/b\n", "x/a/b", false));
        // Globs, within a name.
        assert!(skips("*.bak\n", "deep/x.bak", false));
        assert!(!skips("a*\n", "b/c", true));
        assert!(skips("/data/unicode-*/\n", "data/unicode-16", true));
        assert!(skips("x?z\n", "xyz", false));
        assert!(skips("[a-c]1\n", "b1", false));
        assert!(!skips("[!a-c]1\n", "b1", false));
        assert!(skips("**/gen\n", "a/b/gen", true));
        assert!(skips("a/**/z\n", "a/z", false));
        assert!(skips("a/**/z\n", "a/b/c/z", false));
        assert!(skips("a/**\n", "a/b", false));
        // Comments, blanks, escapes, and the last match deciding.
        assert!(!skips("# build\n\n", "build", true));
        assert!(skips("\\#x\n", "#x", false));
        assert!(!skips("*.md\n!keep.md\n", "keep.md", false));
        assert!(skips("!keep.md\n*.md\n", "keep.md", false));
    }

    #[test]
    fn deeper_files_decide_after_shallower() {
        let files = [
            IgnoreFile::parse("gen/\n", "", ""),
            IgnoreFile::parse("!gen/\n", "lib", ""),
        ];
        assert!(ignored(&files, "gen", true));
        assert!(!ignored(&files, "lib/gen", true));
        // Above the start: its paths have the start's path before them.
        let above = [IgnoreFile::parse("/repo/build/\n", "", "repo")];
        assert!(ignored(&above, "build", true));
        assert!(!ignored(&above, "sub/build", true));
    }
}
