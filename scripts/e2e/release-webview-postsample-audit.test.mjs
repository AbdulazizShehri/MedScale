import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { auditNormalSyntheticSample } from "./release-webview-postsample-audit.mjs";

async function fixture(run) {
  const root = await mkdtemp(path.join(os.tmpdir(), "medscale-normal-sample-"));
  try {
    await mkdir(path.join(root, "EBWebView", "Default"), { recursive: true });
    await mkdir(path.join(root, "synthetic-workspace"), { recursive: true });
    await writeFile(path.join(root, "EBWebView", "Default", "private-cache"), "benign-browser-state");
    await run(root);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

test("normal-mode synthetic sample separates persisted workspace from browser and logs no raw identifiers", async () => {
  await fixture(async root => {
    const syntheticMarker = "synthetic-subject-private-canary-must-stay-hidden";
    await writeFile(path.join(root, "synthetic-workspace", "private-vault-file.sqlite"), syntheticMarker);
    await writeFile(path.join(root, "synthetic-workspace.desktop-seed.json"), syntheticMarker);
    const r = await auditNormalSyntheticSample(root);
    assert.equal(r.browser_coverage_complete, true);
    assert.equal(r.synthetic_workspace_coverage_complete, true);
    assert.equal(r.browser_synthetic_marker_files, 0);
    assert.equal(r.synthetic_workspace_marker_files, 1);
    assert.equal(r.app_seed_manifest_marker_present, true);
    assert.equal(r.synthetic_sample_observed, true);
    assert.equal(r.interpretation, "NO_SYNTHETIC_MARKERS_IN_SCANNED_NORMAL_RELEASE_POSTSAMPLE_RUNTIME");
    const json = JSON.stringify(r);
    for (const secret of [root, "private-cache", "private-vault-file.sqlite", syntheticMarker]) {
      assert.equal(json.includes(secret), false);
    }
  });
});

test("test fixture detects synthetic marker written to the normal-mode WebView2 runtime", async () => {
  await fixture(async root => {
    await writeFile(path.join(root, "EBWebView", "Default", "private-browser-file"), "synthetic-subject-browser-canary");
    await writeFile(path.join(root, "synthetic-workspace", "private-vault-file"), "synthetic-subject-workspace-canary");
    await writeFile(path.join(root, "synthetic-workspace.desktop-seed.json"), "synthetic-subject-seed-canary");
    const r = await auditNormalSyntheticSample(root);
    assert.equal(r.synthetic_sample_observed, true);
    assert.equal(r.browser_synthetic_marker_files, 1);
    assert.equal(r.interpretation, "SYNTHETIC_MARKERS_IN_NORMAL_RELEASE_WEBVIEW2_RUNTIME");
    assert.equal(JSON.stringify(r).includes("browser-canary"), false);
  });
});

test("no synthetic workspace after a welcome button action must remain inconclusive", async () => {
  await fixture(async root => {
    const r = await auditNormalSyntheticSample(root);
    assert.equal(r.synthetic_sample_observed, false);
    assert.equal(r.interpretation, "INCONCLUSIVE_INCOMPLETE_NATIVE_PROFILE_OR_SYNTHETIC_WORKSPACE");
  });
});


test("workspace canary without seed manifest is INCONCLUSIVE, not a release privacy pass", async () => {
  await fixture(async root => {
    await writeFile(path.join(root, "synthetic-workspace", "synthetic-data"), "synthetic-subject-workspace-only");
    const report = await auditNormalSyntheticSample(root);
    assert.equal(report.synthetic_workspace_marker_files, 1);
    assert.equal(report.app_seed_manifest_marker_present, false);
    assert.equal(report.synthetic_sample_observed, false);
    assert.equal(report.interpretation, "INCONCLUSIVE_NO_SYNTHETIC_WORKSPACE_MARKERS");
  });
});

test("seed manifest without populated synthetic workspace is INCONCLUSIVE", async () => {
  await fixture(async root => {
    await writeFile(path.join(root, "synthetic-workspace", "empty-workspace"), "non-sensitive fixture");
    await writeFile(path.join(root, "synthetic-workspace.desktop-seed.json"), "synthetic-subject-seed-only");
    const report = await auditNormalSyntheticSample(root);
    assert.equal(report.synthetic_workspace_marker_files, 0);
    assert.equal(report.app_seed_manifest_marker_present, true);
    assert.equal(report.synthetic_sample_observed, false);
    assert.equal(report.interpretation, "INCONCLUSIVE_NO_SYNTHETIC_WORKSPACE_MARKERS");
  });
});

test("unattributed app-local synthetic marker fails closed instead of claiming browser privacy", async () => {
  await fixture(async root => {
    await writeFile(path.join(root, "synthetic-workspace", "private-vault-file"), "synthetic-subject-workspace");
    await writeFile(path.join(root, "synthetic-workspace.desktop-seed.json"), "synthetic-subject-seed");
    await writeFile(path.join(root, "private-unclassified-app-data"), "synthetic-subject-unattributed");
    const report = await auditNormalSyntheticSample(root);
    assert.equal(report.app_data_coverage_complete, true);
    assert.equal(report.app_data_synthetic_marker_files, 3);
    assert.equal(report.browser_synthetic_marker_files, 0);
    assert.equal(report.synthetic_workspace_marker_files, 1);
    assert.equal(report.unattributed_app_data_marker_files, 1);
    assert.equal(report.interpretation, "INCONCLUSIVE_UNATTRIBUTED_APP_DATA_MARKERS");
    const serialized = JSON.stringify(report);
    for (const privateValue of ["private-unclassified-app-data", "synthetic-subject-unattributed", root]) {
      assert.equal(serialized.includes(privateValue), false);
    }
  });
});

test("project-key marker in app-local data refuses release privacy pass even if subject markers are attributed", async () => {
  await fixture(async root => {
    await writeFile(path.join(root, "synthetic-workspace", "sample-data"), "synthetic-subject-accepted-canary");
    await writeFile(path.join(root, "synthetic-workspace.desktop-seed.json"), "synthetic-subject-seed-canary");
    await writeFile(path.join(root, "EBWebView", "Default", "private-app-reference"), "medscale.project");
    const report = await auditNormalSyntheticSample(root);
    assert.equal(report.synthetic_sample_observed, true);
    assert.equal(report.app_data_project_key_marker_files, 1);
    assert.equal(report.browser_project_key_marker_files, 1);
    assert.equal(report.browser_synthetic_marker_files, 0);
    assert.equal(report.unattributed_app_data_marker_files, 0);
    assert.equal(report.interpretation, "PROJECT_KEY_MARKER_IN_SCANNED_APP_DATA");
    assert.equal(JSON.stringify(report).includes("private-app-reference"), false);
  });
});

test("project key marker in a directory name outside browser and vault rejects normal-mode privacy pass", async () => {
  await fixture(async root => {
    await writeFile(path.join(root, "synthetic-workspace", "synthetic-data"), "synthetic-subject-fixture");
    await writeFile(path.join(root, "synthetic-workspace.desktop-seed.json"), "synthetic-subject-seed-canary");
    const privateFolder = "medscale.project-confidential-folder";
    await mkdir(path.join(root, privateFolder));
    await writeFile(path.join(root, privateFolder, "ordinary-file"), "ordinary contents");
    const report = await auditNormalSyntheticSample(root);
    assert.equal(report.app_data_coverage_complete, true);
    assert.equal(report.app_data_project_key_marker_files, 0);
    assert.equal(report.app_data_project_name_marker_entries, 1);
    assert.equal(report.interpretation, "MARKER_IN_SCANNED_APP_DATA_ENTRY_NAME");
    assert.equal(JSON.stringify(report).includes(privateFolder), false);
    assert.equal(JSON.stringify(report).includes(root), false);
  });
});
