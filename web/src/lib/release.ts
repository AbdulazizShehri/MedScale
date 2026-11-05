// Platform-aware installer selection for GitHub Releases.
//
// Adapted from TheHalfMoon/CommunityFinance-CoFi site/app.js
// (commit 6f8d620d90d6b799110d2987b9a10e4e66133a43): asset scoring per platform,
// architecture detection, and refusing to guess between architecture-specific
// installers. MedScale adds a qualification gate: nothing is fetched or linked
// unless a release is explicitly recorded as qualified below.
import manifest from "./release-manifest.json" with { type: "json" };

export type Platform = "windows" | "macos" | "linux" | "unknown";
export type Arch = "x64" | "arm64" | "x86" | "generic";
export interface ReleaseAsset { name: string; browser_download_url: string; size: number }

/**
 * Release qualification gate. A tag is listed here only after its installers pass
 * release qualification and the founder approves those exact artifacts. Each
 * installer's signing state is recorded per platform in release-manifest.json and
 * shown to visitors, so the gate never implies a signature that doesn't exist.
 * Nothing is approved yet, so this list is empty.
 */
export const QUALIFIED_RELEASE_TAGS: readonly string[] = [];

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
  windows: { surface: "Desktop app + CLI", arch: "x64", state: "Build from source", note: "An unsigned engineering-preview installer is built by CI for testing. No signed public installer yet." },
  macos: { surface: "Desktop app + CLI", arch: "Apple silicon", state: "Build from source", note: "The engineering-preview app is ad-hoc signed and not notarized. No public installer yet." },
  linux: { surface: "CLI", arch: "x86_64", state: "CLI from source", note: "The Rust Core and CLI are tested on Linux. A Linux desktop app is not available yet." },
} as const;

// Direct downloads. The site is static: release-manifest.json is reviewed and built
// in, and no release API is called. A platform is offered only if its entry passes
// every check below; one bad platform never hides or taints another.

export const REPOSITORY = "AbdulazizShehri/MedScale";
export type InstallerPlatform = "windows" | "macos";
export type SigningState = "authenticode" | "unsigned" | "developer-id-notarized" | "ad-hoc";

export interface PlatformRelease {
  fileName: string;
  url: string;
  sha256: string;
  size: number;
  signing: SigningState;
  /** The founder's comment accepting this signing state for this release. */
  signingDecision: string;
  /** The workflow run that qualified these exact bytes. */
  qualificationRun: string;
  companions: { checksums: string; sbom: string; notices: string; packageManifest: string };
}

export interface PublicRelease {
  tag: string;
  version: string;
  sourceSha: string;
  /** The founder's comment approving these exact artifacts for publication. */
  founderApproval: string;
  notes: string[];
  platforms: Partial<Record<InstallerPlatform, PlatformRelease>>;
}

export interface ReleaseValidation {
  release?: PublicRelease;
  errors: string[];
}

const INSTALLERS: Record<InstallerPlatform, { name: RegExp; arch: Arch; signing: readonly SigningState[] }> = {
  windows: { name: /^MedScale_(\d+\.\d+\.\d+)_x64-setup\.exe$/, arch: "x64", signing: ["authenticode", "unsigned"] },
  macos: { name: /^MedScale_(\d+\.\d+\.\d+)_aarch64\.dmg$/, arch: "arm64", signing: ["developer-id-notarized", "ad-hoc"] },
};
const TAG = /^v(\d+\.\d+\.\d+)-beta\.\d+$/;
const SAFE_FILE = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;
const COMMENT = new RegExp(`^https://github\\.com/${REPOSITORY}/(issues|pull)/\\d+#issuecomment-\\d+$`);
const RUN = new RegExp(`^https://github\\.com/${REPOSITORY}/actions/runs/\\d+$`);
const MAX_INSTALLER_BYTES = 1024 * 1024 * 1024;
const MIN_INSTALLER_BYTES = 1024 * 1024;

export function releaseAssetUrl(tag: string, fileName: string): string {
  return `https://github.com/${REPOSITORY}/releases/download/${tag}/${fileName}`;
}

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function platformErrors(key: InstallerPlatform, raw: Record<string, unknown>, tag: string, baseVersion: string): string[] {
  const rule = INSTALLERS[key];
  const errors: string[] = [];
  const name = typeof raw.fileName === "string" ? raw.fileName : "";
  const match = rule.name.exec(name);
  if (!match) errors.push("unexpected installer file name");
  else if (match[1] !== baseVersion) errors.push("installer version differs from the tag");
  if (raw.url !== releaseAssetUrl(tag, name)) errors.push("url is not this release's immutable asset");
  if (typeof raw.sha256 !== "string" || !/^[0-9a-f]{64}$/.test(raw.sha256)) errors.push("sha256 is missing or malformed");
  if (!Number.isInteger(raw.size) || (raw.size as number) < MIN_INSTALLER_BYTES || (raw.size as number) > MAX_INSTALLER_BYTES) errors.push("size is missing or implausible");
  if (!rule.signing.includes(raw.signing as SigningState)) errors.push("signing state is missing or invalid for this platform");
  if (typeof raw.signingDecision !== "string" || !COMMENT.test(raw.signingDecision)) errors.push("signing decision is missing");
  if (typeof raw.qualificationRun !== "string" || !RUN.test(raw.qualificationRun)) errors.push("qualification run is missing");
  const companions = isRecord(raw.companions) ? raw.companions : {};
  for (const c of ["checksums", "sbom", "notices", "packageManifest"]) {
    const v = companions[c];
    if (typeof v !== "string" || !SAFE_FILE.test(v)) errors.push(`${c} file is missing`);
  }
  if (match && pickInstaller([{ name, browser_download_url: String(raw.url), size: Number(raw.size) }], key, rule.arch)?.name !== name) errors.push("installer selection disagrees");
  return errors;
}

/**
 * The release visitors may be offered, or nothing. Refuses anything outside the
 * gate, any URL other than this tag's exact asset, and any platform without a
 * checksum, a size, a qualification run or a recorded signing decision.
 */
export function validateRelease(raw: unknown, qualifiedTags: readonly string[] = QUALIFIED_RELEASE_TAGS): ReleaseValidation {
  if (!isRecord(raw) || raw.schema !== 1) return { errors: ["unknown manifest schema"] };
  const r = raw.release;
  if (r === null) return { errors: [] };
  if (!isRecord(r)) return { errors: ["release is malformed"] };
  const tag = typeof r.tag === "string" ? r.tag : "";
  const tagMatch = TAG.exec(tag);
  const errors: string[] = [];
  if (!tagMatch) errors.push("tag is malformed");
  else if (!qualifiedTags.includes(tag)) errors.push(`tag ${tag} is not in the qualification gate`);
  if (r.version !== tag.slice(1)) errors.push("version differs from the tag");
  if (typeof r.sourceSha !== "string" || !/^[0-9a-f]{40}$/.test(r.sourceSha)) errors.push("source SHA is missing or malformed");
  if (typeof r.founderApproval !== "string" || !COMMENT.test(r.founderApproval)) errors.push("founder approval is missing");
  if (!Array.isArray(r.notes) || r.notes.length === 0 || !r.notes.every((n) => typeof n === "string" && n.length > 0)) errors.push("release notes are missing");
  if (errors.length > 0 || !tagMatch) return { errors };

  const platforms: Partial<Record<InstallerPlatform, PlatformRelease>> = {};
  const given = isRecord(r.platforms) ? r.platforms : {};
  for (const key of Object.keys(given)) {
    if (key !== "windows" && key !== "macos") { errors.push(`${key}: not a supported installer platform`); continue; }
    const entry = given[key];
    const problems = isRecord(entry) ? platformErrors(key, entry, tag, tagMatch[1]) : ["entry is malformed"];
    if (problems.length > 0) errors.push(...problems.map((p) => `${key}: ${p}`));
    else platforms[key] = entry as unknown as PlatformRelease;
  }
  if (Object.keys(platforms).length === 0) return { errors: errors.length ? errors : ["no platform is offered"] };
  return { errors, release: { ...(r as unknown as PublicRelease), platforms } };
}

/** The reviewed release built into this site, if any part of it is valid. */
export const publicRelease: PublicRelease | undefined = validateRelease(manifest).release;
/** What to call the product today. */
export const productStage = publicRelease ? "Beta" : "Engineering preview";

export function installerFor(release: PublicRelease | undefined, platform: Platform): PlatformRelease | undefined {
  return platform === "windows" || platform === "macos" ? release?.platforms[platform] : undefined;
}

/** Plain signing status, derived from the state so a manifest can't word its way past it. */
export function signingNote(signing: SigningState): string {
  switch (signing) {
    case "authenticode": return "Code-signed for Windows.";
    case "unsigned": return "Not code-signed. Windows SmartScreen may warn before it runs.";
    case "developer-id-notarized": return "Signed with Apple Developer ID and notarized.";
    case "ad-hoc": return "Ad-hoc signed and not notarized. macOS blocks the first open until you approve it.";
  }
}
