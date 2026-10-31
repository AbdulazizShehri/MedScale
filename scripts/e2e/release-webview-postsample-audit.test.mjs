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
