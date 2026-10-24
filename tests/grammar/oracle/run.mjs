// Test-only oracle runner for src/grammar (src/grammar T1, V1): runs the
// caveman grammar of cavemem 0.2.1 (MIT, Julius Brussee) over the corpus
// and writes the golden files the Rust tests compare byte for byte.
// Never part of the chert binary; the test suite reads only its output.
//
// usage: node run.mjs <chunk-T35V7EPZ.js> <corpus dir> <oracle dir>
//
// The bundle also imports better-sqlite3, so it is not imported whole: the
// grammar's own slice (lexicon, tokenize, expand, compress) is evaluated
// as it ships, after the bundle's sha256 matches the pin in ORACLE.md.
import { createHash } from "node:crypto";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

const PIN = "03ca2793d362a22902820f4d095988cd976da23b976d494b93dd413ea5029353";
const FROM = "// ../../packages/compress/src/lexicon.json";
const TO = "// ../../packages/compress/src/privacy.ts";
const INTENSITIES = ["lite", "full", "ultra"];

const [bundle, corpus, out] = process.argv.slice(2);
const bytes = readFileSync(bundle);
const sha = createHash("sha256").update(bytes).digest("hex");
if (sha !== PIN) throw new Error(`bundle sha256 ${sha}, pinned ${PIN}`);
const text = bytes.toString("utf8");
const slice = text.slice(text.indexOf(FROM), text.indexOf(TO));
const grammar = new Function(`${slice}\nreturn { compress, expand };`)();

for (const file of readdirSync(corpus).filter((f) => f.endsWith(".txt"))) {
  const input = readFileSync(join(corpus, file), "utf8");
  const name = basename(file, ".txt");
  for (const intensity of INTENSITIES) {
    const compressed = grammar.compress(input, { intensity });
    const stem = join(out, `${name}.${intensity}`);
    writeFileSync(`${stem}.compressed`, compressed);
    writeFileSync(`${stem}.expanded`, grammar.expand(compressed));
  }
}
