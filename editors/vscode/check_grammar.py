# /// script
# requires-python = ">=3.11"
# ///
"""Checks the TextMate grammars in syntaxes/, with no packages.

    uv run editors/vscode/check_grammar.py

Nothing here can run VS Code's TextMate engine (vscode-textmate with
Oniguruma) offline, so this checks the grammars three ways:

1. **Structure:** each grammar is JSON with a scopeName, every rule is a
   `match`, `begin` with `end` or `while`, or `include`, every `#name`
   include is in the repository, and every regex compiles. Regexes keep to
   what Oniguruma and Python's `re` read alike: no `\\G`, `\\h`, `\\p{…}`,
   atomic groups, or possessive quantifiers.
2. **Scopes:** a small TextMate tokenizer (begin/end with backreferences,
   begin/while, captures, includes) scopes samples of Temper and of
   literate Temper, and each sample's expected scopes are checked.
3. **Keywords** match the playground's highlighting
   (site/playground/src/temper-language.ts) and the words editor analysis
   won't rename to (crates/temper-analysis/src/references.rs).
"""

import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GRAMMARS = {p.name: json.loads(p.read_text()) for p in sorted((HERE / "syntaxes").glob("*.tmLanguage.json"))}
BY_SCOPE = {g["scopeName"]: g for g in GRAMMARS.values()}
# Grammars VS Code has that these include; the tokenizer reads them as empty.
EXTERNAL = {"text.html.markdown"}
UNPORTABLE = [r"\G", r"\h", r"\p{", "(?>", "*+", "++", "?+"]

problems: list[str] = []


def fail(message: str) -> None:
    problems.append(message)


def rules(grammar: dict):
    """Every rule in a grammar, with where it is."""
    todo = [("patterns", r) for r in grammar.get("patterns", [])]
    todo += [(f"repository.{k}", r) for k, r in grammar.get("repository", {}).items()]
    while todo:
        where, rule = todo.pop()
        yield where, rule
        todo += [(f"{where}.patterns", r) for r in rule.get("patterns", [])]


def check_structure() -> None:
    for name, grammar in GRAMMARS.items():
        if "scopeName" not in grammar:
            fail(f"{name}: no scopeName")
        repository = grammar.get("repository", {})
        for where, rule in rules(grammar):
            kinds = [k for k in ("match", "begin", "include") if k in rule]
            if len(kinds) > 1 or (not kinds and "patterns" not in rule):
                fail(f"{name} {where}: not one of match, begin, include, patterns")
            if "begin" in rule and ("end" in rule) == ("while" in rule):
                fail(f"{name} {where}: begin needs end or while")
            include = rule.get("include", "")
            if include.startswith("#") and include[1:] not in repository:
                fail(f"{name} {where}: no {include}")
            if include and not include.startswith(("#", "$")) and include not in BY_SCOPE | dict.fromkeys(EXTERNAL):
                fail(f"{name} {where}: unknown grammar {include}")
            for key in ("match", "begin", "end", "while"):
                if key not in rule:
                    continue
                source = rule[key]
                for bad in UNPORTABLE:
                    if bad in source:
                        fail(f"{name} {where}.{key}: {bad} isn't portable")
                try:
                    # End and while may refer to begin's groups.
                    re.compile(re.sub(r"\\(\d)", "x", source) if key in ("end", "while") else source)
                except re.error as e:
                    fail(f"{name} {where}.{key}: {e}")


class Tokenizer:
    """A small TextMate tokenizer: enough of vscode-textmate's rules for
    these grammars."""

    def __init__(self, scope: str):
        self.grammar = BY_SCOPE[scope]

    def patterns(self, rule_patterns: list, grammar: dict, seen: frozenset = frozenset()) -> list:
        """Patterns with includes expanded, each with its grammar."""
        out = []
        for p in rule_patterns:
            include = p.get("include")
            if include is None:
                if "match" in p or "begin" in p:
                    out.append((p, grammar))
                else:
                    out += self.patterns(p.get("patterns", []), grammar, seen)
                continue
            key = (id(grammar), include)
            if key in seen:
                continue
            if include.startswith("#"):
                target, g = grammar["repository"][include[1:]], grammar
            elif include in ("$self", "$base"):
                target, g = {"patterns": grammar["patterns"]}, grammar
            elif include in BY_SCOPE:
                g = BY_SCOPE[include]
                target = {"patterns": g["patterns"]}
            else:
                continue
            if "match" in target or "begin" in target:
                out.append((target, g))
            else:
                out += self.patterns(target.get("patterns", []), g, seen | {key})
        return out

    def tokenize(self, text: str) -> list[tuple[str, list[str]]]:
        """Each token's text and scopes, line by line."""
        base = [self.grammar["scopeName"]]
        # The stack: each open begin rule, its grammar, its compiled end or
        # while, and the scopes inside it.
        stack: list[tuple[dict, dict, re.Pattern, list[str]]] = []
        tokens: list[tuple[str, list[str]]] = []

        def scopes() -> list[str]:
            return stack[-1][3] if stack else base

        for line in text.split("\n"):
            # `while` rules hold only while their line starts match.
            pos = 0
            for depth, (rule, _, pattern, _) in enumerate(stack):
                if "while" in rule:
                    m = pattern.match(line, pos)
                    if not m:
                        del stack[depth:]
                        break
                    if m.end() > pos:
                        tokens.append((line[pos : m.end()], list(stack[depth][3])))
                    pos = m.end()
            while pos <= len(line):
                rule, grammar = (stack[-1][0], stack[-1][1]) if stack else (self.grammar, self.grammar)
                candidates = self.patterns(rule.get("patterns", []), grammar)
                best = None
                if stack and "end" in rule:
                    m = stack[-1][2].search(line, pos)
                    if m:
                        best = (m.start(), 0, None, m)
                for i, (p, g) in enumerate(candidates):
                    m = re.compile(p.get("match") or p["begin"]).search(line, pos)
                    if m and (best is None or m.start() < best[0]):
                        best = (m.start(), i + 1, (p, g), m)
                if best is None:
                    if pos < len(line):
                        tokens.append((line[pos:], list(scopes())))
                    break
                start, _, found, m = best
                if start > pos:
                    tokens.append((line[pos:start], list(scopes())))
                if found is None:
                    inner = list(stack[-1][3])
                    if rule.get("contentName"):
                        inner = inner[:-1]
                    self.captured(m, rule.get("endCaptures", rule.get("captures", {})), inner, tokens)
                    stack.pop()
                elif "match" in found[0]:
                    p = found[0]
                    outer = scopes() + ([p["name"]] if "name" in p else [])
                    self.captured(m, p.get("captures", {}), outer, tokens)
                else:
                    p, g = found
                    outer = scopes() + ([p["name"]] if "name" in p else [])
                    self.captured(m, p.get("beginCaptures", p.get("captures", {})), outer, tokens)
                    inner = outer + ([p["contentName"]] if "contentName" in p else [])
                    closing = p.get("end") or p["while"]
                    closing = re.sub(r"\\(\d)", lambda b: re.escape(m.group(int(b.group(1))) or ""), closing)
                    stack.append((p, g, re.compile(closing), inner))
                if m.end() == pos and found is not None and "match" in found[0]:
                    pos += 1  # an empty match mustn't loop
                    continue
                if m.end() == pos and found is None:
                    pos = m.end()
                    if pos >= len(line):
                        break
                    continue
                pos = m.end()
                if pos >= len(line) and not (stack and "end" in stack[-1][0]):
                    break
        return [(t, s) for t, s in tokens if t]

    @staticmethod
    def captured(m: re.Match, captures: dict, scopes: list[str], tokens: list) -> None:
        """A match's text, with its groups' scopes where it has them."""
        whole = captures.get("0", {}).get("name")
        outer = scopes + ([whole] if whole else [])
        at = m.start()
        groups = sorted(
            (m.start(int(k)), m.end(int(k)), v["name"])
            for k, v in captures.items()
            if k != "0" and "name" in v and m.group(int(k))
        )
        for start, end, name in groups:
            if start > at:
                tokens.append((m.string[at:start], list(outer)))
            tokens.append((m.string[start:end], outer + [name]))
            at = end
        if m.end() > at:
            tokens.append((m.string[at : m.end()], list(outer)))


def scope_of(tokens, text: str, nth: int = 0) -> list[str]:
    """The scopes of the `nth` token that is `text`."""
    found = [s for t, s in tokens if t.strip() == text]
    if len(found) <= nth:
        fail(f"no token {text!r} in {[t for t, _ in tokens]}")
        return []
    return found[nth]


def expect(tokens, text: str, scope: str, nth: int = 0, present: bool = True) -> None:
    got = scope_of(tokens, text, nth)
    has = any(s == scope or s.startswith(scope + ".") for s in got)
    if has != present:
        fail(f"{text!r}: {'wanted' if present else 'unwanted'} {scope} in {got}")


TEMPER = '''/** A box. */
export class Box<T>(public item: T) {
  @connected("Box::size")
  public get size(): Int { 1 }
}
let greet(name: String): String { "Hello, ${name.toUpperCase()}!" }
var n = 0x1F + 2.5e3; // count
let ok = n > 1 && true;
let s = """
  "Line ${greet("a")}
  :if (ok) {
  "yes
  :}
.trim();
'''

MARKDOWN = '''# Title with let in it

Prose: let x = 1 here isn't code.

```temper
let fenced = 1;
```

```js
let notTemper = 1;
```

```temper inert
let inert = 1;
```

    let indented = "yes";

~~~
let tilde = 2;
~~~
'''


def check_scopes() -> None:
    tokens = Tokenizer("source.temper").tokenize(TEMPER)
    expect(tokens, "/**", "comment.block.documentation")
    expect(tokens, "export", "storage.modifier")
    expect(tokens, "class", "storage.type")
    expect(tokens, "Box", "entity.name.type.class")
    expect(tokens, "connected", "entity.name.function.decorator")
    expect(tokens, "get", "storage.type")
    expect(tokens, "Int", "entity.name.type")
    expect(tokens, "greet", "entity.name.function")
    expect(tokens, "${", "punctuation.section.embedded.begin")
    expect(tokens, "toUpperCase", "entity.name.function.call")
    expect(tokens, "toUpperCase", "string.quoted.double")
    expect(tokens, "0x1F", "constant.numeric.hex")
    expect(tokens, "2.5e3", "constant.numeric.decimal")
    expect(tokens, "//", "comment.line")
    expect(tokens, "count", "comment.line")
    expect(tokens, "true", "constant.language")
    expect(tokens, "&&", "keyword.operator")
    # Multi-quoted: content lines are string, `:` lines code; `.trim()` after.
    expect(tokens, "Line", "string.quoted.triple")
    expect(tokens, "if", "keyword.control")
    expect(tokens, "if", "source.temper")
    expect(tokens, "trim", "string.quoted.triple", present=False)
    expect(tokens, "trim", "entity.name.function.call")

    tokens = Tokenizer("text.html.markdown.temper").tokenize(MARKDOWN)
    expect(tokens, "Prose: let x = 1 here isn't code.", "meta.embedded.block.temper", present=False)
    expect(tokens, "fenced", "meta.embedded.block.temper")
    expect(tokens, "fenced", "variable.other.declaration")
    expect(tokens, "let notTemper = 1;", "markup.raw.block")
    expect(tokens, "let notTemper = 1;", "meta.embedded.block.temper", present=False)
    expect(tokens, "let inert = 1;", "meta.embedded.block.temper", present=False)
    expect(tokens, "indented", "meta.embedded.block.temper")
    expect(tokens, "yes", "string.quoted.double")
    expect(tokens, "tilde", "meta.embedded.block.temper")


def words_in(pattern: str) -> set[str]:
    m = re.search(r"\(\?:([a-z|]+)\)", pattern)
    return set(m.group(1).split("|")) if m else set()


def check_keywords() -> None:
    grammar = GRAMMARS["temper.tmLanguage.json"]
    ours: set[str] = set()
    for rule in grammar["repository"]["keywords"]["patterns"]:
        ours |= words_in(rule["match"])
    ts = (ROOT / "site/playground/src/temper-language.ts").read_text()
    theirs: set[str] = set()
    for name in ("KEYWORDS", "ATOMS"):
        block = re.search(name + r" = new Set\(\[(.*?)\]\)", ts, re.S)
        theirs |= set(re.findall(r'"([a-z]+)"', block.group(1) if block else ""))
    rs = (ROOT / "crates/temper-analysis/src/references.rs").read_text()
    block = re.search(r"pub const KEYWORDS: &\[&str\] = &\[(.*?)\];", rs, re.S)
    analysis = set(re.findall(r'"([a-z]+)"', block.group(1) if block else ""))
    for label, other in (("the playground", theirs), ("temper-analysis", analysis)):
        if ours != other:
            fail(f"keywords differ from {label}'s: only here {sorted(ours - other)}, only there {sorted(other - ours)}")


def main() -> int:
    check_structure()
    check_scopes()
    check_keywords()
    for p in problems:
        print(f"check_grammar: {p}", file=sys.stderr)
    if not problems:
        print(f"check_grammar: {len(GRAMMARS)} grammars ok")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
