// Declarations for the few Node APIs the extension and its tests use, in
// place of @types/node.

declare namespace NodeJS {
  type Platform = "aix" | "android" | "darwin" | "freebsd" | "haiku" | "linux" | "openbsd" | "sunos" | "win32" | "cygwin" | "netbsd";
}

declare const process: {
  platform: NodeJS.Platform;
  arch: string;
  env: Record<string, string | undefined>;
  argv: string[];
  exitCode: number | undefined;
  exit(code?: number): never;
};

declare const console: {
  log(...values: unknown[]): void;
  error(...values: unknown[]): void;
};

declare function setTimeout(callback: () => void, ms: number): unknown;

declare module "fs" {
  export function existsSync(path: string): boolean;
  export function statSync(path: string): { isFile(): boolean };
  export function mkdtempSync(prefix: string): string;
  export function mkdirSync(path: string, options?: { recursive?: boolean }): void;
  export function writeFileSync(path: string, text: string): void;
  export function chmodSync(path: string, mode: number): void;
  export function rmSync(path: string, options?: { recursive?: boolean; force?: boolean }): void;
}
declare module "node:fs" {
  export * from "fs";
}

declare module "path" {
  export function join(...parts: string[]): string;
  export function dirname(path: string): string;
}
declare module "node:path" {
  export * from "path";
}

declare module "node:os" {
  export function tmpdir(): string;
}

declare module "node:test" {
  export function test(name: string, f: () => void | Promise<void>): void;
}

declare module "node:assert/strict" {
  interface Assert {
    (value: unknown, message?: string): asserts value;
    ok(value: unknown, message?: string): asserts value;
    equal<T>(actual: unknown, expected: T, message?: string): asserts actual is T;
    deepEqual(actual: unknown, expected: unknown, message?: string): void;
    match(text: string, pattern: RegExp, message?: string): void;
  }
  const assert: Assert;
  export default assert;
}
