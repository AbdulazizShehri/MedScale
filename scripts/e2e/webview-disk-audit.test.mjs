import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { auditProfiles } from "./webview-disk-audit.mjs";

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
    assert.equal(report.profiles[1].exists, false);
    assert.equal(report.profiles[1].files_examined, 0);
  } finally { await rm(root, { recursive: true, force: true }); }
});
