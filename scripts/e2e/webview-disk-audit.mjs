/**
 * Read-only, bounded synthetic WebView profile inspection on a disposable GitHub
 * Windows runner. This is NOT a PHI/OS privacy attestation: content is not
 * printed, uploaded, or copied, and the scan excludes OS swap, crash and backups.
 */
import { createReadStream } from "node:fs";
import { lstat, readdir, mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const probes = ["medscale.project", "synthetic-subject-"];
const needleSets = probes.map((key) => [Buffer.from(key, "utf8"), Buffer.from(key, "utf16le")]);
const overlap = Math.max(...needleSets.flat().map((b) => b.length)) - 1;
const defaults = { maxFiles: 1500, maxBytes: 128 * 1024 * 1024, maxFileBytes: 24 * 1024 * 1024 };

function classify(parts) {
  const names = parts.map((x) => x.toLowerCase());
  if (names.includes("local storage")) return "local_storage";
  if (names.includes("indexeddb")) return "indexed_db";
  if (names.some((x) => /^(cache|cache_storage|code cache|gpucache|dawncache)$/.test(x))) return "cache";
  if (names.some((x) => /^(crashpad|logs?)$/.test(x))) return "crash_or_log";
  return "other";
}

async function scanNeedles(file, remaining, found) {
  let scanned = 0;
  let tail = Buffer.alloc(0);
  const reader = createReadStream(file, { highWaterMark: 64 * 1024 });
  for await (const chunk of reader) {
    const n = Math.min(chunk.length, remaining - scanned);
    if (n <= 0) break;
    const buf = Buffer.concat([tail, chunk.subarray(0, n)]);
    for (let i = 0; i < needleSets.length; i++) {
      if (!found[i] && needleSets[i].some((needle) => buf.includes(needle))) found[i] = true;
    }
    scanned += n;
    tail = buf.subarray(Math.max(0, buf.length - overlap));
    if (scanned >= remaining) break;
  }
  return scanned;
}

export async function auditProfiles(roots, limits = defaults) {
  const report = { scope: "synthetic-only-ci-app-profile-byte-presence; not a privacy clearance", profiles: [] };
  for (const { label, root } of roots) {
    const info = {
      label, exists: false, files_examined: 0, bytes_examined: 0, skipped_symlinks: 0,
      skipped_large_files: 0, unreadable_entries: 0, bounded: false, categories: {},
    };
    report.profiles.push(info);
    let stat;
    try { stat = await lstat(root); } catch (e) {
      if (e.code !== "ENOENT") info.unreadable_entries++;
      continue;
    }
    if (!stat.isDirectory() || stat.isSymbolicLink()) { info.unreadable_entries++; continue; }
    info.exists = true;
    const pending = [{ dir: root, parts: [] }];
    while (pending.length) {
      if (info.files_examined >= limits.maxFiles || info.bytes_examined >= limits.maxBytes) {
        info.bounded = true; break;
      }
      const { dir, parts } = pending.pop();
      let entries;
      try { entries = await readdir(dir, { withFileTypes: true }); }
      catch { info.unreadable_entries++; continue; }
      for (const ent of entries) {
        if (info.files_examined >= limits.maxFiles || info.bytes_examined >= limits.maxBytes) {
          info.bounded = true; break;
        }
        // No junction or symlink traversal, no ownership mutation, no names in report.
        if (ent.isSymbolicLink()) { info.skipped_symlinks++; continue; }
        const file = path.join(dir, ent.name);
        if (ent.isDirectory()) { pending.push({ dir: file, parts: [...parts, ent.name] }); continue; }
        if (!ent.isFile()) continue;
        const category = classify(parts);
        const bucket = info.categories[category] ??= {
          files: 0, key_marker_files: 0, synthetic_subject_marker_files: 0,
        };
        bucket.files++;
        let metadata;
        try { metadata = await lstat(file); } catch { info.unreadable_entries++; continue; }
        if (!metadata.isFile() || metadata.size > limits.maxFileBytes) { info.skipped_large_files++; continue; }
        const found = [false, false];
        try {
          const n = await scanNeedles(file, limits.maxBytes - info.bytes_examined, found);
          info.bytes_examined += n;
          info.files_examined++;
          if (found[0]) bucket.key_marker_files++;
          if (found[1]) bucket.synthetic_subject_marker_files++;
        } catch { info.unreadable_entries++; }
      }
    }
  }
  return report;
}

async function main() {
  // Guard against accidental use on the owner's workstation or a live profile.
  if (process.env.GITHUB_ACTIONS !== "true" || process.env.MEDSCALE_CI_WEBVIEW_AUDIT !== "1" ||
      process.platform !== "win32" || !process.env.RUNNER_TEMP || !process.env.LOCALAPPDATA) {
    throw new Error("Disposable Windows GitHub runner and explicit CI profile-audit opt-in required");
  }
  const report = await auditProfiles([
    { label: "tauri_identifier_profile", root: path.join(process.env.LOCALAPPDATA, "org.medscale.desktop.preview") },
    { label: "legacy_medscale_profile", root: path.join(process.env.LOCALAPPDATA, "MedScale") },
  ]);
  const output = path.join("target", "e2e-shots", "webview-disk-profile-audit.json");
  await mkdir(path.dirname(output), { recursive: true });
  await writeFile(output, JSON.stringify(report, null, 2) + "\n", "utf8");
  console.log("WEBVIEW_SYNTHETIC_DISK_AUDIT=" + JSON.stringify({
    scope: report.scope,
    profiles: report.profiles.map(({ label, exists, files_examined, bytes_examined, bounded, unreadable_entries }) =>
      ({ label, exists, files_examined, bytes_examined, bounded, unreadable_entries })),
  }));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(() => {
    // No native error path or byte content in public logs.
    console.error("WEBVIEW_SYNTHETIC_DISK_AUDIT=UNQUALIFIED");
    process.exitCode = 1;
  });
}
