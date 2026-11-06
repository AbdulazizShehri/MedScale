// Recounts every number the site shows from the public file it comes from.
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(process.cwd(), "..");
const declared = fs.readFileSync("src/lib/proof.ts", "utf8");
const value = (field) => {
  const match = declared.match(new RegExp(`\\b${field}: (\\d+)`));
  assert.ok(match, `proof.ts declares ${field}`);
  return Number(match[1]);
};

const rows = fs.readFileSync(path.join(root, "third_party/medscale-model-catalog-v3/models.jsonl"), "utf8").split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line));
const ids = new Set(rows.map((row) => row.repo_id));
assert.equal(ids.size, rows.length, "catalog entries are unique");
assert.equal(value("catalogModels"), ids.size, "catalog model count");
assert.equal(value("catalogFamilies"), new Set(rows.map((row) => row.family)).size, "catalog family count");
assert.equal(value("catalogLanguages"), new Set(rows.flatMap((row) => row.languages)).size, "catalog language count");
assert.ok(rows.every((row) => row.repo_id.startsWith("OpenMed/")), "the site credits OpenMed for every row, so a new publisher needs new wording");

const matrix = fs.readFileSync(path.join(root, "specs/103-local-model-catalog/CAPABILITY_MATRIX.md"), "utf8");
assert.ok(matrix.includes(`Executed or expected runnable: **${value("catalogRunnable").toLocaleString("en-US")} of ${ids.size.toLocaleString("en-US")}**`), "runnable count matches the capability matrix");

assert.equal(value("specs"), fs.readdirSync(path.join(root, "specs"), { withFileTypes: true }).filter((entry) => entry.isDirectory()).length, "spec count");

let tests = 0;
const walk = (dir) => {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) { if (entry.name !== "target") walk(full); }
    else if (entry.name.endsWith(".rs")) tests += (fs.readFileSync(full, "utf8").match(/#\[(tokio::)?test\]/g) ?? []).length;
  }
};
walk(path.join(root, "crates"));
walk(path.join(root, "apps/desktop-tauri/src-tauri/src"));
assert.ok(tests >= value("rustTestsFloor"), `found ${tests} Rust tests`);

console.log(`PASS: site numbers match their sources: ${ids.size} models, ${value("catalogLanguages")} languages, ${value("catalogFamilies")} families, ${value("specs")} specs, ${tests} Rust tests`);
