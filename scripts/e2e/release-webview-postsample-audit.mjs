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

const MARK = Buffer.from("synthetic-subject-");
const ROOT_NAME = "org.medscale.desktop.preview";
const MANIFEST_NAME = "synthetic-workspace.desktop-seed.json";

async function manifestMarkerExists(filename) {
  try {
    const st = await lstat(filename);
    if (!st.isFile() || st.isSymbolicLink() || st.size > 1024 * 1024 || st.size === 0) return false;
    const bytes = await readFile(filename);
    return bytes.includes(MARK) || bytes.includes(Buffer.from("synthetic-subject-", "utf16le"));
  } catch { return false; }
}

export async function auditNormalSyntheticSample(appDataRoot) {
  const browser = await auditReleaseWebviewWelcomeRoot(path.join(appDataRoot, "EBWebView"));
  const workspace = await auditReleaseWebviewWelcomeRoot(path.join(appDataRoot, "synthetic-workspace"));
  // The exact Tauri app-local root includes browser and synthetic vault, but
  // may contain OTHER app-owned files. Audit it independently to ensure no
  // canary remains in an unaccounted directory. No file paths are exported.
  const appData = await auditReleaseWebviewWelcomeRoot(appDataRoot);
  const knownManifestCanary = await manifestMarkerExists(path.join(appDataRoot, MANIFEST_NAME));
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
