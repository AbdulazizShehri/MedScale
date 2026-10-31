import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { auditProfiles, hasObservedProfileBytes, hasCompleteAuditCoverage } from "./webview-disk-audit.mjs";
import { compareSyntheticProfilePhases } from "./webview-audit-compare.mjs";

test("bounded scanner emits counts and marker booleans only, no paths or contents", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "medscale-webview-audit-"));
  try {
    const storage = path.join(root, "Local Storage", "leveldb");
    const cache = path.join(root, "Cache");
    await mkdir(storage, { recursive: true });
    await mkdir(cache);
    const sensitiveValue = "synthetic-subject-test-only-do-not-publish";
    await writeFile(path.join(storage, "001.ldb"), Buffer.concat([
      Buffer.from("prefix-medscale.pro", "utf8"), Buffer.from("ject-suffix"),
      Buffer.from("synthetic-subject-", "utf16le"), Buffer.from("more-data"),
    ]));
    await writeFile(path.join(cache, "index"), Buffer.from(sensitiveValue));
    const result = await auditProfiles([{ label: "synthetic_fixture", root }]);
    const info = result.profiles[0];
    assert.equal(info.exists, true);
    assert.equal(hasObservedProfileBytes(result), true);
    assert.equal(hasCompleteAuditCoverage(result), true);
    assert.equal(info.files_examined, 2);
    assert.equal(info.categories.local_storage.key_marker_files, 1);
    assert.equal(info.categories.local_storage.synthetic_subject_marker_files, 1);
    assert.equal(info.categories.cache.synthetic_subject_marker_files, 1);
    assert.equal(JSON.stringify(result).includes(root), false);
    assert.equal(JSON.stringify(result).includes(sensitiveValue), false);
    assert.equal(JSON.stringify(result).includes("001.ldb"), false);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test("the scan is bounded and missing roots are reported, never guessed clear", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "medscale-webview-audit-"));
  try {
    for (const n of ["1", "2", "3"]) await writeFile(path.join(root, n), "medscale.project");
    const report = await auditProfiles([
      { label: "limited", root },
      { label: "missing", root: path.join(root, "does-not-exist") },
    ], { maxFiles: 1, maxBytes: 4096, maxFileBytes: 4096 });
    assert.equal(report.profiles[0].bounded, true);
    assert.equal(report.profiles[0].files_examined, 1);
    assert.equal(hasObservedProfileBytes(report), true, "some bytes were scanned");
    assert.equal(hasCompleteAuditCoverage(report), false, "bounded partial scans fail closed");
    assert.equal(report.profiles[1].exists, false);
    assert.equal(report.profiles[1].files_examined, 0);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test("empty and missing app profiles cannot yield a passing audit", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "medscale-webview-audit-"));
  try {
    const empty = path.join(root, "empty");
    await mkdir(empty);
    const roots = [
      { label: "missing", root: path.join(root, "absent") },
      { label: "empty", root: empty },
    ];
    assert.equal(hasObservedProfileBytes(await auditProfiles(roots)), false);
    await writeFile(path.join(empty, "zero-byte"), "");
    assert.equal(hasObservedProfileBytes(await auditProfiles(roots)), false);
    await writeFile(path.join(empty, "synthetic-only"), "synthetic-only-fixture");
    const complete = await auditProfiles(roots);
    assert.equal(hasObservedProfileBytes(complete), true);
    assert.equal(hasCompleteAuditCoverage(complete), true, "missing optional legacy root is allowed");
  } finally { await rm(root, { recursive: true, force: true }); }
});

test("a readable marker cannot hide skipped oversized files", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "medscale-webview-audit-"));
  try {
    await writeFile(path.join(root, "small"), "synthetic-subject-visible");
    await writeFile(path.join(root, "oversized"), "a".repeat(128));
    const report = await auditProfiles([{ label: "partial", root }],
      { maxFiles: 20, maxBytes: 4096, maxFileBytes: 64 });
    assert.equal(report.profiles[0].files_examined, 1);
    assert.equal(report.profiles[0].skipped_large_files, 1);
    assert.equal(hasObservedProfileBytes(report), true);
    assert.equal(hasCompleteAuditCoverage(report), false);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test("unreadable and symlink-skipped entries cannot be mistaken for complete scans", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "medscale-webview-audit-"));
  try {
    await writeFile(path.join(root, "synthetic-fixture"), "medscale.project");
    const complete = await auditProfiles([{ label: "app", root }]);
    assert.equal(hasCompleteAuditCoverage(complete), true);
    for (const field of ["unreadable_entries", "skipped_symlinks", "skipped_large_files"]) {
      const report = structuredClone(complete);
      report.profiles[0][field] = 1;
      assert.equal(hasCompleteAuditCoverage(report), false, field + " should disqualify coverage");
    }
  } finally { await rm(root, { recursive: true, force: true }); }
});

test("a single final file truncated at the byte budget is not complete evidence", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "medscale-webview-audit-"));
  try {
    await writeFile(path.join(root, "only-profile-file"), "synthetic-subject-end-marker");
    const limited = await auditProfiles([{ label: "app", root }],
      { maxFiles: 10, maxBytes: 8, maxFileBytes: 4096 });
    assert.equal(limited.profiles[0].files_examined, 1);
    assert.equal(limited.profiles[0].bytes_examined, 8);
    assert.equal(hasObservedProfileBytes(limited), true);
    assert.equal(limited.profiles[0].bounded, true,
      "exhausting the budget in the last file must be reported");
    assert.equal(hasCompleteAuditCoverage(limited), false);

    await writeFile(path.join(root, "only-profile-file"), "12345678");
    const exact = await auditProfiles([{ label: "app", root }],
      { maxFiles: 10, maxBytes: 8, maxFileBytes: 4096 });
    assert.equal(exact.profiles[0].bytes_examined, 8);
    assert.equal(exact.profiles[0].bounded, false,
      "a completely scanned file exactly at the budget is not partial");
    assert.equal(hasCompleteAuditCoverage(exact), true);
  } finally { await rm(root, { recursive: true, force: true }); }
});

function phaseFixture(markerFiles = 0) {
  return { profiles: [
    { label: "tauri_identifier_profile", exists: true, bytes_examined: 100,
      bounded: false, unreadable_entries: 0, skipped_large_files: 0, skipped_symlinks: 0,
      categories: { other: { files: 3, key_marker_files: 0,
        synthetic_subject_marker_files: markerFiles } },
      other_marker_storage_hints: markerFiles ? { unclassified_other: markerFiles } : {} },
    { label: "legacy_medscale_profile", exists: false, bytes_examined: 0,
      bounded: false, unreadable_entries: 0, skipped_large_files: 0, skipped_symlinks: 0,
      categories: {}, other_marker_storage_hints: {} },
  ] };
}

test("synthetic phase comparison reports three baseline markers without claiming file identity", () => {
  const report = compareSyntheticProfilePhases(phaseFixture(3), phaseFixture(3));
  assert.equal(report.qualified_coverage, true);
  assert.equal(report.interpretation, "MARKERS_ALREADY_IN_BASELINE_COUNTS_UNATTRIBUTED");
  assert.equal(report.rows.find((r) => r.category === "other").delta.synthetic_subject_marker_files, 0);
  assert.equal(JSON.stringify(report).includes("synthetic-subject-test-only"), false);
});

test("new synthetic marker counts after journeys require investigation", () => {
  const report = compareSyntheticProfilePhases(phaseFixture(0), phaseFixture(3));
  assert.equal(report.qualified_coverage, true);
  assert.equal(report.interpretation, "INCREASED_SYNTHETIC_MARKER_COUNTS");
  assert.equal(report.rows.find((r) => r.category === "other").delta.synthetic_subject_marker_files, 3);
});

test("missing or bounded baseline bytes are inconclusive, not privacy clearance", () => {
  const incomplete = phaseFixture(0);
  incomplete.profiles[0].bounded = true;
  assert.equal(compareSyntheticProfilePhases(incomplete, phaseFixture(0)).qualified_coverage, false);
  assert.equal(compareSyntheticProfilePhases(incomplete, phaseFixture(0)).interpretation, "INCONCLUSIVE_COVERAGE");
  const empty = phaseFixture(0);
  empty.profiles[0].bytes_examined = 0;
  assert.equal(compareSyntheticProfilePhases(empty, phaseFixture(3)).qualified_coverage, false);
  const unexpected = phaseFixture(0);
  unexpected.profiles[0].categories.private_location = { files: 1 };
  assert.throws(() => compareSyntheticProfilePhases(unexpected, phaseFixture(0)),
    /Unknown or missing synthetic audit profile\/category/);
});


test("other-category synthetic markers expose only fixed storage-type hints", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "medscale-webview-audit-"));
  try {
    const session = path.join(root, "Session Storage");
    const worker = path.join(root, "Service Worker");
    await mkdir(session);
    await mkdir(worker);
    const marker = "synthetic-subject-unit-only-sensitive-canary";
    await writeFile(path.join(session, "secret-file-to-hide.log"), marker);
    await writeFile(path.join(worker, "secret-worker-file"), marker);
    await writeFile(path.join(root, "secret-file-to-hide.sqlite"), marker);
    const report = await auditProfiles([{ label: "synthetic_fixture", root }]);
    assert.equal(hasCompleteAuditCoverage(report), true);
    assert.equal(report.profiles[0].categories.other.synthetic_subject_marker_files, 3);
    assert.deepEqual(report.profiles[0].other_marker_storage_hints, {
      sqlite_like: 1, session_state: 1, service_worker_state: 1,
    });
    const exported = JSON.stringify(report);
    assert.equal(exported.includes("secret-file-to-hide"), false);
    assert.equal(exported.includes("secret-worker-file"), false);
    assert.equal(exported.includes("unit-only-sensitive-canary"), false);
    assert.equal(exported.includes(root), false);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});


test("phase delta safely reports fixed storage hints and rejects forged totals", () => {
  const before = phaseFixture(0);
  const after = phaseFixture(3);
  after.profiles[0].other_marker_storage_hints = { session_state: 2, sqlite_like: 1 };
  const report = compareSyntheticProfilePhases(before, after);
  const app = report.other_marker_storage_hints.find((entry) =>
    entry.profile === "tauri_identifier_profile");
  assert.equal(app.before.session_state, 0);
  assert.equal(app.after.session_state, 2);
  assert.equal(app.delta.sqlite_like, 1);
  assert.equal(JSON.stringify(report).includes("secret-file-to-hide"), false);

  const missing = structuredClone(after);
  delete missing.profiles[0].other_marker_storage_hints;
  assert.equal(compareSyntheticProfilePhases(before, missing)
    .other_marker_storage_hints[0].delta, null, "missing hints must not look like zeros");

  const forged = structuredClone(after);
  forged.profiles[0].other_marker_storage_hints = { session_state: 4 };
  assert.throws(() => compareSyntheticProfilePhases(before, forged),
    /do not reconcile/);

  forged.profiles[0].other_marker_storage_hints = { user_private_path: 3 };
  assert.throws(() => compareSyntheticProfilePhases(before, forged),
    /Unrecognized synthetic storage hint category/);
});


test("app data scopes distinguish WebView2 runtime from persisted synthetic workspace without file details", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "medscale-app-scope-"));
  const marker = "synthetic-subject-unit-private-only";
  try {
    await mkdir(path.join(root, "EBWebView", "Default"), { recursive: true });
    await mkdir(path.join(root, "synthetic-workspace"));
    await writeFile(path.join(root, "EBWebView", "Default", "private-webview-cache"), marker);
    await writeFile(path.join(root, "synthetic-workspace", "private-vault.sqlite"), marker);
    await writeFile(path.join(root, "synthetic-workspace.desktop-seed.json"), marker);
    await writeFile(path.join(root, "private-other-app-file"), marker);
    const report = await auditProfiles([{ label: "synthetic_fixture", root }]);
    assert.equal(hasCompleteAuditCoverage(report), true);
    const scoped = report.profiles[0].app_data_scopes;
    assert.equal(scoped.webview2_runtime.synthetic_subject_marker_files, 1);
    assert.equal(scoped.synthetic_workspace.synthetic_subject_marker_files, 2);
    assert.equal(scoped.other_app_data.synthetic_subject_marker_files, 1);
    assert.equal(Object.values(scoped).reduce((n, x) => n + x.files_examined, 0),
      report.profiles[0].files_examined);
    const serialized = JSON.stringify(report);
    for (const sensitive of [marker, "private-webview-cache", "private-vault.sqlite",
      "private-other-app-file", "synthetic-workspace.desktop-seed.json", root]) {
      assert.equal(serialized.includes(sensitive), false, "raw runtime file details must not escape");
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});


test("app-data ownership scope comparison separates WebView2 from intended synthetic vault", () => {
  const before = phaseFixture(0);
  const after = phaseFixture(3);
  before.profiles[0].app_data_scopes = {
    webview2_runtime: { files_examined: 70, key_marker_files: 0, synthetic_subject_marker_files: 0 },
    other_app_data: { files_examined: 30, key_marker_files: 0, synthetic_subject_marker_files: 0 },
  };
  after.profiles[0].app_data_scopes = {
    webview2_runtime: { files_examined: 70, key_marker_files: 0, synthetic_subject_marker_files: 0 },
    synthetic_workspace: { files_examined: 3, key_marker_files: 0, synthetic_subject_marker_files: 3 },
    other_app_data: { files_examined: 27, key_marker_files: 0, synthetic_subject_marker_files: 0 },
  };
  before.profiles[1].app_data_scopes = {};
  after.profiles[1].app_data_scopes = {};
  const delta = compareSyntheticProfilePhases(before, after).app_data_scope_comparison[0];
  assert.equal(delta.after.webview2_runtime.synthetic_subject_marker_files, 0);
  assert.equal(delta.before.synthetic_workspace.synthetic_subject_marker_files, 0);
  assert.equal(delta.after.synthetic_workspace.synthetic_subject_marker_files, 3);
  assert.equal(delta.delta.synthetic_workspace.synthetic_subject_marker_files, 3);

  const forged = structuredClone(after);
  forged.profiles[0].app_data_scopes.synthetic_workspace.synthetic_subject_marker_files = 4;
  assert.throws(() => compareSyntheticProfilePhases(before, forged), /do not reconcile/);

  const unknown = structuredClone(after);
  unknown.profiles[0].app_data_scopes.private_path = { files_examined: 1 };
  assert.throws(() => compareSyntheticProfilePhases(before, unknown),
    /Unrecognized synthetic app-data scope/);

  const old = structuredClone(before);
  delete old.profiles[0].app_data_scopes;
  assert.equal(compareSyntheticProfilePhases(old, after)
    .app_data_scope_comparison[0].delta, null,
    "legacy artifacts without app-data scopes must not be interpreted as zero");
});
