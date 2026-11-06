// The example picker's examples, from ../examples/, bundled as text.
// Each starts with a `// Title` comment, which names it.

import hello from "../examples/1-hello.temper";
import interpolation from "../examples/2-interpolation.temper";
import genericClass from "../examples/3-generic-class.temper";
import asyncAwait from "../examples/4-async.temper";
import generator from "../examples/5-generator.temper";
import tests from "../examples/6-tests.temper";

export interface Example {
  title: string;
  source: string;
}

export const examples: Example[] = [hello, interpolation, genericClass, asyncAwait, generator, tests].map(
  (source) => ({ title: /^\/\/ (.*)/.exec(source)?.[1] ?? "Example", source }),
);
