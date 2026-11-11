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
  windows: { surface: "Desktop app + CLI", arch: "x64", state: "Not released yet", note: "CI builds and tests an unsigned per-user installer and a portable zip. Nothing is published until a release is approved." },
  macos: { surface: "Desktop app + CLI", arch: "Apple silicon and Intel", state: "Not released yet", note: "CI builds and tests DMGs that are ad-hoc signed and not notarized. Nothing is published until a release is approved." },
  linux: { surface: "Desktop app + CLI", arch: "x86_64 and arm64", state: "Not released yet", note: "CI builds and tests .deb packages and AppImages. Nothing is published until a release is approved." },
} as const;

// Direct downloads. The site is static: release-manifest.json is reviewed and built
// in, and no release API is called. A platform is offered only if its entry passes
// every check below; one bad platform never hides or taints another.

export const REPOSITORY = "AbdulazizShehri/MedScale";
/** Every downloadable file the gate knows. Anything else is refused. */
export type InstallerPlatform =
  | "windows" | "windows-portable" | "macos" | "macos-intel"
  | "linux-deb" | "linux-appimage" | "linux-arm64-deb" | "linux-arm64-appimage"
  | "cli-windows-x64" | "cli-macos-arm64" | "cli-macos-x64" | "cli-linux-x64" | "cli-linux-arm64";
export type DownloadGroup = "windows" | "macos" | "linux" | "cli";
export type DownloadKind = "installer" | "portable" | "dmg" | "deb" | "appimage" | "cli";
export type SigningState = "authenticode" | "unsigned" | "developer-id-notarized" | "ad-hoc";
type Companions = { checksums: string; sbom: string; notices: string; packageManifest?: string };

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
  companions: Companions;
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

interface DownloadRule {
  name: RegExp;
  group: DownloadGroup;
  kind: DownloadKind;
  arch: Arch;
  signing: readonly SigningState[];
  /** Platform named in this file's package manifest, for packaged installers. */
  manifestPlatform?: "windows" | "macos" | "linux";
}
const V = "(\\d+\\.\\d+\\.\\d+)";
const rx = (pattern: string) => new RegExp(`^${pattern.replace("{v}", V)}$`);
const WIN: readonly SigningState[] = ["authenticode", "unsigned"];
const MAC: readonly SigningState[] = ["developer-id-notarized", "ad-hoc"];
const NONE: readonly SigningState[] = ["unsigned"];
export const DOWNLOADS: Record<InstallerPlatform, DownloadRule> = {
  windows: { name: rx("MedScale_{v}_x64-setup\\.exe"), group: "windows", kind: "installer", arch: "x64", signing: WIN, manifestPlatform: "windows" },
  "windows-portable": { name: rx("MedScale_{v}_x64-portable\\.zip"), group: "windows", kind: "portable", arch: "x64", signing: WIN },
  macos: { name: rx("MedScale_{v}_aarch64\\.dmg"), group: "macos", kind: "dmg", arch: "arm64", signing: MAC, manifestPlatform: "macos" },
  "macos-intel": { name: rx("MedScale_{v}_x64\\.dmg"), group: "macos", kind: "dmg", arch: "x64", signing: MAC, manifestPlatform: "macos" },
  "linux-deb": { name: rx("MedScale_{v}_amd64\\.deb"), group: "linux", kind: "deb", arch: "x64", signing: NONE, manifestPlatform: "linux" },
  "linux-appimage": { name: rx("MedScale_{v}_amd64\\.AppImage"), group: "linux", kind: "appimage", arch: "x64", signing: NONE },
  "linux-arm64-deb": { name: rx("MedScale_{v}_arm64\\.deb"), group: "linux", kind: "deb", arch: "arm64", signing: NONE, manifestPlatform: "linux" },
  "linux-arm64-appimage": { name: rx("MedScale_{v}_aarch64\\.AppImage"), group: "linux", kind: "appimage", arch: "arm64", signing: NONE },
  "cli-windows-x64": { name: rx("medscale-{v}-x86_64-pc-windows-msvc\\.zip"), group: "cli", kind: "cli", arch: "x64", signing: WIN },
  "cli-macos-arm64": { name: rx("medscale-{v}-aarch64-apple-darwin\\.tar\\.gz"), group: "cli", kind: "cli", arch: "arm64", signing: NONE },
  "cli-macos-x64": { name: rx("medscale-{v}-x86_64-apple-darwin\\.tar\\.gz"), group: "cli", kind: "cli", arch: "x64", signing: NONE },
  "cli-linux-x64": { name: rx("medscale-{v}-x86_64-unknown-linux-gnu\\.tar\\.gz"), group: "cli", kind: "cli", arch: "x64", signing: NONE },
  "cli-linux-arm64": { name: rx("medscale-{v}-aarch64-unknown-linux-gnu\\.tar\\.gz"), group: "cli", kind: "cli", arch: "arm64", signing: NONE },
};
const PACKAGED: readonly DownloadKind[] = ["installer", "dmg", "deb"];
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
  const rule = DOWNLOADS[key];
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
  for (const c of PACKAGED.includes(rule.kind) ? ["checksums", "sbom", "notices", "packageManifest"] : ["checksums", "sbom", "notices"]) {
    const v = companions[c];
    if (typeof v !== "string" || !SAFE_FILE.test(v)) errors.push(`${c} file is missing`);
  }
  if (match && (rule.kind === "installer" || rule.kind === "dmg" || rule.kind === "deb" || rule.kind === "appimage")
    && pickInstaller([{ name, browser_download_url: String(raw.url), size: Number(raw.size) }], rule.group as Platform, rule.arch)?.name !== name) errors.push("installer selection disagrees");
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
    if (!Object.hasOwn(DOWNLOADS, key)) { errors.push(`${key}: not a supported installer platform`); continue; }
    const entry = given[key];
    const k = key as InstallerPlatform;
    const problems = isRecord(entry) ? platformErrors(k, entry, tag, tagMatch[1]) : ["entry is malformed"];
    if (problems.length > 0) errors.push(...problems.map((p) => `${key}: ${p}`));
    else platforms[k] = entry as unknown as PlatformRelease;
  }
  if (Object.keys(platforms).length === 0) return { errors: errors.length ? errors : ["no platform is offered"] };
  return { errors, release: { ...(r as unknown as PublicRelease), platforms } };
}

/** The reviewed release built into this site, if any part of it is valid. */
export const publicRelease: PublicRelease | undefined = validateRelease(manifest).release;
/** What to call the product today. */
export const productStage = publicRelease ? "Beta" : "Engineering preview";

/** The main desktop download for a platform: the installer, the Apple silicon DMG, or the x64 .deb. */
export function installerFor(release: PublicRelease | undefined, platform: Platform): PlatformRelease | undefined {
  const key: InstallerPlatform | undefined = platform === "windows" ? "windows" : platform === "macos" ? "macos" : platform === "linux" ? "linux-deb" : undefined;
  return key ? release?.platforms[key] : undefined;
}

export interface Download { key: InstallerPlatform; rule: DownloadRule; file: PlatformRelease }

/** Every validated download in one group, in a fixed order. */
export function downloadsFor(release: PublicRelease | undefined, group: DownloadGroup): Download[] {
  if (!release) return [];
  return (Object.keys(DOWNLOADS) as InstallerPlatform[])
    .filter((key) => DOWNLOADS[key].group === group && release.platforms[key])
    .map((key) => ({ key, rule: DOWNLOADS[key], file: release.platforms[key] as PlatformRelease }));
}

export function kindLabel(kind: DownloadKind): string {
  return { installer: "Installer (.exe)", portable: "Portable (.zip)", dmg: "Disk image (.dmg)", deb: "Debian/Ubuntu package (.deb)", appimage: "AppImage", cli: "CLI archive" }[kind];
}

export function archLabel(arch: Arch, group: DownloadGroup): string {
  if (group === "macos") return arch === "arm64" ? "Apple silicon" : "Intel";
  return arch === "arm64" ? "arm64" : arch === "x64" ? "x64" : arch;
}

/**
 * What was tested on these exact bytes, derived from the kind of file so the
 * manifest can't word its way past it. The release gate only accepts a file
 * whose qualification run passed these checks.
 */
export function testedNote(kind: DownloadKind): string {
  switch (kind) {
    case "installer": case "dmg": case "deb": return "Installed, opened and uninstalled on a clean CI machine.";
    case "portable": case "appimage": return "Opened without installing on a clean CI machine.";
    case "cli": return "Run from the extracted archive on a clean CI machine, with its tests.";
  }
}

/** Plain signing status, derived from the state so a manifest can't word its way past it. */
export function signingNote(signing: SigningState, group: DownloadGroup = "windows", key?: InstallerPlatform): string {
  switch (signing) {
    case "authenticode": return "Code-signed for Windows.";
    case "unsigned":
      if (group === "windows" || key === "cli-windows-x64") return "Not code-signed. Windows SmartScreen warns before it runs, and Smart App Control blocks it.";
      if (key === "cli-macos-arm64" || key === "cli-macos-x64") return "Not signed or notarized. macOS asks you to approve it before the first run.";
      return "Not signed. Check the SHA-256 before you run it.";
    case "developer-id-notarized": return "Signed with Apple Developer ID and notarized.";
    case "ad-hoc": return "Ad-hoc signed and not notarized. macOS blocks the first open until you approve it.";
  }
}
