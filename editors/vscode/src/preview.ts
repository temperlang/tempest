// The rendered preview of literate Temper (`.temper.md`), which VS Code's
// own Markdown preview doesn't offer for our language: VS Code's Markdown
// engine (the built-in `markdown.api.render` command) renders it, page.ts
// highlights its Temper blocks with our grammar, and a webview shows it,
// updated as the document changes, and scrolled with its editor both ways.

import * as vscode from "vscode";
import type { Grammar } from "./highlight";
import type { FromPreview, ToPreview } from "./messages";
import { body, fragmentLine, linkTarget, page } from "./page";

const VIEW_TYPE = "temper.preview";
/** How long typing pauses before the preview renders again. */
const DEBOUNCE_MS = 150;
/** How long after following the preview the editor's scrolling is ours. */
const QUIET_MS = 200;

/** One preview, for tests: what it last showed and heard. */
export interface PreviewHook {
  readonly uri: vscode.Uri;
  readonly panel: vscode.WebviewPanel;
  /** The messages from its webview, oldest first. */
  readonly messages: FromPreview[];
  post(message: ToPreview): Thenable<boolean>;
}

class Preview implements PreviewHook {
  readonly messages: FromPreview[] = [];
  private readonly disposables: vscode.Disposable[] = [];
  private timer: ReturnType<typeof setTimeout> | undefined;
  private quietUntil = 0;
  private rendering = 0;
  /** The last update sent, for a webview that loads again (shown after
   * being hidden) from the first page. */
  private latest: { html: string; version: number } | undefined;

  constructor(
    readonly uri: vscode.Uri,
    readonly panel: vscode.WebviewPanel,
    private readonly previews: Previews,
  ) {
    this.disposables.push(
      panel.onDidDispose(() => this.dispose()),
      panel.webview.onDidReceiveMessage((m: FromPreview) => this.receive(m)),
      vscode.workspace.onDidChangeTextDocument((e) => {
        if (e.document.uri.toString() === uri.toString()) this.schedule();
      }),
      vscode.window.onDidChangeTextEditorVisibleRanges((e) => {
        if (e.textEditor.document.uri.toString() !== uri.toString() || Date.now() < this.quietUntil) return;
        const first = e.visibleRanges[0];
        if (first) void this.post({ type: "scroll", line: first.start.line });
      }),
    );
  }

  post(message: ToPreview): Thenable<boolean> {
    return this.panel.webview.postMessage(message);
  }

  /** Renders the page again, first or after a change. */
  async load(): Promise<void> {
    const document = await vscode.workspace.openTextDocument(this.uri);
    const webview = this.panel.webview;
    const ext = this.previews.extensionUri;
    const folder = vscode.Uri.joinPath(this.uri, "..");
    webview.options = {
      enableScripts: true,
      enableCommandUris: false,
      localResourceRoots: [folder, ...(vscode.workspace.workspaceFolders ?? []).map((f) => f.uri), vscode.Uri.joinPath(ext, "out"), vscode.Uri.joinPath(ext, "media")],
    };
    this.panel.title = `Preview ${baseName(this.uri)}`;
    webview.html = page({
      cspSource: webview.cspSource,
      nonce: nonce(),
      base: `${webview.asWebviewUri(folder).toString()}/`,
      style: webview.asWebviewUri(vscode.Uri.joinPath(ext, "media", "preview.css")).toString(),
      script: webview.asWebviewUri(vscode.Uri.joinPath(ext, "out", "preview.js")).toString(),
      title: baseName(this.uri),
      body: await this.previews.render(document),
      version: document.version,
    });
  }

  private schedule(): void {
    if (this.timer !== undefined) clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      this.timer = undefined;
      void this.update();
    }, DEBOUNCE_MS);
  }

  private async update(): Promise<void> {
    const ticket = ++this.rendering;
    const document = vscode.workspace.textDocuments.find((d) => d.uri.toString() === this.uri.toString());
    if (!document) return;
    const html = await this.previews.render(document);
    // A newer render may have finished first.
    if (ticket !== this.rendering) return;
    this.latest = { html, version: document.version };
    void this.post({ type: "update", ...this.latest });
  }

  private editor(): vscode.TextEditor | undefined {
    return vscode.window.visibleTextEditors.find((e) => e.document.uri.toString() === this.uri.toString());
  }

  private receive(message: FromPreview): void {
    this.messages.push(message);
    if (this.messages.length > 200) this.messages.splice(0, this.messages.length - 100);
    switch (message.type) {
      case "ready": {
        if (this.latest) void this.post({ type: "update", ...this.latest });
        const first = this.editor()?.visibleRanges[0];
        if (first && first.start.line > 0) void this.post({ type: "scroll", line: first.start.line });
        break;
      }
      case "reveal": {
        const editor = this.editor();
        if (!editor) break;
        const line = Math.max(0, Math.min(editor.document.lineCount - 1, Math.floor(message.line)));
        this.quietUntil = Date.now() + QUIET_MS;
        editor.revealRange(new vscode.Range(line, 0, line, 0), vscode.TextEditorRevealType.AtTop);
        break;
      }
      case "open":
        void this.open(message.href);
        break;
    }
  }

  /** Opens a clicked link: the web outside, files in VS Code. */
  private async open(href: string): Promise<void> {
    const target = linkTarget(href);
    if (!target || target.kind === "fragment") return;
    if (target.kind === "external") {
      await vscode.env.openExternal(vscode.Uri.parse(target.uri, true));
      return;
    }
    const root = vscode.workspace.getWorkspaceFolder(this.uri)?.uri;
    const uri =
      target.path === ""
        ? this.uri
        : target.from === "document"
          ? vscode.Uri.joinPath(this.uri, "..", target.path)
          : target.from === "workspace" && root
            ? vscode.Uri.joinPath(root, target.path)
            : vscode.Uri.file(target.path);
    const line = fragmentLine(target.fragment);
    const options: vscode.TextDocumentShowOptions = { viewColumn: this.editor()?.viewColumn ?? vscode.ViewColumn.Active };
    if (line !== undefined) options.selection = new vscode.Range(line, 0, line, 0);
    await vscode.commands.executeCommand("vscode.open", uri, options);
  }

  dispose(): void {
    if (this.timer !== undefined) clearTimeout(this.timer);
    this.previews.forget(this);
    for (const d of this.disposables.splice(0)) d.dispose();
  }
}

/** The open previews, one per document. */
export class Previews implements vscode.Disposable {
  private readonly open = new Map<string, Preview>();
  private grammar: Promise<Grammar | undefined> | undefined;

  constructor(readonly extensionUri: vscode.Uri) {}

  /** Opens or shows the preview of `uri`, beside its editor or in its place. */
  async show(uri: vscode.Uri, beside: boolean): Promise<PreviewHook> {
    const column = beside ? vscode.ViewColumn.Beside : (vscode.window.activeTextEditor?.viewColumn ?? vscode.ViewColumn.Active);
    const existing = this.open.get(uri.toString());
    if (existing) {
      existing.panel.reveal(column, beside);
      return existing;
    }
    const panel = vscode.window.createWebviewPanel(VIEW_TYPE, `Preview ${baseName(uri)}`, { viewColumn: column, preserveFocus: beside }, { enableScripts: true, enableFindWidget: true });
    const preview = new Preview(uri, panel, this);
    this.open.set(uri.toString(), preview);
    await preview.load();
    return preview;
  }

  /** The preview of `uri`, if it's open. */
  get(uri: vscode.Uri): PreviewHook | undefined {
    return this.open.get(uri.toString());
  }

  forget(preview: Preview): void {
    if (this.open.get(preview.uri.toString()) === preview) this.open.delete(preview.uri.toString());
  }

  /** The document as preview HTML, by VS Code's Markdown engine. */
  async render(document: vscode.TextDocument): Promise<string> {
    let html: string | undefined;
    try {
      html = await vscode.commands.executeCommand<string>("markdown.api.render", document);
    } catch (error) {
      const why = error instanceof Error ? error.message : String(error);
      return `<p class="code-line" data-line="0">The preview needs VS Code's built-in Markdown extension (Markdown Language Features), enabled: ${escape(why)}</p>`;
    }
    return body(typeof html === "string" ? html : "", await this.loadGrammar(), document.lineCount, document.getText());
  }

  private loadGrammar(): Promise<Grammar | undefined> {
    this.grammar ??= Promise.resolve(vscode.workspace.fs.readFile(vscode.Uri.joinPath(this.extensionUri, "syntaxes", "temper.tmLanguage.json"))).then(
      (bytes) => JSON.parse(new TextDecoder().decode(bytes)) as Grammar,
      () => undefined,
    );
    return this.grammar;
  }

  dispose(): void {
    for (const p of [...this.open.values()]) p.panel.dispose();
  }
}

/** Registers the preview's commands. */
export function registerPreview(context: vscode.ExtensionContext): Previews {
  const previews = new Previews(context.extensionUri);
  const command = (beside: boolean) => async (uri?: vscode.Uri) => {
    const target = uri instanceof vscode.Uri ? uri : vscode.window.activeTextEditor?.document.uri;
    if (target) await previews.show(target, beside);
  };
  context.subscriptions.push(
    previews,
    vscode.commands.registerCommand("temper.showPreview", command(false)),
    vscode.commands.registerCommand("temper.showPreviewToSide", command(true)),
  );
  return previews;
}

function baseName(uri: vscode.Uri): string {
  return uri.path.slice(uri.path.lastIndexOf("/") + 1);
}

function nonce(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

const escape = (text: string): string => text.replace(/[&<>"]/g, (c) => `&#${c.charCodeAt(0)};`);
