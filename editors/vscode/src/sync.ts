// Scroll sync between a document's lines and its preview's offsets, from
// the blocks the Markdown engine marks with their first source line
// (`class="code-line" data-line="N"`). Shared by the extension's tests
// and the preview's script; no DOM here.

/** A marked block: its first source line and its top in the preview. */
export interface Block {
  line: number;
  top: number;
}

/** The blocks in order of both line and top: nested and out-of-order
 * ones (a list item and its first paragraph share a line) dropped. */
export function ordered(blocks: readonly Block[]): Block[] {
  const out: Block[] = [];
  for (const b of blocks) {
    const last = out[out.length - 1];
    if (!Number.isFinite(b.line) || !Number.isFinite(b.top)) continue;
    if (!last || (b.line > last.line && b.top >= last.top)) out.push(b);
  }
  return out;
}

/** The two blocks around `x` by `key`: the last at or before, the first after. */
function around(blocks: readonly Block[], x: number, key: "line" | "top"): [Block | undefined, Block | undefined] {
  let lo = 0;
  let hi = blocks.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (blocks[mid]![key] <= x) lo = mid + 1;
    else hi = mid;
  }
  return [blocks[lo - 1], blocks[lo]];
}

/** The preview offset showing source `line` (fractional) at the top,
 * interpolated between the blocks around it. */
export function topForLine(blocks: readonly Block[], line: number): number {
  const [before, after] = around(blocks, line, "line");
  if (!before) return 0;
  if (!after) return before.top;
  return before.top + ((line - before.line) / (after.line - before.line)) * (after.top - before.top);
}

/** The source line (fractional) at preview offset `top`: the inverse of
 * topForLine, so a scroll synced one way doesn't drift synced back. */
export function lineForTop(blocks: readonly Block[], top: number): number {
  const [before, after] = around(blocks, top, "top");
  if (!before) return 0;
  if (!after || after.top === before.top) return before.line;
  return before.line + ((top - before.top) / (after.top - before.top)) * (after.line - before.line);
}
