// The Temper extension: starts `tempest lsp` and connects VS Code to it
// with vscode-languageclient, which syncs documents (incrementally, as
// the server asks), shows its diagnostics, and maps VS Code's providers
// to LSP's requests.

import * as vscode from "vscode";
import { LanguageClient, type LanguageClientOptions, type ServerOptions } from "vscode-languageclient/node";
import { type Found, findServer, notFound } from "./server";

let client: LanguageClient | undefined;
let server: Found | undefined;

/** What the extension gives other extensions, and its tests. */
export interface Api {
  /** The `tempest` running the server, if one was found. */
  server(): Found | undefined;
  /** The running client, once started. */
  client(): LanguageClient | undefined;
}

async function start(context: vscode.ExtensionContext, output: vscode.LogOutputChannel): Promise<void> {
  const config = vscode.workspace.getConfiguration("temper");
  server = findServer({
    setting: config.get<string>("server.path", ""),
    extensionDir: context.extensionPath,
    platform: process.platform,
    env: process.env,
  });
  if (!server) {
    const message = notFound(process.platform, process.arch);
    output.appendLine(message);
    void vscode.window.showErrorMessage(message);
    return;
  }
  const args = ["lsp", ...config.get<string[]>("server.args", [])];
  output.appendLine(`Starting ${server.command} ${args.join(" ")} (${server.from})`);
  const folder = vscode.workspace.workspaceFolders?.[0];
  const serverOptions: ServerOptions = {
    command: server.command,
    args,
    options: folder ? { cwd: folder.uri.fsPath } : {},
  };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ language: "temper" }, { language: "temper-markdown" }],
    outputChannel: output,
  };
  client = new LanguageClient("temper", "Temper", serverOptions, clientOptions);
  try {
    await client.start();
  } catch (error) {
    const message = `Temper: couldn't start ${server.command} (${String(error)}). Set temper.server.path to tempest's path.`;
    output.appendLine(message);
    void vscode.window.showErrorMessage(message);
  }
}

async function stop(): Promise<void> {
  const running = client;
  client = undefined;
  if (running?.isRunning()) await running.stop();
}

export async function activate(context: vscode.ExtensionContext): Promise<Api> {
  const output = vscode.window.createOutputChannel("Temper", { log: true });
  const restart = async (): Promise<void> => {
    await stop();
    await start(context, output);
  };
  context.subscriptions.push(
    output,
    vscode.commands.registerCommand("temper.restartServer", restart),
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("temper.server")) void restart();
    }),
  );
  await start(context, output);
  return { server: () => server, client: () => client };
}

export async function deactivate(): Promise<void> {
  await stop();
}
