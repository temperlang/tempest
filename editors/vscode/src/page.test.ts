// Tests for the preview's pure parts (highlight.ts, page.ts, sync.ts),
// under Node's test runner, run from editors/vscode.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { type Grammar, classOf, highlight, tokenize } from "./highlight";
import { INERT_TITLE, body, fragmentLine, highlightCodeBlocks, inertBlocks, linkTarget, page } from "./page";
import { lineForTop, ordered, topForLine } from "./sync";

const grammar = JSON.parse(readFileSync("syntaxes/temper.tmLanguage.json", "utf8")) as Grammar;

/** The preview class of the `nth` token whose text is `text`. */
function classAt(code: string, text: string, nth = 0): string | undefined {
  const found = tokenize(grammar, code).filter(([t]) => t.trim() === text);
  assert.ok(found.length > nth, `no token ${JSON.stringify(text)} in ${JSON.stringify(tokenize(grammar, code).map(([t]) => t))}`);
  return classOf(found[nth]![1]);
}

const TEMPER = `/** A box. */
export class Box<T>(public item: T) {
  @connected("Box::size")
  public get size(): Int { 1 }
}
let greet(name: String): String { "Hello, \${name.toUpperCase()}!" }
var n = 0x1F + 2.5e3; // count
let ok = n > 1 && true;
let s = """
  "Line \${greet("a")}
  :if (ok) {
  "yes
  :}
.trim();
`;

test("tokens take the grammar's scopes", () => {
  assert.equal(classAt(TEMPER, "/**"), "comment");
  assert.equal(classAt(TEMPER, "export"), "keyword");
  assert.equal(classAt(TEMPER, "Box"), "type");
  assert.equal(classAt(TEMPER, "connected"), "decorator");
  assert.equal(classAt(TEMPER, "greet"), "function");
  assert.equal(classAt(TEMPER, "Int"), "type");
  assert.equal(classAt(TEMPER, "${"), "escape");
  // Code inside a string's interpolation isn't string.
  assert.equal(classAt(TEMPER, "toUpperCase"), "function");
  assert.equal(classAt(TEMPER, "0x1F"), "number");
  assert.equal(classAt(TEMPER, "count"), "comment");
  assert.equal(classAt(TEMPER, "true"), "constant");
  assert.equal(classAt(TEMPER, "ok"), "variable");
  // Multi-quoted: content lines are string, `:` lines code, `.trim()` after.
  assert.equal(classAt(TEMPER, "Line"), "string");
  assert.equal(classAt(TEMPER, "if"), "control");
  assert.equal(classAt(TEMPER, "trim"), "function");
  // Every character is in some token, in order.
  assert.equal(tokenize(grammar, TEMPER).map(([t]) => t).join(""), TEMPER);
});

test("highlighting escapes and wraps tokens", () => {
  const html = highlight(grammar, 'let a = "<b>" & 1;');
  assert.equal(html, '<span class="tm-keyword">let</span> <span class="tm-variable">a</span> <span class="tm-operator">=</span> <span class="tm-string">&quot;&lt;b&gt;&quot;</span> <span class="tm-operator">&amp;</span> <span class="tm-number">1</span>;');
});

test("Temper code blocks are highlighted, others left", () => {
  const rendered = [
    '<h1 data-line="0" class="code-line" dir="auto" id="title">Title</h1>',
    // As VS Code's engine renders them: fenced, tagged and untagged,
    // indented, and other languages, with and without highlighting.
    '<pre><code data-line="2" class="code-line language-temper" dir="auto">let x = &quot;a&quot;;\n</code></pre>',
    '<pre><code data-line="6" class="code-line" dir="auto">let y = 1;\n</code></pre>',
    '<pre data-line="9" class="code-line" dir="auto"><code>let z = 2;\n</code></pre>',
    '<pre><code data-line="11" class="code-line language-js" dir="auto"><span class="hljs-keyword">let</span> q = 3;\n</code></pre>',
    '<pre><code data-line="14" class="code-line language-text" dir="auto">let w = 4;\n</code></pre>',
  ].join("\n");
  const html = highlightCodeBlocks(rendered, grammar);
  assert.match(html, /<code data-temper="" data-line="2" class="code-line language-temper" dir="auto"><span class="tm-keyword">let<\/span> <span class="tm-variable">x<\/span> <span class="tm-operator">=<\/span> <span class="tm-string">&quot;a&quot;<\/span>;/);
  assert.match(html, /<span class="tm-variable">y<\/span>/);
  assert.match(html, /<span class="tm-variable">z<\/span>/);
  assert.match(html, /<span class="hljs-keyword">let<\/span> q = 3;/);
  assert.match(html, /<code data-line="14" class="code-line language-text" dir="auto">let w = 4;/);
  assert.equal((html.match(/data-temper/g) ?? []).length, 3);
  assert.match(body(rendered, grammar, 12), /<div class="code-line" data-line="12"><\/div>$/);
});

test("temper inert blocks are found as the literate split finds fences", () => {
  const source = [
    "# Notes", // 0
    "```temper inert", // 1
    "let a = 1;",
    "```", // 3
    "````markdown", // 4
    "```temper inert", // 5: in another fence, not one
    "```",
    "````", // 7
    "~~~ temper  other inert", // 8
    "~~~~", // 9
    "```temper", // 10: compiled
    "```",
    "```inert", // 12: not Temper
    "```",
    "```temper inert", // 14: never closed
    "let b = 2;",
  ].join("\n");
  assert.deepEqual(inertBlocks(source), [
    { open: 1, close: 3 },
    { open: 8, close: 9 },
    { open: 14, close: 15 },
  ]);
});

test("inert blocks are highlighted, marked, and labeled", () => {
  // VS Code's engine drops the info after `temper`, so the fence's line
  // says which blocks are inert.
  const rendered = [
    '<pre><code data-line="1" class="code-line language-temper" dir="auto">let a = 1;\n</code></pre>',
    '<pre><code data-line="5" class="code-line language-temper" dir="auto">let b = 2;\n</code></pre>',
  ].join("\n");
  const html = body(rendered, grammar, 8, "x\n```temper inert\nlet a = 1;\n```\n\n```temper\nlet b = 2;\n```");
  assert.match(html, new RegExp(`^<pre><span class="inert-label" title="${INERT_TITLE}">inert</span><code data-temper="" data-inert="" data-line="1"`));
  assert.match(html, /<span class="tm-variable">a<\/span>/);
  assert.equal((html.match(/data-inert/g) ?? []).length, 1);
  assert.equal((html.match(/inert-label/g) ?? []).length, 1);
  assert.match(html, /<code data-temper="" data-line="5"/);
});

test("links: fragments, the web, and files", () => {
  assert.deepEqual(linkTarget("#a%20b"), { kind: "fragment", id: "a b" });
  assert.deepEqual(linkTarget("https://temperlang.dev/"), { kind: "external", uri: "https://temperlang.dev/" });
  assert.deepEqual(linkTarget("mailto:a@b.c"), { kind: "external", uri: "mailto:a@b.c" });
  assert.deepEqual(linkTarget("shapes.temper#L2"), { kind: "file", path: "shapes.temper", fragment: "L2", from: "document" });
  assert.deepEqual(linkTarget("/docs/a%20b.md"), { kind: "file", path: "/docs/a b.md", fragment: "", from: "workspace" });
  assert.deepEqual(linkTarget("file:///tmp/x.md#top"), { kind: "file", path: "/tmp/x.md", fragment: "top", from: "root" });
  assert.equal(linkTarget("command:workbench.action.quit"), undefined);
  assert.equal(linkTarget("javascript:alert(1)"), undefined);
  assert.equal(fragmentLine("L12-L14"), 11);
  assert.equal(fragmentLine("heading"), undefined);
});

test("the page's CSP allows only nonce scripts and nothing remote", () => {
  const html = page({ cspSource: "vscode-webview://x", nonce: "n0", base: "vscode-webview://x/dir/", style: "s.css", script: "p.js", title: "a<b", body: "<p>hi</p>", version: 3 });
  const csp = /content="([^"]*)"/.exec(html)?.[1] ?? "";
  assert.match(csp, /default-src 'none'/);
  assert.match(csp, /script-src 'nonce-n0'(;|$)/);
  assert.doesNotMatch(csp, /https:|\*|unsafe-eval/);
  assert.doesNotMatch(/script-src[^;]*/.exec(csp)?.[0] ?? "", /unsafe/);
  assert.match(html, /<script nonce="n0" src="p.js">/);
  assert.match(html, /<title>a&lt;b<\/title>/);
  assert.match(html, /data-version="3"><p>hi<\/p><\/main>/);
});

test("scroll sync maps lines to offsets and back", () => {
  const blocks = ordered([
    { line: 0, top: 0 },
    { line: 4, top: 100 },
    { line: 4, top: 100 },
    { line: 3, top: 150 },
    { line: 10, top: 400 },
  ]);
  assert.deepEqual(blocks.map((b) => b.line), [0, 4, 10]);
  assert.equal(topForLine(blocks, 2), 50);
  assert.equal(topForLine(blocks, 7), 250);
  assert.equal(topForLine(blocks, 20), 400);
  assert.equal(lineForTop(blocks, 250), 7);
  assert.equal(lineForTop(blocks, 999), 10);
  for (const line of [0, 1.5, 4, 8.25]) assert.equal(lineForTop(blocks, topForLine(blocks, line)), line);
  assert.equal(topForLine([], 3), 0);
});
