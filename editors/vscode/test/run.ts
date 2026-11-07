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
  };
  try {
    mkdirSync(workspace);
    for (const [name, text] of Object.entries(files)) writeFileSync(path.join(workspace, name), text);
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
