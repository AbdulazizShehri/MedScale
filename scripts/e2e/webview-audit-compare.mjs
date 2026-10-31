/**
 * Compare two synthetic-only, read-only, app-owned WebView2 profile scans.
 *
 * The result is aggregate counts, NEVER private file paths, names, content,
 * canary subject IDs, audio, or hashes of user-owned data. Equal counts do
 * not prove that the same files are present, nor imply secure erasure.
 */
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { hasCompleteAuditCoverage, hasObservedProfileBytes } from "./webview-disk-audit.mjs";

const categories = ["local_storage", "indexed_db", "cache", "crash_or_log", "other"];
const counters = ["files", "key_marker_files", "synthetic_subject_marker_files"];
const labels = ["tauri_identifier_profile", "legacy_medscale_profile"];

const hintLabels = ["session_state", "service_worker_state", "blob_or_file_state",
  "leveldb_like", "sqlite_like", "profile_root_or_default", "unclassified_other"];

function storageHintCounts(report, label) {
  const profile = report.profiles.find((p) => p.label === label);
  const values = profile?.other_marker_storage_hints;
  // Older report versions did not record hints: missing evidence is not zero.
  if (values === undefined) return null;
  if (!values || typeof values !== "object" || Array.isArray(values) ||
      Object.keys(values).some((key) => !hintLabels.includes(key))) {
    throw new Error("Unrecognized synthetic storage hint category");
  }
  const numbers = Object.fromEntries(hintLabels.map((key) => {
    const count = values[key] ?? 0;
    if (!Number.isSafeInteger(count) || count < 0) {
      throw new Error("Invalid synthetic storage hint count");
    }
    return [key, count];
  }));
  const total = Object.values(numbers).reduce((a, b) => a + b, 0);
  const otherMarkers = profile.categories?.other?.synthetic_subject_marker_files ?? 0;
  if (total !== otherMarkers) {
    throw new Error("Synthetic storage hint counters do not reconcile");
  }
  return numbers;
}


function counts(report, label, category) {
  const root = report.profiles.find((p) => p.label === label);
  const values = root?.categories?.[category] ?? {};
  const result = {};
  for (const key of counters) {
    const n = values[key] ?? 0;
    if (!Number.isSafeInteger(n) || n < 0) throw new Error("Invalid synthetic aggregate counter");
    result[key] = n;
  }
  return result;
}

export function compareSyntheticProfilePhases(before, after) {
  for (const report of [before, after]) {
    if (!Array.isArray(report?.profiles) ||
        report.profiles.length !== labels.length ||
        labels.some((label) => report.profiles.filter((p) => p.label === label).length !== 1) ||
        report.profiles.some((p) => Object.keys(p.categories ?? {}).some((c) => !categories.includes(c)))) {
      throw new Error("Unknown or missing synthetic audit profile/category");
    }
  }
  const baselineValid = hasObservedProfileBytes(before) && hasCompleteAuditCoverage(before);
  const afterValid = hasObservedProfileBytes(after) && hasCompleteAuditCoverage(after);
  const rows = [];
  for (const label of labels) {
    for (const category of categories) {
      const b = counts(before, label, category);
      const a = counts(after, label, category);
      rows.push({
        profile: label, category,
        before: b, after: a,
        delta: Object.fromEntries(counters.map((key) => [key, a[key] - b[key]])),
      });
    }
  }
  const other_marker_storage_hints = labels.map((label) => {
    const b = storageHintCounts(before, label);
    const a = storageHintCounts(after, label);
    return {
      profile: label,
      before: b,
      after: a,
      delta: b && a ? Object.fromEntries(hintLabels.map((key) => [key, a[key] - b[key]])) : null,
    };
  });
  const baselineMarkers = rows.reduce((s, row) => s + row.before.synthetic_subject_marker_files, 0);
  const newMarkerCount = rows.reduce((s, row) => s +
    Math.max(0, row.delta.synthetic_subject_marker_files), 0);
  return {
    scope: "synthetic-only aggregated relative counts; not file identity, PHI or OS privacy clearance",
    phase: "before_workspace_vs_after_journeys",
    qualified_coverage: baselineValid && afterValid,
    interpretation: !(baselineValid && afterValid) ? "INCONCLUSIVE_COVERAGE" :
      newMarkerCount > 0 ? "INCREASED_SYNTHETIC_MARKER_COUNTS" :
      baselineMarkers > 0 ? "MARKERS_ALREADY_IN_BASELINE_COUNTS_UNATTRIBUTED" :
      "NO_SYNTHETIC_MARKERS_IN_SCANNED_PHASES",
    rows,
    other_marker_storage_hints,
  };
}

async function main() {
  if (process.env.GITHUB_ACTIONS !== "true" || process.platform !== "win32" ||
      process.env.MEDSCALE_CI_WEBVIEW_AUDIT !== "1" || !process.env.RUNNER_TEMP) {
    throw new Error("Disposable Windows synthetic profile-audit opt-in required");
  }
  const dir = path.join("target", "e2e-shots");
  const before = JSON.parse(await readFile(path.join(dir,
    "webview-disk-profile-before-workspace.json"), "utf8"));
  const after = JSON.parse(await readFile(path.join(dir,
    "webview-disk-profile-audit.json"), "utf8"));
  const result = compareSyntheticProfilePhases(before, after);
  await writeFile(path.join(dir, "webview-disk-profile-phase-delta.json"),
    JSON.stringify(result, null, 2) + "\n", "utf8");
  console.log("WEBVIEW_SYNTHETIC_PHASE_DELTA=" + JSON.stringify({
    qualified_coverage: result.qualified_coverage,
    interpretation: result.interpretation,
    synthetic_counts_by_phase: result.rows
      .filter((r) => r.before.synthetic_subject_marker_files ||
                    r.after.synthetic_subject_marker_files)
      .map((r) => ({ profile: r.profile, category: r.category,
        before: r.before.synthetic_subject_marker_files,
        after: r.after.synthetic_subject_marker_files })),
    storage_hints_by_phase: result.other_marker_storage_hints.filter((r) => r.before && r.after &&
      (Object.values(r.before).some(Boolean) || Object.values(r.after).some(Boolean))),
  }));
  if (!result.qualified_coverage) throw new Error("Incomplete baseline or post-journey scan");
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(() => {
    console.error("WEBVIEW_SYNTHETIC_PHASE_DELTA=UNQUALIFIED");
    process.exitCode = 1;
  });
}
