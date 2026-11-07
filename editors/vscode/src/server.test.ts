// Tests for finding tempest (server.ts), under Node's test runner.

import assert from "node:assert/strict";
import { chmodSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import * as path from "node:path";
import { test } from "node:test";
import { findServer, notFound, search } from "./server";

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
    touch(path.join(dir, "my", "tempest"));
    const mine = path.join(dir, "my", "tempest");
    assert.deepEqual(findServer({ ...where, setting: ` ${mine} ` }), { command: mine, from: "setting" });
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

test("nothing found says what was tried and what to do", () => {
  const where = { setting: "", extensionDir: "/nowhere", platform: "linux" as const, env: { PATH: "" } };
  const { found, tried } = search(where);
  assert.equal(found, undefined);
  assert.deepEqual(tried, [`the bundled ${path.join("/nowhere", "bin", "tempest")}`, "tempest on PATH"]);
  const message = notFound(tried, "linux", "x64");
  assert.match(message, /bin\/tempest; tempest on PATH/);
  assert.match(message, /linux-x64/);
  assert.match(message, /Temper: Server Path \(temper\.server\.path\)/);
});

test("a setting that isn't there isn't used, nor anything else", () => {
  const dir = scratch();
  try {
    const ext = path.join(dir, "ext");
    touch(path.join(ext, "bin", "tempest"));
    const where = { setting: "/no/tempest", extensionDir: ext, platform: "linux" as const, env: { PATH: "" } };
    assert.deepEqual(search(where), { tried: ["temper.server.path, /no/tempest"] });
    // A bare name is looked for on PATH.
    touch(path.join(dir, "bin", "my-tempest"));
    const named = { ...where, setting: "my-tempest" };
    assert.deepEqual(search({ ...named, env: { PATH: "" } }), { tried: ["temper.server.path, my-tempest, on PATH"] });
    const env = { PATH: path.join(dir, "bin") };
    assert.deepEqual(findServer({ ...named, env }), { command: path.join(dir, "bin", "my-tempest"), from: "setting" });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
