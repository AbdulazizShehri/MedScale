// Platform-aware installer selection for GitHub Releases.
//
// Adapted from TheHalfMoon/CommunityFinance-CoFi site/app.js
// (commit 6f8d620d90d6b799110d2987b9a10e4e66133a43): asset scoring per platform,
// architecture detection, and refusing to guess between architecture-specific
// installers. MedScale distinguishes qualified signed production releases from the
// separately identified, unsigned synthetic-only Beta installer channel.

export type Platform = "windows" | "macos" | "linux" | "unknown";
export type Arch = "x64" | "arm64" | "x86" | "generic";
export interface ReleaseAsset { name: string; browser_download_url: string; size: number }

/**
 * Release qualification gate. A tag is listed here only when its installers are
 * signed (Windows), signed and notarized (macOS), and qualified for public use.
 * v0.1.0 exists only as an unsigned engineering preview, so this list is empty.
 */
export const QUALIFIED_RELEASE_TAGS: readonly string[] = [];
export const releaseQualified = QUALIFIED_RELEASE_TAGS.length > 0;

/**
 * Separate, unqualified download channel for synthetic-only engineering
 * testers. A preview never satisfies QUALIFIED_RELEASE_TAGS.
 */
export interface UnsignedPreviewAsset { name: string; browser_download_url: string; size: number }
export interface UnsignedPreviewRelease {
  tag_name: string;
  prerelease: boolean;
  draft: boolean;
  assets: UnsignedPreviewAsset[];
}
export type UnsignedPreview = { tag: string; windows?: UnsignedPreviewAsset; macos?: UnsignedPreviewAsset };
/** Last directly verified, immutable-tag Beta release. This is a safe fallback
 * when browser-side GitHub API requests are rate-limited or inaccessible.
 * Update only after independently verifying published EXE and DMG assets.
 * This does not qualify the unsigned installers for clinical use. */
export const VERIFIED_BETA_PREVIEW: UnsignedPreview = {
  tag: "engineering-preview-19d419dd7b15",
  windows: { name: "MedScale-engineering-preview-windows-x64.exe", size: 13872773, browser_download_url: "https://github.com/AbdulazizShehri/MedScale/releases/download/engineering-preview-19d419dd7b15/MedScale-engineering-preview-windows-x64.exe" },
  macos: { name: "MedScale-engineering-preview-macos.dmg", size: 20827457, browser_download_url: "https://github.com/AbdulazizShehri/MedScale/releases/download/engineering-preview-19d419dd7b15/MedScale-engineering-preview-macos.dmg" },
};


export function selectUnsignedPreview(releases: readonly UnsignedPreviewRelease[]): UnsignedPreview | undefined {
  for (const release of releases) {
    if (!release || !release.prerelease || release.draft || !/^engineering-preview-[0-9a-f]{12}$/.test(release.tag_name)) continue;
    const prefix = `https://github.com/AbdulazizShehri/MedScale/releases/download/${release.tag_name}/`;
    const find = (filename: string): UnsignedPreviewAsset | undefined => release.assets?.find(
      (asset) => asset.name === filename && asset.browser_download_url === prefix + filename && Number.isFinite(asset.size) && asset.size > 0
    );
    const windows = find("MedScale-engineering-preview-windows-x64.exe");
    const macos = find("MedScale-engineering-preview-macos.dmg");
    if (windows || macos) return { tag: release.tag_name, windows, macos };
  }
  return undefined;
}

export function detectPlatform(platform: string): Platform {
  const p = platform.toLowerCase();
  // macOS first: "darwin" contains "win".
  if (p.includes("mac") || p.includes("darwin")) return "macos";
  if (p.includes("win")) return "windows";
  if (p.includes("linux") && !p.includes("android")) return "linux";
  return "unknown";
}

export function scoreAsset(name: string, platform: Platform): number {
  const n = name.toLowerCase();
  if (n.endsWith(".sig") || n.endsWith(".sha256") || n.includes("sbom")) return 0;
  if (platform === "windows") {
    if (n.endsWith(".msi")) return 45;
    if (n.endsWith(".exe")) return 40;
  }
  if (platform === "macos") {
    if (n.endsWith(".dmg")) return 50;
    if (n.endsWith(".pkg")) return 45;
  }
  if (platform === "linux") {
    if (n.endsWith(".appimage")) return 50;
    if (n.endsWith(".deb")) return 45;
    if (n.endsWith(".rpm")) return 40;
  }
  return 0;
}

export function assetArch(name: string): Arch {
  const n = name.toLowerCase();
  if (/arm64|aarch64/.test(n)) return "arm64";
  if (/x86_64|amd64|x64/.test(n)) return "x64";
  if (/x86|win32|i[3-6]86/.test(n)) return "x86";
  return "generic";
}

/**
 * The best installer for a platform. With several equally scored candidates,
 * the visitor's architecture decides when it is known; otherwise a single
 * generic build is used, and anything else is ambiguous (undefined).
 */
export function pickInstaller(assets: readonly ReleaseAsset[], platform: Platform, arch?: Arch): ReleaseAsset | undefined {
  const ranked = assets
    .map((asset) => ({ asset, score: scoreAsset(asset.name, platform), arch: assetArch(asset.name) }))
    .filter((c) => c.score > 0)
    .sort((a, b) => b.score - a.score);
  if (ranked.length === 0) return undefined;
  const best = ranked.filter((c) => c.score === ranked[0].score);
  if (best.length === 1) return best[0].asset;
  if (arch) {
    const match = best.filter((c) => c.arch === arch);
    if (match.length === 1) return match[0].asset;
  }
  const generic = best.filter((c) => c.arch === "generic");
  return generic.length === 1 ? generic[0].asset : undefined;
}

export function formatSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "";
  const mb = bytes / 1024 / 1024;
  return `${mb.toFixed(mb >= 100 ? 0 : 1)} MB`;
}

export function platformLabel(platform: Platform): string {
  return platform === "windows" ? "Windows" : platform === "macos" ? "macOS" : platform === "linux" ? "Linux" : "your platform";
}

/** Today's truthful status for each platform, independent of any release fetch. */
export const platformStatus = {
  windows: { surface: "Desktop app + CLI", arch: "x64", state: "Unsigned Beta", note: "Direct EXE download for synthetic-only evaluation. Not a signed or production-qualified release." },
  macos: { surface: "Desktop app + CLI", arch: "Apple silicon", state: "Unsigned Beta", note: "Direct DMG download for synthetic-only evaluation. Ad-hoc signed; not notarized or production-qualified." },
  linux: { surface: "CLI", arch: "x86_64", state: "CLI from source", note: "The Rust Core and CLI are tested on Linux. A Linux desktop app is not available yet." },
} as const;
