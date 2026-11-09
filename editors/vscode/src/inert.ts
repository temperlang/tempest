// Marks `temper inert` blocks in `.temper.md` editors, which the
// literate split doesn't compile: their lines tinted, and an "inert"
// label after the opening fence, with a hover saying what it means.
// The grammar highlights them as Temper; the language server, rightly,
// doesn't analyze them.

import * as vscode from "vscode";
import { INERT_TITLE, inertBlocks } from "./page";

/** For tests: each document's inert blocks as last decorated. */
export type InertRanges = (uri: vscode.Uri) => vscode.Range[] | undefined;

export function registerInert(context: vscode.ExtensionContext): InertRanges {
  const tint = vscode.window.createTextEditorDecorationType({
    isWholeLine: true,
    backgroundColor: new vscode.ThemeColor("temper.inertBlockBackground"),
  });
  const label = vscode.window.createTextEditorDecorationType({
    after: {
      contentText: "inert",
      color: new vscode.ThemeColor("temper.inertLabelForeground"),
      border: "1px solid",
      borderColor: new vscode.ThemeColor("temper.inertLabelForeground"),
      margin: "0 0 0 1.5em",
      fontStyle: "italic",
    },
  });
  const decorated = new Map<string, vscode.Range[]>();
  const pending = new Set<vscode.TextEditor>();
  let timer: ReturnType<typeof setTimeout> | undefined;

  const decorate = (editor: vscode.TextEditor): void => {
    const document = editor.document;
    if (document.languageId !== "temper-markdown") return;
    const last = (line: number) => document.lineAt(Math.min(line, document.lineCount - 1)).range.end;
    const blocks = inertBlocks(document.getText()).map((b) => new vscode.Range(b.open, 0, b.close, 0).with({ end: last(b.close) }));
    editor.setDecorations(tint, blocks);
    editor.setDecorations(
      label,
      blocks.map((b) => ({ range: document.lineAt(b.start.line).range, hoverMessage: INERT_TITLE })),
    );
    decorated.set(document.uri.toString(), blocks);
  };
  // Edits come a keystroke at a time; decorate after a short pause.
  const soon = (editors: readonly vscode.TextEditor[]): void => {
    for (const editor of editors) pending.add(editor);
    clearTimeout(timer);
    timer = setTimeout(() => {
      for (const editor of pending) decorate(editor);
      pending.clear();
    }, 100);
  };
  context.subscriptions.push(
    tint,
    label,
    vscode.window.onDidChangeVisibleTextEditors((editors) => editors.forEach(decorate)),
    vscode.workspace.onDidChangeTextDocument((e) =>
      soon(vscode.window.visibleTextEditors.filter((editor) => editor.document === e.document)),
    ),
    { dispose: () => clearTimeout(timer) },
  );
  vscode.window.visibleTextEditors.forEach(decorate);
  return (uri) => decorated.get(uri.toString());
}
