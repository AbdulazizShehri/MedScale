/**
 * Release-mode WebView2 welcome-screen profile probe, synthetic CI only.
 *
 * This scans ONLY the known EBWebView child of Tauri's app-local data root,
 * not the app vault. It NEVER prints file names, paths, file contents or
 * subject identifiers. It cannot prove privacy after workspace interaction,
 * PHI absence, crash/swap safety or InPrivate enforcement by itself.
 */
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { auditProfiles, hasCompleteAuditCoverage, hasObservedProfileBytes } from "./webview-disk-audit.mjs";

const LABEL = "release_webview2_runtime";

export function isAuthorizedNativeRunner(env, platform) {
  return platform === "win32" && env.GITHUB_ACTIONS === "true" &&
    env.RUNNER_ENVIRONMENT === "github-hosted" && env.RUNNER_OS === "Windows" &&
    env.MEDSCALE_RELEASE_WEBVIEW_PROBE === "1" &&
    Boolean(env.RUNNER_TEMP) && Boolean(env.LOCALAPPDATA);
}

export async function auditReleaseWebviewWelcomeRoot(root, limits) {
  const profileReport = await auditProfiles([{ label: LABEL, root }], limits);
  const p = profileReport.profiles[0];
  const coverageComplete = hasCompleteAuditCoverage(profileReport) &&
    hasObservedProfileBytes(profileReport) && p.exists && p.files_examined > 0;
  const counters = Object.values(p.categories);
  const syntheticMarkerFiles = counters.reduce((n, c) => n + c.synthetic_subject_marker_files, 0);
  const projectKeyMarkerFiles = counters.reduce((n, c) => n + c.key_marker_files, 0);
  const conclusion = !p.exists ? "INCONCLUSIVE_MISSING_WEBVIEW2_RUNTIME" :
    !coverageComplete ? "INCONCLUSIVE_UNREADABLE_OR_BOUNDED_RUNTIME" :
    syntheticMarkerFiles > 0 ? "SYNTHETIC_MARKERS_OBSERVED_IN_RELEASE_WELCOME_RUNTIME" :
    "NO_SYNTHETIC_MARKERS_IN_SCANNED_RELEASE_WELCOME_RUNTIME";
  return {
    scope: "synthetic-only-native-release-welcome-screen-EBWebView; not PHI, OS or post-workspace clearance",
    runtime_observed: p.exists && p.files_examined > 0,
    coverage_complete: coverageComplete,
    files_examined: p.files_examined,
    bytes_examined: p.bytes_examined,
    synthetic_marker_files: syntheticMarkerFiles,
    project_key_marker_files: projectKeyMarkerFiles,
    conclusion,
  };
}

async function main() {
  if (!isAuthorizedNativeRunner(process.env, process.platform)) {
    throw new Error("Refusing release WebView2 probe outside disposable hosted Windows CI");
  }
  const root = path.join(process.env.LOCALAPPDATA, "org.medscale.desktop.preview", "EBWebView");
  const result = await auditReleaseWebviewWelcomeRoot(root);
  const file = path.join("target", "e2e-shots", "release-webview-runtime-welcome-audit.json");
  await mkdir(path.dirname(file), { recursive: true });
  await writeFile(file, JSON.stringify(result, null, 2) + "\n");
  console.log("RELEASE_WEBVIEW_WELCOME_AUDIT=" + JSON.stringify(result));
  if (!result.coverage_complete || result.synthetic_marker_files > 0) {
    throw new Error("Native release welcome-screen WebView2 evidence is not qualified");
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(() => {
    console.error("RELEASE_WEBVIEW_WELCOME_AUDIT=UNQUALIFIED");
    process.exitCode = 1;
  });
}
