// Tokenizes Temper and literate Temper with VS Code's own TextMate engine
// (vscode-textmate, with Oniguruma as wasm from vscode-oniguruma), headless:
//
//     node editors/vscode/test/grammar.mjs [FILE.temper.md ...]
//
// It checks a few samples' scopes, and then, in every literate file given
// (by default the repository's docs and notes), that no line starting with
// code is still inside a multi-quoted string: a `"""` string ends at the
// first line without a margin (`"`, `~`, `:`, or a comment). VS Code's
// built-in Markdown grammar isn't here, so prose isn't scoped; Temper's is.
// check_grammar.py still checks the grammars' structure and keywords.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import oniguruma from "vscode-oniguruma";
import textmate from "vscode-textmate";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..", "..");
const require = createRequire(import.meta.url);
const wasm = readFileSync(require.resolve("vscode-oniguruma/release/onig.wasm"));
await oniguruma.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));

const files = {
  "source.temper": join(here, "..", "syntaxes", "temper.tmLanguage.json"),
  "text.html.markdown.temper": join(here, "..", "syntaxes", "temper-markdown.tmLanguage.json"),
};
const registry = new textmate.Registry({
  onigLib: Promise.resolve({
    createOnigScanner: (patterns) => new oniguruma.OnigScanner(patterns),
    createOnigString: (s) => new oniguruma.OnigString(s),
  }),
  loadGrammar: async (scope) =>
    files[scope] ? textmate.parseRawGrammar(readFileSync(files[scope], "utf8"), files[scope]) : null,
});

/** Each line's tokens: [text, scopes] pairs. */
function tokenize(grammar, text) {
  let state = textmate.INITIAL;
  return text.split("\n").map((line) => {
    const result = grammar.tokenizeLine(line, state);
    state = result.ruleStack;
    return result.tokens.map((t) => [line.slice(t.startIndex, t.endIndex), t.scopes]);
  });
}

let failures = 0;
function fail(message) {
  failures++;
  console.error(`grammar: ${message}`);
}

/** Checks that the first token whose text contains `word` has (or lacks) a scope starting with `scope`. */
function expect(lines, word, scope, present = true) {
  for (const tokens of lines) {
    for (const [text, scopes] of tokens) {
      if (text.includes(word)) {
        if (scopes.some((s) => s.startsWith(scope)) !== present) {
          fail(`'${word}': wanted ${present ? "" : "no "}${scope} in ${scopes.join(" ")}`);
        }
        return;
      }
    }
  }
  fail(`no token with '${word}'`);
}

const temper = await registry.loadGrammar("source.temper");
const markdown = await registry.loadGrammar("text.html.markdown.temper");

// Samples.
{
  const lines = tokenize(temper, [
    'let page = html"""',
    '  ~<p>"${name}"</p>',
    '  : if (more) {',
    '  ~<b>more</b>',
    '  : }',
    "let afterTriple = 3;",
  ].join("\n"));
  expect(lines, "<p>", "string.quoted.triple");
  expect(lines, "if", "keyword.control");
  expect(lines, "afterTriple", "variable.other.declaration");
  expect(lines, "afterTriple", "string.quoted.triple", false);
}
{
  const lines = tokenize(markdown, [
    "Prose.",
    "",
    '    let page = html"""',
    '      ~<p>"${name}"</p>',
    "    let afterTriple = 3;",
    "",
    "```temper",
    'let fenced = html"""',
    '  ~<p>x</p>',
    "let afterFenced = 4;",
    "```",
  ].join("\n"));
  expect(lines, "<p>", "string.quoted.triple");
  expect(lines, "afterTriple", "variable.other.declaration");
  expect(lines, "afterTriple", "string.quoted.triple", false);
  expect(lines, "afterFenced", "variable.other.declaration");
  expect(lines, "afterFenced", "string.quoted.triple", false);
}

// Literate files: no code line left inside a multi-quoted string.
function literate(dir, out = []) {
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name.startsWith(".") || name === "target" || name === "_out") continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) literate(path, out);
    else if (name.endsWith(".temper.md")) out.push(path);
  }
  return out;
}
const given = process.argv.slice(2);
const paths = given.length ? given : ["docs", "notes", "crates"].flatMap((d) => literate(join(root, d)));
let checked = 0;
for (const path of paths) {
  const text = readFileSync(path, "utf8");
  tokenize(markdown, text).forEach((tokens, i) => {
    const first = tokens.find(([t]) => t.trim() !== "");
    if (!first) return;
    const lead = first[0].trimStart()[0];
    const inTriple = first[1].some((s) => s.startsWith("string.quoted.triple"));
    const code = first[1].some((s) => s.startsWith("meta.embedded.block.temper"));
    if (code && inTriple && !`"~:/*`.includes(lead)) {
      fail(`${relative(root, path)}:${i + 1}: code still inside a """ string: ${first[0].trim().slice(0, 60)}`);
    }
  });
  checked++;
}

if (failures) {
  console.error(`grammar: ${failures} problem(s)`);
  process.exit(1);
}
console.log(`grammar: samples ok; ${checked} literate file(s) ok, with VS Code's TextMate engine`);
