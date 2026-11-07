// Temper highlighting for the preview's code blocks, from the same
// TextMate grammar the editor uses (syntaxes/temper.tmLanguage.json): a
// small TextMate tokenizer, like check_grammar.py's, enough for that
// grammar (match, begin/end and begin/while with captures, includes,
// contentName), on JavaScript regexes. Tokens become spans with a few
// classes the preview's CSS colors.

/** A TextMate rule, as the grammar's JSON has it. */
export interface Rule {
  name?: string;
  contentName?: string;
  match?: string;
  begin?: string;
  end?: string;
  while?: string;
  include?: string;
  captures?: Captures;
  beginCaptures?: Captures;
  endCaptures?: Captures;
  patterns?: Rule[];
}

type Captures = Record<string, { name?: string }>;

export interface Grammar extends Rule {
  scopeName: string;
  repository?: Record<string, Rule>;
}

/** A token's text and its scopes, outermost first. */
export type Token = [text: string, scopes: string[]];

interface Open {
  rule: Rule;
  closing: RegExp;
  scopes: string[];
}

const regexes = new Map<string, RegExp>();

/** A pattern compiled once, global (to search from a position) with indices. */
function compile(source: string): RegExp {
  let re = regexes.get(source);
  if (!re) {
    re = new RegExp(source, "gd");
    regexes.set(source, re);
  }
  return re;
}

function search(re: RegExp, line: string, pos: number): RegExpExecArray | null {
  re.lastIndex = pos;
  return re.exec(line);
}

const escapeRegExp = (text: string): string => text.replace(/[\\^$.*+?()[\]{}|/-]/g, "\\$&");

/** Tokenizes `text` with `grammar`, line by line. */
export function tokenize(grammar: Grammar, text: string): Token[] {
  const expanded = new Map<Rule, Rule[]>();
  // A rule's patterns, includes expanded (within this one grammar).
  const patterns = (rule: Rule): Rule[] => {
    let found = expanded.get(rule);
    if (found) return found;
    found = [];
    const seen = new Set<Rule>();
    const add = (list: Rule[] | undefined): void => {
      for (const p of list ?? []) {
        let target: Rule | undefined = p;
        if (p.include !== undefined) {
          target = p.include.startsWith("#") ? grammar.repository?.[p.include.slice(1)] : p.include === "$self" || p.include === grammar.scopeName ? { patterns: grammar.patterns } : undefined;
          if (!target || seen.has(target)) continue;
          seen.add(target);
        }
        if (target.match !== undefined || target.begin !== undefined) found!.push(target);
        else add(target.patterns);
      }
    };
    add(rule.patterns);
    expanded.set(rule, found);
    return found;
  };

  const base = [grammar.scopeName];
  const stack: Open[] = [];
  const tokens: Token[] = [];
  const push = (t: string, scopes: string[]): void => {
    if (t) tokens.push([t, scopes]);
  };
  const scopes = (): string[] => stack[stack.length - 1]?.scopes ?? base;

  const lines = text.split("\n");
  lines.forEach((line, n) => {
    let pos = 0;
    // `while` rules hold only while their line starts match.
    for (let depth = 0; depth < stack.length; depth++) {
      const open = stack[depth]!;
      if (open.rule.while === undefined) continue;
      const m = search(open.closing, line, pos);
      if (!m || m.index !== pos) {
        stack.length = depth;
        break;
      }
      push(m[0], open.scopes);
      pos += m[0].length;
    }
    for (let guard = 0; pos <= line.length && guard < 10_000; guard++) {
      const open = stack[stack.length - 1];
      let best: { m: RegExpExecArray; rule?: Rule } | undefined;
      if (open?.rule.end !== undefined) {
        const m = search(open.closing, line, pos);
        if (m) best = { m };
      }
      for (const p of patterns(open?.rule ?? grammar)) {
        const m = search(compile((p.match ?? p.begin)!), line, pos);
        if (m && (!best || m.index < best.m.index)) best = { m, rule: p };
      }
      if (!best) {
        push(line.slice(pos), scopes());
        break;
      }
      const { m, rule } = best;
      push(line.slice(pos, m.index), scopes());
      const end = m.index + m[0].length;
      if (!rule) {
        // The open rule ends.
        const inner = open!.rule.contentName ? open!.scopes.slice(0, -1) : open!.scopes;
        captured(m, open!.rule.endCaptures ?? open!.rule.captures, inner, push);
        stack.pop();
      } else {
        const outer = rule.name ? [...scopes(), rule.name] : scopes();
        if (rule.match !== undefined) {
          captured(m, rule.captures, outer, push);
        } else {
          captured(m, rule.beginCaptures ?? rule.captures, outer, push);
          // End and while may refer to begin's groups.
          const closing = (rule.end ?? rule.while)!.replace(/\\(\d)/g, (_, g: string) => escapeRegExp(m[Number(g)] ?? ""));
          stack.push({ rule, closing: compile(closing), scopes: rule.contentName ? [...outer, rule.contentName] : outer });
        }
      }
      if (end === pos) {
        // An empty match mustn't loop: a match rule steps on; an end or a
        // begin stays, as it changed the stack.
        if (rule?.match !== undefined) {
          push(line.slice(pos, pos + 1), scopes());
          pos += 1;
        } else if (pos >= line.length) break;
        continue;
      }
      pos = end;
      if (pos >= line.length && stack[stack.length - 1]?.rule.end === undefined) break;
    }
    if (n < lines.length - 1) push("\n", scopes());
  });
  return tokens;
}

/** A match's text, with its groups' scopes where it has them. */
function captured(m: RegExpExecArray, captures: Captures | undefined, scopes: string[], push: (t: string, s: string[]) => void): void {
  const whole = captures?.["0"]?.name;
  const outer = whole ? [...scopes, whole] : scopes;
  const groups: [number, number, string][] = [];
  for (const [k, v] of Object.entries(captures ?? {})) {
    const at = m.indices?.[Number(k)];
    if (k !== "0" && v.name && at && at[1] > at[0]) groups.push([at[0], at[1], v.name]);
  }
  groups.sort((a, b) => a[0] - b[0]);
  let at = m.index;
  for (const [start, end, name] of groups) {
    if (start < at) continue;
    push(m.input.slice(at, start), outer);
    push(m.input.slice(start, end), [...outer, name]);
    at = end;
  }
  push(m.input.slice(at, m.index + m[0].length), outer);
}

// The preview's classes, by scope prefix, most specific first.
const CLASSES: [string, string][] = [
  ["comment", "comment"],
  ["constant.character.escape", "escape"],
  ["punctuation.section.embedded", "escape"],
  ["string", "string"],
  ["constant.numeric", "number"],
  ["constant.language", "constant"],
  ["variable.language", "constant"],
  ["keyword.operator", "operator"],
  ["keyword.control", "control"],
  ["keyword", "keyword"],
  ["storage", "keyword"],
  ["entity.name.function.decorator", "decorator"],
  ["punctuation.decorator", "decorator"],
  ["entity.name.type", "type"],
  ["entity.name.function", "function"],
  ["variable.other.declaration", "variable"],
];

/** The preview's class for a token's scopes, innermost scope first. */
export function classOf(scopes: readonly string[]): string | undefined {
  for (let i = scopes.length - 1; i >= 0; i--) {
    const scope = scopes[i]!;
    // Code embedded in a string isn't string.
    if (scope === "source.temper" && i > 0) return undefined;
    for (const [prefix, name] of CLASSES) {
      if (scope === prefix || scope.startsWith(prefix + ".")) return name;
    }
  }
  return undefined;
}

const escapes: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" };
export const escapeHtml = (text: string): string => text.replace(/[&<>"]/g, (c) => escapes[c]!);

/** Temper code as HTML: escaped, with `<span class="tm-CLASS">` per token. */
export function highlight(grammar: Grammar, code: string): string {
  let html = "";
  let open: string | undefined;
  let run = "";
  const flush = (): void => {
    if (run) html += open ? `<span class="tm-${open}">${escapeHtml(run)}</span>` : escapeHtml(run);
    run = "";
  };
  for (const [text, scopes] of tokenize(grammar, code)) {
    const name = /^\s*$/.test(text) ? undefined : classOf(scopes);
    if (name !== open) {
      flush();
      open = name;
    }
    run += text;
  }
  flush();
  return html;
}
