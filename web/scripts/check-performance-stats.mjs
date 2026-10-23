// Fail site CI when headline speed figures diverge from reproducible evidence.
import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";

const root = path.resolve(process.cwd(), "..");
const read = (p) => fs.readFileSync(path.join(root,p),"utf8");
const search = JSON.parse(read("evidence/045-lexical-10k-corpus/perf_harness_delivery_plan_scale.json"));
const timeline = JSON.parse(read("evidence/027-perf-sbom-release-evidence/perf_harness_latest.json"));
const csv = read("evidence/075-data-source-fabric/SCALE_MEASUREMENTS.md");
const stat = read("web/src/lib/performance-stats.ts");
function expectNumber(field, value) {
  assert.ok(Number.isFinite(value), "evidence must contain finite numeric values");
  assert.ok(new RegExp(field + "\\s*:\\s*" + String(value).replace(".", "\\.") + "\\b").test(stat),
    "Advertised " + field + " does not match bound benchmark evidence: " + value);
}
assert.equal(search.binding.fixture_identity.lexical_doc_count, 10000, "expected 10,000-document test");
assert.equal(search.methodology.timed_runs, 3);
assert.equal(search.methodology.synthetic_only, true);
assert.equal(search.budgets_claimed_met, false);
expectNumber("p95Ms", search.results_ms.lexical_search.p95);
expectNumber("syntheticDocuments", search.binding.fixture_identity.lexical_doc_count);
assert.equal(timeline.binding.fixture_identity.timeline_events, 64);
assert.equal(timeline.methodology.timed_runs, 30);
assert.equal(timeline.methodology.synthetic_only, true);
assert.equal(timeline.budgets_claimed_met, false);
const timelineSlice = stat.slice(stat.indexOf("timeline64:"));
const advertisedTimeline = timelineSlice.match(/p95Ms:\\s*([0-9.]+)/);
assert.ok(advertisedTimeline && Math.abs(Number(advertisedTimeline[1]) - timeline.results_ms.timeline_projection.p95) < 0.001,
  "Advertised timeline speed must match evidence to the published precision");
assert.ok(timelineSlice.includes("syntheticEvents: 64"), "Timeline scale mismatch");
assert.match(csv, /20,000 deterministic rows/, "20K synthetic row fixture is absent");
expectNumber("syntheticRows", 20000);
assert.ok(stat.includes("samples: 3") && stat.includes("samples: 30"), "Run counts must stay explicit");
console.log("PASS: marketing performance metrics are sourced from pinned engineering evidence");
