// A smoke test of the built playground in headless Chromium, driven by
// site/docs/mermaid's playwright-core (no packages of its own):
//
//   node site/playground/smoke.mjs [DIR] [--chromium PATH]
//
// DIR is the build (default site/_out/playground), served by a tiny
// static server at /site/playground/, as Pages nests it. For each
// example: pick it, wait for its compile, check every backend's tab has
// code and no error, that there are no error diagnostics, and that Run
// shows output, with the interpreter's and the JavaScript's runs
// agreeing. Then a hover shows a type, an edit survives a reload in the
// URL's fragment, and the top bar's Temper link leads to the site (../).
// Throughout, no console errors, page errors, or failed requests.
// Chromium is PATH, else $TEMPEST_CHROMIUM, else Playwright's own
// (`npx playwright-core install --only-shell chromium` in
// site/docs/mermaid). Exits with 1 if a check fails, 3 if there's no
// Chromium.

import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { extname, join, normalize, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(new URL("../docs/mermaid/package.json", import.meta.url));
const { chromium } = require("playwright-core");

const BACKENDS = 7;
const TIMEOUT = 60_000;

const args = process.argv.slice(2);
const at = args.indexOf("--chromium");
const executablePath = (at >= 0 ? args.splice(at, 2)[1] : null) || process.env.TEMPEST_CHROMIUM || undefined;
if (args.length > 1) {
  console.error("usage: node smoke.mjs [DIR] [--chromium PATH]");
  process.exit(2);
}
const dir = resolve(args[0] ?? fileURLToPath(new URL("../_out/playground", import.meta.url)));

// The build, at /site/playground/; anything else is a 404.
const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".wasm": "application/wasm",
  ".txt": "text/plain; charset=utf-8",
  ".map": "application/json",
};
const PREFIX = "/site/playground/";
const server = createServer(async (request, response) => {
  const path = decodeURIComponent(new URL(request.url ?? "/", "http://x").pathname);
  const rel = path.startsWith(PREFIX) ? normalize(path.slice(PREFIX.length) || "index.html") : null;
  try {
    if (rel === null || rel.startsWith("..")) throw new Error("outside");
    const body = await readFile(join(dir, rel));
    response.writeHead(200, { "content-type": TYPES[extname(rel)] ?? "application/octet-stream" });
    response.end(body);
  } catch {
    response.writeHead(404).end();
  }
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const origin = `http://127.0.0.1:${server.address().port}`;
const url = `${origin}${PREFIX}`;

let browser;
try {
  browser = await chromium.launch({ executablePath, headless: true, args: ["--no-sandbox"] });
} catch (e) {
  console.error(`smoke.mjs: no Chromium (${executablePath ?? "Playwright's own"}):`);
  console.error(String(e.message).split("\n").slice(0, 3).join("\n"));
  server.close();
  process.exit(3);
}

const failures = [];
const started = performance.now();
let stepStarted = started;
function step(name, problems = []) {
  const ms = performance.now() - stepStarted;
  stepStarted = performance.now();
  failures.push(...problems.map((p) => `${name}: ${p}`));
  console.log(`${problems.length ? "FAIL" : "ok  "} ${name} (${(ms / 1000).toFixed(1)} s)${problems.map((p) => `\n     ${p}`).join("")}`);
}

try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  page.setDefaultTimeout(TIMEOUT);
  const noise = [];
  page.on("console", (m) => m.type() === "error" && noise.push(`console: ${m.text()}`));
  page.on("pageerror", (e) => noise.push(`page error: ${e.message}`));
  page.on("requestfailed", (r) => noise.push(`request failed: ${r.url()} ${r.failure()?.errorText ?? ""}`));
  page.on("response", (r) => r.status() >= 400 && noise.push(`HTTP ${r.status()}: ${r.url()}`));
  const quiet = () => noise.splice(0);

  /** Waits for the compile that `action` starts to finish. */
  async function compiled(action) {
    await page.evaluate(() => {
      const status = document.querySelector("#compile-status");
      if (status) status.textContent = "";
    });
    await action();
    await page.waitForFunction(() => /^Compiled in /.test(document.querySelector("#compile-status")?.textContent ?? ""));
  }

  await compiled(() => page.goto(url));
  step("loads and compiles", quiet());

  const examples = await page.$$eval("#example option", (os) => os.map((o) => [o.value, o.textContent]));
  for (const [value, title] of examples) {
    const problems = [];
    if (value !== (await page.$eval("#example", (e) => e.value))) {
      await compiled(() => page.selectOption("#example", value));
    }
    const tabs = await page.$$eval(".tab", (ts) => ts.map((t) => t.dataset.tab));
    const backends = tabs.filter((t) => t !== "run" && t !== "errors");
    if (backends.length !== BACKENDS) problems.push(`${backends.length} backend tabs, not ${BACKENDS}`);
    for (const backend of backends) {
      await page.click(`#tab-${backend}`);
      const error = await page.$eval("#backend-error", (e) => (e.hidden ? "" : e.textContent));
      const code = await page.$eval("#code .cm-content", (e) => e.textContent ?? "");
      if (error) problems.push(`${backend}: ${error}`);
      else if (!code.trim()) problems.push(`${backend}: no code`);
    }
    const errors = await page.$$eval("#errors .diagnostic.error", (ds) => ds.map((d) => d.textContent));
    problems.push(...errors.map((e) => `error: ${e}`));
    if (await page.$(".cm-lintRange-error")) problems.push("an error underlined in the editor");
    await page.click("#tab-run");
    await page.waitForSelector("#same:not([hidden])");
    const interp = await page.$eval("#interp", (e) => e.textContent ?? "");
    const js = await page.$eval("#js", (e) => e.textContent ?? "");
    const same = await page.$eval("#same", (e) => `${e.dataset.kind}: ${e.textContent}`);
    if (!interp.trim()) problems.push("no interpreter output");
    if (!js.trim()) problems.push("no JavaScript output");
    if (!same.startsWith("good:")) problems.push(same);
    if (/✗|Tests passed: (\d+) of (?!\1\b)/.test(interp + js)) problems.push("a test failed");
    step(`example ${title}`, [...problems, ...quiet()]);
  }

  // Hover: on the first example, over `greet` where it's called.
  await compiled(() => page.selectOption("#example", examples[0][0]));
  const box = await page.evaluate(() => {
    for (const line of document.querySelectorAll("#editor .cm-line")) {
      if (!line.textContent?.startsWith("console.log(greet")) continue;
      const walker = document.createTreeWalker(line, NodeFilter.SHOW_TEXT);
      for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        const i = node.textContent?.indexOf("greet") ?? -1;
        if (i < 0) continue;
        const range = document.createRange();
        range.setStart(node, i + 1);
        range.setEnd(node, i + 3);
        const r = range.getBoundingClientRect();
        return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
      }
    }
    return null;
  });
  if (box) {
    await page.mouse.move(box.x, box.y);
    const type = await page
      .waitForSelector(".cm-tooltip-hover .cm-hover-type", { timeout: 10_000 })
      .then((e) => e.textContent())
      .catch(() => "");
    step("hover shows a type", type.includes("String") ? quiet() : [`hover showed ${JSON.stringify(type)}`, ...quiet()]);
  } else {
    step("hover shows a type", ["no `greet` call to hover over"]);
  }

  // Permalink: an edit goes into the fragment and survives a reload.
  const mark = "// smoke test was here";
  await compiled(async () => {
    await page.click("#editor .cm-content");
    await page.keyboard.press("Control+End");
    await page.keyboard.type(`\n${mark}`);
  });
  await page.waitForFunction(() => location.hash.length > 1);
  await compiled(() => page.reload());
  const text = await page.$eval("#editor .cm-content", (e) => e.textContent ?? "");
  step("permalink survives a reload", text.includes(mark) ? quiet() : ["the edit was lost", ...quiet()]);

  // The Temper link: to the site, one level up, with the logo.
  const link = await page.$eval("header .brand a", (a) => ({
    href: a.getAttribute("href"),
    resolved: a.href,
    logo: a.querySelector("svg.logo")?.getAttribute("aria-label") ?? null,
  }));
  const linkProblems = [];
  if (link.href !== "../" || link.resolved !== `${origin}/site/`) linkProblems.push(`href ${link.href} (${link.resolved})`);
  if (link.logo !== "Temper") linkProblems.push("no Temper logo");
  step("Temper link", linkProblems);
} catch (e) {
  step("unexpected", [String(e.message ?? e).split("\n")[0]]);
} finally {
  await browser.close();
  server.close();
}

const seconds = ((performance.now() - started) / 1000).toFixed(1);
if (failures.length) {
  console.error(`smoke.mjs: ${failures.length} problems, in ${seconds} s`);
  process.exit(1);
}
console.log(`smoke.mjs: all passed in ${seconds} s`);
