// Tests for finding tempest (server.ts), under Node's test runner.

import assert from "node:assert/strict";
import { chmodSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import * as path from "node:path";
import { test } from "node:test";
import { findServer, notFound } from "./server";

function scratch(): string {
  return mkdtempSync(path.join(tmpdir(), "temper-vscode-"));
}

function touch(file: string): void {
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, "");
  chmodSync(file, 0o755);
}

test("the setting comes first, then the bundled binary, then PATH", () => {
  const dir = scratch();
  try {
    const ext = path.join(dir, "ext");
    const bin = path.join(dir, "path-bin");
    touch(path.join(bin, "tempest"));
    const env = { PATH: `/nowhere:${bin}` };
    const where = { setting: "", extensionDir: ext, platform: "linux" as const, env };
    assert.deepEqual(findServer(where), { command: path.join(bin, "tempest"), from: "PATH" });
    touch(path.join(ext, "bin", "tempest"));
    assert.deepEqual(findServer(where), { command: path.join(ext, "bin", "tempest"), from: "bundled" });
    assert.deepEqual(findServer({ ...where, setting: " /my/tempest " }), { command: "/my/tempest", from: "setting" });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("Windows names it tempest.exe, and spells PATH as Path", () => {
  const dir = scratch();
  try {
    touch(path.join(dir, "tempest.exe"));
    const where = { setting: "", extensionDir: path.join(dir, "none"), platform: "win32" as const };
    const found = findServer({ ...where, env: { Path: `C:\\nowhere;${dir}`, PATHEXT: ".COM;.EXE" } });
    assert.deepEqual(found, { command: path.join(dir, "tempest.exe"), from: "PATH" });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("nothing found says what to do", () => {
  const where = { setting: "", extensionDir: "/nowhere", platform: "linux" as const, env: { PATH: "/nowhere" } };
  assert.equal(findServer(where), undefined);
  const message = notFound("linux", "x64");
  assert.match(message, /linux-x64/);
  assert.match(message, /temper\.server\.path/);
});
