/**
 * Layout check of the built front end. Captures every route in both themes and fails
 * if page content leaves the page edge or the app overflows. IPC is simulated with
 * made-up synthetic data, so this says nothing about Core, native rendering or
 * clinical content, and its images are not product screenshots.
 * Usage: node qa/visual-routes.mjs [out-dir]   (after `vite build`)
 */
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const dist = path.join(path.resolve(path.dirname(fileURLToPath(import.meta.url)), ".."), "dist");
const outDir = process.argv[2];
const ROUTES = ["Home", "Patients", "Documents", "Insights", "Evidence", "Projects", "Data", "Browse", "Analytics", "Knowledge", "Research OS", "Models", "MedAgent", "Model Fleet", "Workflows", "Tasks", "Messages", "Collaboration", "Audio", "Privacy", "Audit Trail", "Exports", "Integrations", "Settings", "About"];
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".ttf": "font/ttf", ".png": "image/png" };

const mock = () => {
  const slots = ["present", "present", "conflict", "present", "absent", "unknown", "present", "unsupported_resource_type", "present"].map((status, i) => ({ concept_key: ["birth_date", "conditions", "medications", "allergies", "vitals", "labs", "encounters", "care_plan", "documents"][i], status, values: [], notes: [] }));
  const field = (field_key, value, status = "present") => ({ field_key, value, status, evidence_refs: ["src-1"] });
  const subject = (subject_ref, display_name, condition_summary, sources) => ({ subject_ref, display_name, birth_date: "1980-03-12", condition_summary, coverage_summary: "5 present · 1 conflict · 1 absent · 1 unknown · 1 unsupported", coverage_state: "Needs review", coverage_slots: slots, latest_event: "2019-12-01", event_count: 4, source_count: sources });
  const ada = subject("synthetic-subject-ada", "Ada Synthetic", "Asthma, persistent", 7);
  const R = {
    workspace_status: { open: true, version: "0.1.0", network_default_deny: true, local_only: true, egress: "Default deny, except a Whisper model download you approve and sites you allowlist in Browse", encrypted_vault_exists: false, real_phi_authorized: false, workspace: { kind: "synthetic", label: "Synthetic workspace", vault_id: "desktop-synthetic", subjects: 2, sources: 12 } },
    projects_list: [{ id: "p1", name: "Sample study", status: "active", revision: 1, experiments: 0, refs: 0, edges: 0 }],
    patients_list: { scope: "Subjects promoted into this vault by this desktop", subjects: [ada, subject("synthetic-subject-coverage", "NoBirth", "Synthetic example", 3)] },
    patient_detail: { ...ada, timeline: [], labs: [], medications_state: "conflict", documents_state: "present", care_plan_state: "absent", sources: [{ source_id: "src-1", resource_type: "Patient", fixture: "patient-golden.json", claim_kind: "structured" }, { source_id: "src-8", resource_type: "Observation", fixture: "observation-hr.json", claim_kind: "structured" }], contracts: { brief: { sections: { identity: [field("display_name", "Ada Synthetic"), field("birth_date", "1980-03-12")], vitals: [], conditions: [field("primary_condition", "Asthma, persistent")], coverage_summary: {} }, built_rules_version: { extractors: [], order_rules: "", brief_schema: "" } }, coverage: { slots } } },
    evidence_corpus: { corpus_id: "synthetic-lexical", version: "1.0.0", source_identity: "synthetic-lexical@1.0.0", content_digest: "88605d138c0a1f2e3d4c5b6a79880011", rights: "synthetic fixture", rights_uri: "fixture://rights", documents: [{ doc_id: "doc-asthma", text: "Synthetic asthma inhaler note.", content_digest: "88605d138c0a1f2e3d4c5b6a79880011", status: "current", freshness_marker: "fixture_current", conflict_marker: null }, { doc_id: "doc-conflict", text: "Two synthetic notes disagree about a dose.", content_digest: "1f2e3d4c5b6a79880011223344556677", status: "current", freshness_marker: null, conflict_marker: "c1" }, { doc_id: "doc-retracted", text: "Synthetic statin note retracted for the evidence lifecycle demo.", content_digest: "aa11bb22cc33dd44ee55ff6677889900", status: "retracted", freshness_marker: null, conflict_marker: null }] },
    models_overview: { models: [] },
    workflow_overview: { synthetic_only: true, workflow_name: "Sample review workflow", workflow_state: "Waiting for explicit review", outbox_summary: "Nothing sent", action_boundary: "Proposal only", task_boundary: "Bound to a payload digest", message_boundary: "Previews, not receipts", outbox_entries: 0, steps: [{ order: "01", title: "Prepare action", detail: "A read-only proposal.", state: "read-only" }, { order: "02", title: "Review", detail: "A person reviews the proposal.", state: "review required" }], tasks: [{ action_id: "action-1", title: "Review sample summary", state: "review required", payload_digest: "5d41402abc4b2a76", next_step: "Inspect the payload", reconcile_required: false }], messages: [{ title: "Sample preview", body: "Preview text.", status: "preview", related_action_id: "action-1" }] },
    documents_list: { document_understanding: "Structured extraction from synthetic FHIR fixtures.", sources: [["Patient", "patient-golden.json"], ["MedicationRequest", "unsupported-medicationrequest.json"]].map(([resource_type, fixture], i) => ({ source_id: `src-${i + 1}`, resource_type, fixture, claim_kind: "structured", subject_ref: "synthetic-subject-ada" })) },
    insights_overview: { cohort_size: 2, review_attention_count: 2, evidence_gap_count: 4, present_count: 7, synthetic_only: true, condition_distribution: [{ label: "active", value: "2", detail: "" }], coverage_distribution: [{ label: "Present", value: "7", detail: "Supported trusted presentation" }, { label: "Conflict", value: "5", detail: "Unresolved conflicting evidence" }], cohorts: [{ subject_ref: "synthetic-subject-ada", display_name: "Ada Synthetic", condition: "active", evidence_state: "Conflict, unresolved", review_attention: "Review required" }], evidence: [], risk_state: "not computed", trend_state: "not computed", gap_state: "listed", recommendation_state: "none", assistant_default: "abstain" },
    governance_overview: { synthetic_only: true, audit_boundary: "Read-only", export_boundary: "Disclosed", settings_boundary: "Local", integrations_boundary: "Default deny", audit_rows: [{ record_id: "disclosure-1", action: "sample interchange", subject: "synthetic-subject-ada", detail: "7 resources", status: "Synthetic disclosure" }], export_rows: [], settings_rows: [{ label: "Real PHI", value: "Not authorized", detail: "Synthetic fixtures only.", status: "Fail closed" }], integration_rows: [], missing_release_evidence: "Release evidence is not complete.", fhir_support: null },
    about_info: { product: "MedScale", tagline: "Clinical Intelligence OS", version: "0.1.0", shell: "Tauri 2", core: "Rust Core Host, in-process", local_only: true, network_default_deny: true, license: "Apache-2.0", build_profile: "release", target: "windows-x86_64", release_ready: false },
  };
  window.__TAURI_INTERNALS__ = { metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main", windowLabel: "main" } }, transformCallback: () => 0, invoke: async (cmd) => { if (cmd in R) return R[cmd]; throw { kind: "unavailable", message: `Layout check has no data for ${cmd}` }; } };
};

function audit() {
  const textLeft = (el) => { const w = document.createTreeWalker(el, NodeFilter.SHOW_TEXT, { acceptNode: (n) => (n.textContent.trim() ? 1 : 2) }); const n = w.nextNode(); if (!n) return null; const r = document.createRange(); r.selectNodeContents(n); return r.getBoundingClientRect().left; };
  const edge = textLeft(document.querySelector(".crumb")), misses = [];
  for (const q of [".pane h1:not(.sr-only)", ".pane .tb th:first-child", ".pane .tb td:first-child", ".pane .page-head", ".pane > .notice", ".pane > .section > .sec"]) {
    for (const el of document.querySelectorAll(q)) {
      if (el.closest('[style*="grid-template-columns"], .grid-2, .grid-3, .inspector')) continue;
      const x = q.includes("th") ? textLeft(el) : el.getBoundingClientRect().left;
      if (x !== null && Math.abs(x - edge) > 0.75) misses.push(`${q} at ${x.toFixed(1)}, page edge ${edge.toFixed(1)}`);
    }
  }
  const sizes = new Set([...document.querySelectorAll('.pane .btn:not(.lg):not([style*="height"])')].map((b) => Math.round(b.getBoundingClientRect().height)));
  if (sizes.size > 1) misses.push(`button heights ${[...sizes].join(", ")}`);
  if (document.documentElement.scrollWidth > innerWidth) misses.push("page overflow");
  return misses;
}

const server = http.createServer((req, res) => { const u = req.url.split("?")[0]; const p = path.join(dist, u === "/" ? "index.html" : u); fs.readFile(p, (err, data) => { res.writeHead(err ? 404 : 200, { "Content-Type": types[path.extname(p)] ?? "application/octet-stream" }); res.end(err ? "" : data); }); });
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ channel: "msedge", headless: true });
if (outDir) fs.mkdirSync(outDir, { recursive: true });
let failures = 0;
for (const theme of ["dark", "light"]) for (const [width, height] of [[1440, 900], [1100, 720]]) {
  const context = await browser.newContext({ viewport: { width, height }, reducedMotion: "reduce" });
  const page = await context.newPage();
  const errors = []; page.on("pageerror", (e) => errors.push(e.message));
  await page.addInitScript(mock);
  for (const route of ROUTES) {
    errors.length = 0;
    await page.goto(`${origin}/#route=${encodeURIComponent(route)}&theme=${theme}`, { waitUntil: "networkidle" });
    await page.reload({ waitUntil: "networkidle" });
    await page.evaluate(() => document.fonts.ready.then(() => 1));
    await page.waitForTimeout(150);
    const misses = [...(await page.evaluate(audit)), ...errors.map((e) => `script error: ${e}`)];
    failures += misses.length;
    for (const miss of misses.slice(0, 4)) console.error(`${route} ${theme} ${width}: ${miss}`);
    if (outDir) await page.screenshot({ path: path.join(outDir, `${route.replace(/ /g, "_")}-${theme}-${width}.jpg`), type: "jpeg", quality: 82 });
  }
  await context.close();
}
await browser.close();
server.close();
console.log(`${failures ? "FAIL" : "PASS"}: ${ROUTES.length} routes, 2 themes, 2 widths, ${failures} misses (simulated IPC)`);
process.exitCode = failures ? 1 : 0;
