/**
 * Release-mode WebView2 welcome-screen profile probe, synthetic CI only.
 *
 * This scans ONLY the known EBWebView child of Tauri's app-local data root,
 * not the app vault. It NEVER prints file names, paths, file contents or
 * subject identifiers. It cannot prove privacy after workspace interaction,
 * PHI absence, crash/swap safety or InPrivate enforcement by itself.
 */
import { lstat, readdir, mkdir, writeFile } from "node:fs/promises";
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

// Independent metadata inventory: the current main scanner predates the
// pending last-file truncation fix, so an exhausted final byte budget may
// otherwise be misreported as complete. Never return names or paths.
async function inventoryRuntimeBytes(root) {
  const top = await lstat(root);
  if (!top.isDirectory() || top.isSymbolicLink()) throw new Error("Invalid runtime root");
  const pending = [root];
  let files = 0;
  let bytes = 0;
  let subject_name_marker_entries = 0;
  let project_name_marker_entries = 0;
  while (pending.length) {
    const current = pending.pop();
    for (const ent of await readdir(current, { withFileTypes: true })) {
      // Names may themselves contain a synthetic subject or project key even
      // when every file's contents are clean. Report only fixed counters.
      const entryName = ent.name.toLowerCase();
      if (entryName.includes("synthetic-subject-")) subject_name_marker_entries++;
      if (entryName.includes("medscale.project")) project_name_marker_entries++;
      if (ent.isSymbolicLink()) throw new Error("Symlink in runtime profile");
      const absolute = path.join(current, ent.name);
      if (ent.isDirectory()) { pending.push(absolute); continue; }
      if (!ent.isFile()) throw new Error("Unsupported runtime entry");
      const st = await lstat(absolute);
      if (!st.isFile() || st.isSymbolicLink()) throw new Error("Runtime entry changed");
      files++;
      bytes += st.size;
      if (!Number.isSafeInteger(bytes)) throw new Error("Unbounded profile metadata");
    }
  }
  return { files, bytes, subject_name_marker_entries, project_name_marker_entries };
}

export async function auditReleaseWebviewWelcomeRoot(root, limits) {
  const profileReport = await auditProfiles([{ label: LABEL, root }], limits);
  const p = profileReport.profiles[0];
  // Fail closed on any unexpected metadata, incomplete last-file read or
  // change during the read-only scan, even before the parent #383 fix merges.
  let inventoryMatches = false;
  let subjectNameMarkers = 0;
  let projectNameMarkers = 0;
  if (p.exists) {
    try {
      const inventory = await inventoryRuntimeBytes(root);
      inventoryMatches = inventory.files === p.files_examined &&
        inventory.bytes === p.bytes_examined;
      subjectNameMarkers = inventory.subject_name_marker_entries;
      projectNameMarkers = inventory.project_name_marker_entries;
    } catch { inventoryMatches = false; }
  }
  const coverageComplete = hasCompleteAuditCoverage(profileReport) &&
    hasObservedProfileBytes(profileReport) && p.exists && p.files_examined > 0 &&
    inventoryMatches;
  const counters = Object.values(p.categories);
  const syntheticMarkerFiles = counters.reduce((n, c) => n + c.synthetic_subject_marker_files, 0);
  const projectKeyMarkerFiles = counters.reduce((n, c) => n + c.key_marker_files, 0);
  const conclusion = !p.exists ? "INCONCLUSIVE_MISSING_WEBVIEW2_RUNTIME" :
    !coverageComplete ? "INCONCLUSIVE_UNREADABLE_OR_BOUNDED_RUNTIME" :
    subjectNameMarkers > 0 || projectNameMarkers > 0 ? "MARKER_IN_RELEASE_RUNTIME_ENTRY_NAME" :
    projectKeyMarkerFiles > 0 ? "PROJECT_KEY_MARKER_IN_SCANNED_RELEASE_RUNTIME" :
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
    subject_name_marker_entries: subjectNameMarkers,
    project_name_marker_entries: projectNameMarkers,
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
  if (result.conclusion !== "NO_SYNTHETIC_MARKERS_IN_SCANNED_RELEASE_WELCOME_RUNTIME") {
    throw new Error("Native release welcome-screen WebView2 evidence is not qualified");
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(() => {
    console.error("RELEASE_WEBVIEW_WELCOME_AUDIT=UNQUALIFIED");
    process.exitCode = 1;
  });
}
