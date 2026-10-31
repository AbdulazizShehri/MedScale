import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { auditProfiles, hasObservedProfileBytes, hasCompleteAuditCoverage } from "./webview-disk-audit.mjs";

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
