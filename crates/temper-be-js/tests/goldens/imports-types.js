import { ExportedClass } from "./exporter.js";

const x = new ExportedClass(42);
console.log(`foo = ${x.foo}`);
