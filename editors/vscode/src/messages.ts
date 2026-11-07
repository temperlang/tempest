// The messages between the extension and its preview's webview.

/** From the extension to the preview. */
export type ToPreview =
  /** New content, rendered from the document at `version`. */
  | { type: "update"; html: string; version: number }
  /** Shows source `line` (fractional) at the top: the editor scrolled. */
  | { type: "scroll"; line: number }
  /** For tests: scrolls to `top` as a reader would, so the editor follows. */
  | { type: "userScroll"; top: number };

/** From the preview to the extension. */
export type FromPreview =
  /** The script is running and can take messages. */
  | { type: "ready" }
  /** Content is in (after the first load or an update), with what's in it. */
  | { type: "rendered"; version: number; headings: string[]; highlighted: number }
  /** An image loaded (or failed to): how many have, so far. */
  | { type: "images"; loaded: number; failed: number }
  /** The preview followed the editor to `line`, at offset `top`. */
  | { type: "scrolled"; line: number; top: number }
  /** The reader scrolled the preview: the editor follows to `line`. */
  | { type: "reveal"; line: number }
  /** A link was clicked, its `href` as written in the document. */
  | { type: "open"; href: string };
