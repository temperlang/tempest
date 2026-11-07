// Finds the `tempest` that runs the language server: the
// `temper.server.path` setting if set, then the binary bundled in a
// platform-specific package (bin/tempest), then `tempest` on PATH.
// Plain Node, so its test runs without VS Code.

import { existsSync, statSync } from "fs";
import * as path from "path";

export interface Found {
  command: string;
  /** Where it came from, for the log and messages. */
  from: "setting" | "bundled" | "PATH";
}

export interface Where {
  /** The `temper.server.path` setting; empty if unset. */
  setting: string;
  /** The extension's own directory. */
  extensionDir: string;
  platform: NodeJS.Platform;
  env: Record<string, string | undefined>;
}

const isFile = (file: string): boolean => {
  try {
    return statSync(file).isFile();
  } catch {
    return false;
  }
};

/** The executable's file names on a platform: `tempest.exe` and such on Windows. */
function names(platform: NodeJS.Platform, env: Where["env"]): string[] {
  if (platform !== "win32") return ["tempest"];
  const exts = (env.PATHEXT ?? ".EXE;.CMD;.BAT").split(";").filter(Boolean);
  return exts.map((ext) => `tempest${ext.toLowerCase()}`);
}

export function findServer({ setting, extensionDir, platform, env }: Where): Found | undefined {
  if (setting.trim()) return { command: setting.trim(), from: "setting" };
  const bundled = path.join(extensionDir, "bin", platform === "win32" ? "tempest.exe" : "tempest");
  if (existsSync(bundled)) return { command: bundled, from: "bundled" };
  const separator = platform === "win32" ? ";" : ":";
  // Windows spells it `Path`, and Node's env there ignores case, but a
  // plain object doesn't.
  const dirs = (env.PATH ?? env.Path ?? "").split(separator).filter(Boolean);
  for (const dir of dirs) {
    for (const name of names(platform, env)) {
      const file = path.join(dir, name);
      if (isFile(file)) return { command: file, from: "PATH" };
    }
  }
  return undefined;
}

/** What to say when there's no `tempest` anywhere. */
export function notFound(platform: string, arch: string): string {
  return (
    `Temper: no tempest language server found. This copy of the extension has no tempest ` +
    `bundled for ${platform}-${arch}, and there's no tempest on PATH. Install the Temper ` +
    `package for your platform, put tempest on PATH, or set temper.server.path to its path.`
  );
}
