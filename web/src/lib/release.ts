// Platform-aware installer selection for GitHub Releases.
//
// Adapted from TheHalfMoon/CommunityFinance-CoFi site/app.js
// (commit 6f8d620d90d6b799110d2987b9a10e4e66133a43): asset scoring per platform,
// architecture detection, and refusing to guess between architecture-specific
// installers. MedScale adds a qualification gate: nothing is fetched or linked
// unless a release is explicitly recorded as qualified below.

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

export function detectPlatform(platform: string): Platform {
  const p = platform.toLowerCase();
  if (p.includes("win")) return "windows";
  if (p.includes("mac") || p.includes("darwin")) return "macos";
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
  windows: { surface: "Desktop app + CLI", arch: "x64", state: "Build from source", note: "An unsigned engineering-preview installer is built by CI for testing. No signed public installer yet." },
  macos: { surface: "Desktop app + CLI", arch: "Apple silicon", state: "Build from source", note: "The engineering-preview app is ad-hoc signed and not notarized. No public installer yet." },
  linux: { surface: "CLI", arch: "x86_64", state: "CLI from source", note: "The Rust Core and CLI are tested on Linux. A Linux desktop app is not available yet." },
} as const;
