/**
 * Synthetic-only post-sample scan of NORMAL Windows Tauri release mode.
 *
 * No WebDriver port, no real patient data, no full-disk scan. The synthetic
 * sample action is driven by CI-only keyboard input in a disposable runner.
 * The browser and app-owned synthetic workspace are reported separately.
 * Only fixed counters/status strings leave the process.
 */
import { lstat, readFile, mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { auditReleaseWebviewWelcomeRoot, isAuthorizedNativeRunner } from "./release-webview-runtime-audit.mjs";

const ROOT_NAME = "org.medscale.desktop.preview";
const MANIFEST_NAME = "synthetic-workspace.desktop-seed.json";

// Freeze the full Rust host.rs seed_plan() contract: two subjects, six
// fixture assertions each. A well-formed *partial* manifest is not a completed
// native GUI sample. Changes to the Rust seed plan must update this QA gate.
const EXPECTED_SEED_FIXTURES = Object.freeze({
  "synthetic-subject-ada": Object.freeze({
    "patient-golden.json": ["Patient", "patient"],
    "observation-hr.json": ["Observation", "observation"],
    "observation-weight.json": ["Observation", "observation"],
    "observation-temp-c.json": ["Observation", "observation"],
    "condition-uri.json": ["Condition", "condition"],
    "unsupported-medicationrequest.json": ["MedicationRequest", "medication"],
  }),
  "synthetic-subject-coverage": Object.freeze({
    "patient-absence.json": ["Patient", "patient"],
    "condition-uri.json": ["Condition", "condition"],
    "condition-conflict-b.json": ["Condition", "condition"],
    "observation-weight.json": ["Observation", "observation"],
    "observation-weight-conflict-unit.json": ["Observation", "observation"],
    "observation-unrecognized-unit.json": ["Observation", "observation"],
  }),
});
const EXPECTED_SEED_SUBJECTS = Object.keys(EXPECTED_SEED_FIXTURES);
const EXPECTED_SEED_SOURCE_COUNT = Object.values(EXPECTED_SEED_FIXTURES)
  .reduce((sum, fixtures) => sum + Object.keys(fixtures).length, 0);

// Fixed-status read-only diagnostics: never return any file name, subject ID,
// source ID or other parsed manifest value.
async function inspectSeedManifest(filename) {
  let st;
  try { st = await lstat(filename); }
  catch (error) {
    return { valid: false, status: error?.code === "ENOENT" ? "MISSING" : "UNREADABLE" };
  }
  if (!st.isFile() || st.isSymbolicLink() || st.size > 1024 * 1024 || st.size === 0) {
    return { valid: false, status: "INVALID_OR_BOUNDED_ENTRY" };
  }
  let manifest;
  try { manifest = JSON.parse((await readFile(filename)).toString("utf8")); }
  catch { return { valid: false, status: "INVALID_JSON_OR_UNREADABLE" }; }
  if (!manifest || typeof manifest !== "object" ||
      !Array.isArray(manifest.subjects) || !Array.isArray(manifest.sources)) {
    return { valid: false, status: "INVALID_SCHEMA" };
  }
  const subjects = manifest.subjects;
  if (subjects.length === 0) return { valid: false, status: "NO_SYNTHETIC_SUBJECT" };
  if (new Set(subjects).size !== subjects.length ||
      !subjects.every((subject) => typeof subject === "string" &&
        Object.hasOwn(EXPECTED_SEED_FIXTURES, subject))) {
    return { valid: false, status: "INVALID_SCHEMA" };
  }
  if (manifest.sources.length === 0) return { valid: false, status: "NO_SEEDED_SOURCES" };
  // Reject truncation, duplicate fixture rows, unknown fixtures and mismatched
  // Rust resource/claim types. Core may deduplicate ingest IDs; don't assume
  // each source_id is globally unique.
  const seen = new Set();
  let complete = subjects.length === EXPECTED_SEED_SUBJECTS.length &&
    manifest.sources.length === EXPECTED_SEED_SOURCE_COUNT;
  for (const source of manifest.sources) {
    if (!source || typeof source !== "object" ||
        !["source_id", "fixture", "resource_type", "claim_kind", "subject_ref"]
          .every((field) => typeof source[field] === "string" && source[field].trim().length > 0) ||
        !Object.hasOwn(EXPECTED_SEED_FIXTURES, source.subject_ref)) {
      complete = false;
      continue;
    }
    const expected = EXPECTED_SEED_FIXTURES[source.subject_ref][source.fixture];
    const key = JSON.stringify([source.subject_ref, source.fixture]);
    if (!expected || source.resource_type !== expected[0] ||
        source.claim_kind !== expected[1] || seen.has(key)) {
      complete = false;
      continue;
    }
    seen.add(key);
  }
  complete = complete && seen.size === EXPECTED_SEED_SOURCE_COUNT;
  return { valid: complete,
    status: complete ? "VERIFIED" : "INCOMPLETE_OR_UNLINKED_SOURCE" };
}

export async function auditNormalSyntheticSample(appDataRoot) {
  const browser = await auditReleaseWebviewWelcomeRoot(path.join(appDataRoot, "EBWebView"));
  const workspace = await auditReleaseWebviewWelcomeRoot(path.join(appDataRoot, "synthetic-workspace"));
  // The exact Tauri app-local root includes browser and synthetic vault, but
  // may contain OTHER app-owned files. Audit it independently to ensure no
  // canary remains in an unaccounted directory. No file paths are exported.
  const appData = await auditReleaseWebviewWelcomeRoot(appDataRoot);
  const manifestEvidence = await inspectSeedManifest(path.join(appDataRoot, MANIFEST_NAME));
  const knownManifestCanary = manifestEvidence.valid;
  // A seed manifest alone does not prove the sample workspace was populated;
  // an isolated workspace marker without a seed manifest is also incomplete.
  // Require BOTH independent expected app-owned synthetic artifacts.
  const syntheticSampleObserved = workspace.coverage_complete && workspace.runtime_observed &&
    workspace.synthetic_marker_files > 0 && knownManifestCanary;
  const unattributedMarkerFiles = appData.synthetic_marker_files -
    browser.synthetic_marker_files - workspace.synthetic_marker_files -
    Number(knownManifestCanary);
  const interpretation = !browser.coverage_complete || !workspace.coverage_complete ||
      !appData.coverage_complete ?
    "INCONCLUSIVE_INCOMPLETE_NATIVE_PROFILE_OR_SYNTHETIC_WORKSPACE" :
    !syntheticSampleObserved ? "INCONCLUSIVE_NO_SYNTHETIC_WORKSPACE_MARKERS" :
    appData.subject_name_marker_entries > 0 || appData.project_name_marker_entries > 0 ?
      "MARKER_IN_SCANNED_APP_DATA_ENTRY_NAME" :
    appData.project_key_marker_files > 0 ? "PROJECT_KEY_MARKER_IN_SCANNED_APP_DATA" :
    browser.synthetic_marker_files > 0 ? "SYNTHETIC_MARKERS_IN_NORMAL_RELEASE_WEBVIEW2_RUNTIME" :
    unattributedMarkerFiles !== 0 ? "INCONCLUSIVE_UNATTRIBUTED_APP_DATA_MARKERS" :
    "NO_SYNTHETIC_MARKERS_IN_SCANNED_NORMAL_RELEASE_POSTSAMPLE_RUNTIME";
  return {
    scope: "normal-release-InPrivate-synthetic-sample-after-exit; not PHI, OS, crash or backup clearance",
    app_data_coverage_complete: appData.coverage_complete,
    app_data_files_examined: appData.files_examined,
    app_data_synthetic_marker_files: appData.synthetic_marker_files,
    app_data_project_key_marker_files: appData.project_key_marker_files,
    app_data_subject_name_marker_entries: appData.subject_name_marker_entries,
    app_data_project_name_marker_entries: appData.project_name_marker_entries,
    unattributed_app_data_marker_files: unattributedMarkerFiles,
    browser_runtime_observed: browser.runtime_observed,
    browser_coverage_complete: browser.coverage_complete,
    browser_files_examined: browser.files_examined,
    browser_bytes_examined: browser.bytes_examined,
    browser_synthetic_marker_files: browser.synthetic_marker_files,
    browser_project_key_marker_files: browser.project_key_marker_files,
    browser_subject_name_marker_entries: browser.subject_name_marker_entries,
    browser_project_name_marker_entries: browser.project_name_marker_entries,
    synthetic_workspace_observed: workspace.runtime_observed,
    synthetic_workspace_coverage_complete: workspace.coverage_complete,
    synthetic_workspace_marker_files: workspace.synthetic_marker_files,
    app_seed_manifest_marker_present: knownManifestCanary,
    app_seed_manifest_probe_status: manifestEvidence.status,
    synthetic_sample_observed: syntheticSampleObserved,
    interpretation,
  };
}

async function main() {
  if (!isAuthorizedNativeRunner(process.env, process.platform) ||
      process.env.MEDSCALE_NORMAL_SAMPLE_PROBE !== "1") {
    throw new Error("Refusing normal sample probe outside authorized disposable hosted Windows CI");
  }
  const root = path.join(process.env.LOCALAPPDATA, ROOT_NAME);
  const result = await auditNormalSyntheticSample(root);
  const artifact = path.join("target", "e2e-shots", "release-webview-normal-postsample-audit.json");
  await mkdir(path.dirname(artifact), { recursive: true });
  await writeFile(artifact, JSON.stringify(result, null, 2) + "\n");
  console.log("RELEASE_WEBVIEW_POSTSAMPLE_AUDIT=" + JSON.stringify(result));
  if (result.interpretation !== "NO_SYNTHETIC_MARKERS_IN_SCANNED_NORMAL_RELEASE_POSTSAMPLE_RUNTIME") {
    throw new Error("Normal-release synthetic-sample WebView2 residual/privacy qualification incomplete");
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(() => {
    console.error("RELEASE_WEBVIEW_POSTSAMPLE_AUDIT=UNQUALIFIED");
    process.exitCode = 1;
  });
}
