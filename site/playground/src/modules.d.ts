// Modules the bundler supplies.

/** Example sources, as text (esbuild's `text` loader). */
declare module "*.temper" {
  const source: string;
  export default source;
}

/** The JavaScript backend's runtime, from vendor/temper/be-js. */
declare module "@temperlang/core" {
  const exports: Record<string, unknown>;
  export = exports;
}

/** Facts about the build, from build.py. */
declare const BUILD: {
  commit: string;
  wasmBytes: number;
  wasmGzipBytes: number;
};
