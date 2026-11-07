// The preview's script, in its webview: takes new content, keeps the
// preview scrolled with the editor and the editor with the preview, and
// sends clicked links to the extension to open.

import type { FromPreview, ToPreview } from "../src/messages";
import { type Block, lineForTop, ordered, topForLine } from "../src/sync";

declare function acquireVsCodeApi(): { postMessage(message: FromPreview): void };

const vscode = acquireVsCodeApi();
const content = document.getElementById("content")!;

/** Until when scroll events are ours, from following the editor, and so
 * not sent back to it. */
let quietUntil = 0;
/** The line last asked for, kept through updates. */
let wanted: number | undefined;

function blocks(): Block[] {
  const found: Block[] = [];
  for (const el of content.querySelectorAll<HTMLElement>(".code-line[data-line]")) {
    found.push({ line: Number(el.dataset.line), top: el.getBoundingClientRect().top + window.scrollY });
  }
  return ordered(found);
}

function scrollToLine(line: number): void {
  wanted = line;
  const top = Math.max(0, Math.round(topForLine(blocks(), line)));
  quietUntil = Date.now() + 200;
  window.scrollTo(0, top);
  vscode.postMessage({ type: "scrolled", line, top: window.scrollY });
}

function rendered(version: number): void {
  const headings = Array.from(content.querySelectorAll("h1, h2, h3, h4, h5, h6"), (h) => h.textContent ?? "");
  const highlighted = content.querySelectorAll("code[data-temper] span[class^='tm-']").length;
  vscode.postMessage({ type: "rendered", version, headings, highlighted });
}

window.addEventListener("message", (event: MessageEvent<ToPreview>) => {
  const message = event.data;
  switch (message.type) {
    case "update": {
      const y = window.scrollY;
      content.innerHTML = message.html;
      content.dataset.version = String(message.version);
      quietUntil = Date.now() + 200;
      window.scrollTo(0, y);
      rendered(message.version);
      break;
    }
    case "scroll":
      scrollToLine(message.line);
      break;
    case "userScroll":
      quietUntil = 0;
      window.scrollTo(0, message.top);
      break;
  }
});

let pending = false;
window.addEventListener("scroll", () => {
  if (Date.now() < quietUntil || pending) return;
  pending = true;
  requestAnimationFrame(() => {
    pending = false;
    if (Date.now() < quietUntil) return;
    const line = lineForTop(blocks(), window.scrollY);
    wanted = line;
    vscode.postMessage({ type: "reveal", line });
  });
});

// Images load after layout and move what's below them: stay on the line.
function imageDone(): void {
  if (wanted !== undefined) {
    quietUntil = Date.now() + 200;
    window.scrollTo(0, Math.max(0, Math.round(topForLine(blocks(), wanted))));
  }
  const images = Array.from(content.querySelectorAll("img"));
  const loaded = images.filter((i) => i.complete && i.naturalWidth > 0).length;
  const failed = images.filter((i) => i.complete && i.naturalWidth === 0).length;
  vscode.postMessage({ type: "images", loaded, failed });
}
content.addEventListener("load", imageDone, true);
content.addEventListener("error", imageDone, true);

document.addEventListener("click", (event) => {
  const a = (event.target as Element | null)?.closest?.("a");
  if (!a) return;
  const href = a.getAttribute("data-href") ?? a.getAttribute("href");
  if (href === null) return;
  event.preventDefault();
  if (href.startsWith("#")) {
    let id = href.slice(1);
    try {
      id = decodeURIComponent(id);
    } catch {
      // As written.
    }
    document.getElementById(id)?.scrollIntoView();
    return;
  }
  vscode.postMessage({ type: "open", href });
});

vscode.postMessage({ type: "ready" });
rendered(Number(content.dataset.version));
