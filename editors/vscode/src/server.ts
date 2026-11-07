// Finds the `tempest` that runs the language server: the
// `temper.server.path` setting if set, then the binary bundled in a
// platform-specific package (bin/tempest), then `tempest` on PATH; and
// what to say when there's none, or it doesn't start.
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

/** Where tempest was found, if it was, and each place looked, for messages. */
export interface Search {
  found?: Found;
  tried: string[];
}

/** A file named `name`, or one of its Windows spellings, in a PATH directory. */
function onPath(name: string, { platform, env }: Where): string | undefined {
  const separator = platform === "win32" ? ";" : ":";
  // Windows spells it `Path`, and Node's env there ignores case, but a
  // plain object doesn't.
  const dirs = (env.PATH ?? env.Path ?? "").split(separator).filter(Boolean);
  const spellings = name === "tempest" ? names(platform, env) : [name];
  for (const dir of dirs) {
    for (const spelling of spellings) {
      const file = path.join(dir, spelling);
      if (isFile(file)) return file;
    }
  }
  return undefined;
}

/**
 * Looks for tempest: the setting alone if it's set (a path, which must be
 * a file, or a name, looked for on PATH), else the bundled binary, then
 * PATH. Only a file that's there is found, so a missing tempest is said
 * plainly rather than failing to spawn.
 */
export function search(where: Where): Search {
  const setting = where.setting.trim();
  if (setting) {
    const isPath = setting.includes("/") || setting.includes("\\");
    const command = isPath ? (isFile(setting) ? setting : undefined) : onPath(setting, where);
    if (command) return { found: { command, from: "setting" }, tried: [] };
    return { tried: [`temper.server.path, ${setting}${isPath ? "" : ", on PATH"}`] };
  }
  const bundled = path.join(where.extensionDir, "bin", where.platform === "win32" ? "tempest.exe" : "tempest");
  if (existsSync(bundled)) return { found: { command: bundled, from: "bundled" }, tried: [] };
  const command = onPath("tempest", where);
  if (command) return { found: { command, from: "PATH" }, tried: [] };
  return { tried: [`the bundled ${bundled}`, "tempest on PATH"] };
}

export function findServer(where: Where): Found | undefined {
  return search(where).found;
}

/** How to get a working tempest. */
const fix = (platform: string, arch: string): string =>
  `Install the Temper extension's package for ${platform}-${arch}, which bundles tempest, ` +
  `put tempest on PATH, or set Temper: Server Path (temper.server.path) to tempest's path.`;

/** What to say when there's no `tempest` where it looked. */
export function notFound(tried: string[], platform: string, arch: string): string {
  return `Temper: no tempest language server found (tried ${tried.join("; ")}). ${fix(platform, arch)}`;
}

/** What to say when tempest was found but didn't start. */
export function notStarted(server: Found, error: string, platform: string, arch: string): string {
  return `Temper: tempest (${server.command}, from ${server.from}) didn't start: ${error}. ${fix(platform, arch)}`;
}
