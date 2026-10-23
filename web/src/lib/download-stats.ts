/**
 * Aggregate real GitHub Release asset downloads, NOT installations or unique users.
 * No cookies, trackers, client identifier or telemetry are sent to MedScale.
 * Count only named Beta installer assets; exclude SBOM, source, and receipts.
 */
export interface InstallerDownloadStats {
  total: number;
  windows: number;
  macos: number;
}
const githubApi = "https://api.github.com/repos/AbdulazizShehri/MedScale/releases";
const releaseTag = /^engineering-preview-[a-f0-9]{12}$/;
const installers = {
  windows: "MedScale-engineering-preview-windows-x64.exe",
  macos: "MedScale-engineering-preview-macos.dmg",
} as const;
const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

/** Fail closed for damaged stats rather than display a fabricated download count. */
export function countInstallerDownloads(releases: unknown): InstallerDownloadStats | null {
  if (!Array.isArray(releases)) return null;
  let windows = 0;
  let macos = 0;
  for (const release of releases) {
    if (!isRecord(release)) return null;
    if (release.draft !== false || release.prerelease !== true || typeof release.tag_name !== "string" || !releaseTag.test(release.tag_name)) continue;
    if (!Array.isArray(release.assets)) return null;
    for (const asset of release.assets) {
      if (!isRecord(asset)) return null;
      const platform = asset.name === installers.windows ? "windows" : asset.name === installers.macos ? "macos" : null;
      if (!platform) continue;
      const expected = `https://github.com/AbdulazizShehri/MedScale/releases/download/${release.tag_name}/${installers[platform]}`;
      const downloads = asset.download_count;
      if (asset.browser_download_url !== expected || !Number.isSafeInteger(downloads) || typeof downloads !== "number" || downloads < 0) return null;
      if (platform === "windows") windows += downloads;
      else macos += downloads;
    }
  }
  if (!Number.isSafeInteger(windows + macos)) return null;
  return { total: windows + macos, windows, macos };
}

/**
 * GitHub paginates at 100 releases. Read all pages (bounded) or show unavailable,
 * never silently present a truncated count. Uncached fetch on page visit.
 */
export async function fetchInstallerDownloads(signal?: AbortSignal): Promise<InstallerDownloadStats | null> {
  const all: unknown[] = [];
  for (let page = 1; page <= 20; page++) {
    const response = await fetch(`${githubApi}?per_page=100&page=${page}`, {
      method: "GET",
      headers: { Accept: "application/vnd.github+json" },
      cache: "no-store",
      signal,
    });
    if (!response.ok) return null;
    const result: unknown = await response.json();
    if (!Array.isArray(result)) return null;
    all.push(...result);
    if (result.length < 100) return countInstallerDownloads(all);
  }
  return null;
}
