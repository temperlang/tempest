// Runs test/suite.ts in a real VS Code, downloaded by
// @vscode/test-electron from VS Code's update servers, on a fresh library
// workspace, with the extension at the path given (a platform package,
// unpacked).
//
//   node out/test/run.js EXTENSION_DIR CACHE_DIR SUITE VERSION [VS_CODE_ARGS...]

import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import * as path from "node:path";
import { runTests } from "@vscode/test-electron";

const [extensionDir, cachePath, suite, version, ...extra] = process.argv.slice(2);

async function main(extensionDir: string, cachePath: string, suite: string, version: string): Promise<void> {
  const scratch = mkdtempSync(path.join(tmpdir(), "temper-vscode-test-"));
  const workspace = path.join(scratch, "shapes");
  const files: Record<string, string> = {
    "config.temper.md": "# Shapes\n",
    "shapes.temper": "/** A square's area. */\nexport let area(side: Int): Int { side * side }\n",
    "use.temper": "let a = area(3);\nlet b: String = a;\n",
    // For the preview: a heading, Temper fenced and indented, an image
    // beside it, and enough after to scroll.
    "notes.temper.md": [
      "# Notes on shapes",
      "",
      "![a dot](dot.svg)",
      "",
      "```temper",
      'let noteName = "square";',
      "```",
      "",
      "    let noteSide = 2;",
      "",
      ...Array.from({ length: 60 }, (_, i) => `## Section ${i + 1}\n\nAbout section ${i + 1}.\n`),
    ].join("\n"),
    "dot.svg": '<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><circle cx="4" cy="4" r="4"/></svg>\n',
  };
  try {
    mkdirSync(workspace);
    for (const [name, text] of Object.entries(files)) writeFileSync(path.join(workspace, name), text);
    // VS Code's built-in AI features, which `--disable-extensions` leaves
    // on, log for a GitHub sign-in again and again; off, the log is ours.
    const user = path.join(scratch, "user-data", "User");
    mkdirSync(user, { recursive: true });
    writeFileSync(path.join(user, "settings.json"), JSON.stringify({ "chat.disableAIFeatures": true }));
    await runTests({
      version,
      cachePath,
      extensionDevelopmentPath: extensionDir,
      extensionTestsPath: suite,
      launchArgs: [
        workspace,
        "--disable-extensions",
        "--disable-workspace-trust",
        "--skip-welcome",
        "--skip-release-notes",
        "--user-data-dir",
        path.join(scratch, "user-data"),
        ...extra,
      ],
    });
  } catch (error) {
    console.error(`VS Code tests failed: ${String(error)}`);
    process.exitCode = 1;
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

if (!extensionDir || !cachePath || !suite || !version) {
  console.error("usage: run.js EXTENSION_DIR CACHE_DIR SUITE VERSION [VS_CODE_ARGS...]");
  process.exit(2);
}
void main(extensionDir, cachePath, suite, version);
