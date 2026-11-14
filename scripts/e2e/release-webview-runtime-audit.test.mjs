import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { auditReleaseWebviewWelcomeRoot, isAuthorizedNativeRunner } from "./release-webview-runtime-audit.mjs";

const env = {
  GITHUB_ACTIONS: "true", RUNNER_ENVIRONMENT: "github-hosted", RUNNER_OS: "Windows",
  MEDSCALE_RELEASE_WEBVIEW_PROBE: "1", RUNNER_TEMP: "disposable-ci",
  LOCALAPPDATA: "synthetic-local-app-data",
};

test("native release WebView2 probe refuses local, non-Windows, and non-opted-in use", () => {
  assert.equal(isAuthorizedNativeRunner(env, "win32"), true);
  assert.equal(isAuthorizedNativeRunner(env, "linux"), false);
  for (const key of Object.keys(env)) {
    const unauthorized = { ...env };
    delete unauthorized[key];
    assert.equal(isAuthorizedNativeRunner(unauthorized, "win32"), false, key);
  }
});

test("missing EBWebView root is inconclusive, never treated as an empty clean browser", async () => {
  const root = path.join(os.tmpdir(), "medscale-absent-wv-" + process.pid + "-" + Date.now());
  const r = await auditReleaseWebviewWelcomeRoot(root);
  assert.equal(r.runtime_observed, false);
  assert.equal(r.coverage_complete, false);
  assert.equal(r.conclusion, "INCONCLUSIVE_MISSING_WEBVIEW2_RUNTIME");
  assert.equal(r.synthetic_marker_files, 0);
});

test("readable synthetic release browser welcome fixture reports only aggregate zero markers", async () => {
  const home = await mkdtemp(path.join(os.tmpdir(), "medscale-release-wv-"));
  try {
    const root = path.join(home, "EBWebView");
    await mkdir(path.join(root, "Default"), { recursive: true });
    await writeFile(path.join(root, "private-browser-asset-name"), "ordinary-runtime-fixture");
    const r = await auditReleaseWebviewWelcomeRoot(root);
    assert.equal(r.coverage_complete, true);
    assert.equal(r.runtime_observed, true);
    assert.equal(r.files_examined, 1);
    assert.equal(r.synthetic_marker_files, 0);
    assert.equal(r.conclusion, "NO_SYNTHETIC_MARKERS_IN_SCANNED_RELEASE_WELCOME_RUNTIME");
    const serialized = JSON.stringify(r);
    for (const item of [home, root, "private-browser-asset-name", "ordinary-runtime-fixture"]) {
      assert.equal(serialized.includes(item), false);
    }
  } finally {
    await rm(home, { recursive: true, force: true });
  }
});

test("synthetic marker is detected but never echoed in the release browser summary", async () => {
  const home = await mkdtemp(path.join(os.tmpdir(), "medscale-release-wv-"));
  try {
    const root = path.join(home, "EBWebView");
    await mkdir(root);
    const secret = "synthetic-subject-private-fixture-do-not-log";
    await writeFile(path.join(root, "hidden-runtime-file"), Buffer.from(secret, "utf16le"));
    const r = await auditReleaseWebviewWelcomeRoot(root);
    assert.equal(r.coverage_complete, true);
    assert.equal(r.synthetic_marker_files, 1);
    assert.equal(r.conclusion, "SYNTHETIC_MARKERS_OBSERVED_IN_RELEASE_WELCOME_RUNTIME");
    const serialized = JSON.stringify(r);
    for (const item of [root, home, secret, "hidden-runtime-file"]) {
      assert.equal(serialized.includes(item), false);
    }
  } finally {
    await rm(home, { recursive: true, force: true });
  }
});

test("a byte-bounded final runtime file is inconclusive rather than clean", async () => {
  const home = await mkdtemp(path.join(os.tmpdir(), "medscale-release-wv-"));
  try {
    const root = path.join(home, "EBWebView");
    await mkdir(root);
    await writeFile(path.join(root, "opaque-runtime"), "01234567890123456789");
    const r = await auditReleaseWebviewWelcomeRoot(root, {
      maxFiles: 3, maxBytes: 8, maxFileBytes: 64,
    });
    assert.equal(r.coverage_complete, false);
    assert.equal(r.conclusion, "INCONCLUSIVE_UNREADABLE_OR_BOUNDED_RUNTIME");
  } finally {
    await rm(home, { recursive: true, force: true });
  }
});

test("subject marker in an otherwise clean runtime filename fails closed without logging names", async () => {
  const home = await mkdtemp(path.join(os.tmpdir(), "medscale-release-name-fixture-"));
  try {
    const root = path.join(home, "EBWebView");
    await mkdir(root);
    const privateFilename = "synthetic-subject-file-name-must-not-escape";
    await writeFile(path.join(root, privateFilename), "clean file contents");
    const report = await auditReleaseWebviewWelcomeRoot(root);
    assert.equal(report.coverage_complete, true);
    assert.equal(report.synthetic_marker_files, 0);
    assert.equal(report.subject_name_marker_entries, 1);
    assert.equal(report.conclusion, "MARKER_IN_RELEASE_RUNTIME_ENTRY_NAME");
    assert.equal(JSON.stringify(report).includes(privateFilename), false);
    assert.equal(JSON.stringify(report).includes(home), false);
  } finally {
    await rm(home, { recursive: true, force: true });
  }
});

test("project key in a directory name fails closed even if content has no marker", async () => {
  const home = await mkdtemp(path.join(os.tmpdir(), "medscale-release-directory-fixture-"));
  try {
    const root = path.join(home, "EBWebView");
    const privateDirectory = "medscale.project-private-profile";
    await mkdir(path.join(root, privateDirectory), { recursive: true });
    await writeFile(path.join(root, privateDirectory, "regular-file"), "regular contents");
    const report = await auditReleaseWebviewWelcomeRoot(root);
    assert.equal(report.coverage_complete, true);
    assert.equal(report.project_key_marker_files, 0);
    assert.equal(report.project_name_marker_entries, 1);
    assert.equal(report.conclusion, "MARKER_IN_RELEASE_RUNTIME_ENTRY_NAME");
    assert.equal(JSON.stringify(report).includes(privateDirectory), false);
  } finally {
    await rm(home, { recursive: true, force: true });
  }
});
