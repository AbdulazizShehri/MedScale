/**
 * Verify the site never inflates model discovery marketing statistics.
 * All rows in the source catalog reference third-party Hugging Face repos.
 */
import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";

const catalog = path.resolve(process.cwd(), "../third_party/medscale-model-catalog-v3/models.jsonl");
const statsPath = path.resolve(process.cwd(), "src/lib/catalog-stats.ts");
const rows = fs.readFileSync(catalog, "utf8").split(/\r?\n/).filter(Boolean);
const unique = new Set(), languages = new Set(), families = new Set();
for (const row of rows) {
  const entry = JSON.parse(row);
  assert.ok(typeof entry.repo_id === "string" && entry.repo_id.includes("/"), "catalog requires an external repo id");
  assert.ok(!unique.has(entry.repo_id), "catalog entry IDs must be unique");
  unique.add(entry.repo_id);
  assert.ok(typeof entry.family === "string" && entry.family.length, "model family required");
  families.add(entry.family);
  assert.ok(Array.isArray(entry.languages), "model language tags required");
  for (const lang of entry.languages) {
    assert.ok(typeof lang === "string" && lang.length, "malformed language tag");
    languages.add(lang);
  }
}
const declared = fs.readFileSync(statsPath, "utf8");
for (const [field,value] of Object.entries({modelRecords:unique.size,languageTags:languages.size,modelFamilies:families.size})) {
  assert.ok(new RegExp(`\\b${field}\\s*:\\s*${value}\\b`).test(declared), `Published catalog metric ${field} no longer matches source (${value}); update before shipping`);
}
console.log(`PASS: marketing catalog metrics verified: ${unique.size} external repo entries, ${languages.size} language tags, ${families.size} families`);
