// Builds the site's release manifest from a release candidate's
// artifact-inventory.json (scripts/assemble-release-candidate.py), so the
// download page lists exactly the qualified files after the founder publishes
// them. Pure: no network, no file access. validateRelease checks the result.
import { DOWNLOADS, REPOSITORY, validateRelease, type InstallerPlatform, type PlatformRelease, type PublicRelease, type SigningState } from "./release.ts";

export interface InventoryArtifact {
  name: string;
  platform: string;
  arch: string;
  kind: string;
  size: number;
  sha256: string;
  signing: string;
}

export interface Inventory {
  schema: number;
  version: string;
  source_sha: string;
  run: string;
  artifacts: InventoryArtifact[];
}

export interface GeneratorInput {
  tag: string;
  /** The founder's comment approving these exact files. */
  founderApproval: string;
  /** The founder's comment with the SIGNING lines; defaults to the approval. */
  signingDecision?: string;
}

/** The notices archive that covers each CLI target. */
const CLI_NOTICES: Record<string, [string, string]> = {
  "x86_64-pc-windows-msvc": ["windows", "x64"],
  "aarch64-apple-darwin": ["macos", "arm64"],
  "x86_64-apple-darwin": ["macos", "x64"],
  "x86_64-unknown-linux-gnu": ["linux", "x64"],
  "aarch64-unknown-linux-gnu": ["linux", "arm64"],
};

function signingState(text: string): SigningState {
  if (text === "unsigned") return "unsigned";
  if (text.startsWith("ad-hoc")) return "ad-hoc";
  throw new Error(`unknown signing state in the inventory: ${text}`);
}

export const CHECKSUMS_FILE = "SHA256SUMS";

export function manifestFromInventory(inventory: Inventory, input: GeneratorInput): { schema: 1; release: PublicRelease } {
  const version = input.tag.replace(/^v/, "");
  if (version !== inventory.version) {
    throw new Error(`the tag ${input.tag} doesn't match the candidate's version ${inventory.version}`);
  }
  const sbom = inventory.artifacts.filter((a) => a.kind === "sbom");
  if (sbom.length !== 1) throw new Error(`expected one SBOM in the inventory, found ${sbom.length}`);
  const companion = (kind: string, platform: string, arch: string) => {
    const found = inventory.artifacts.filter((a) => a.kind === kind && a.platform === platform && a.arch === arch);
    const what = kind === "notices" ? "notices archive" : kind;
    if (found.length !== 1) throw new Error(`expected one ${what} for ${platform} ${arch}, found ${found.length}`);
    return found[0].name;
  };
  const notices = (platform: string, arch: string) => companion("notices", platform, arch);
  const PACKAGED = ["installer", "dmg", "deb"];
  const platforms: Partial<Record<InstallerPlatform, PlatformRelease>> = {};
  for (const a of inventory.artifacts) {
    if (["sbom", "notices", "package-manifest", "platform-sbom", "platform-notice"].includes(a.kind)) continue;
    const matches = (Object.keys(DOWNLOADS) as InstallerPlatform[]).filter((k) => {
      const m = DOWNLOADS[k].name.exec(a.name);
      return m !== null && m[1] === inventory.version.split("-")[0];
    });
    if (matches.length !== 1) throw new Error(`${a.name} matches ${matches.length} download rules`);
    const key = matches[0];
    if (platforms[key]) throw new Error(`two files for ${key}`);
    const [np, na] = a.platform === "cli" ? CLI_NOTICES[a.arch] ?? [] : [a.platform, a.arch];
    if (!np) throw new Error(`no notices mapping for CLI target ${a.arch}`);
    platforms[key] = {
      fileName: a.name,
      url: `https://github.com/${REPOSITORY}/releases/download/${input.tag}/${a.name}`,
      sha256: a.sha256,
      size: a.size,
      signing: signingState(a.signing),
      signingDecision: input.signingDecision ?? input.founderApproval,
      qualificationRun: inventory.run,
      // A packaged installer is checked against its own package manifest,
      // SBOM and NOTICE; anything else against the checksums file.
      companions: PACKAGED.includes(a.kind)
        ? {
            checksums: CHECKSUMS_FILE,
            sbom: companion("platform-sbom", a.platform, a.arch),
            notices: companion("platform-notice", a.platform, a.arch),
            packageManifest: companion("package-manifest", a.platform, a.arch),
          }
        : { checksums: CHECKSUMS_FILE, sbom: sbom[0].name, notices: notices(np, na) },
    };
  }
  return {
    schema: 1,
    release: {
      tag: input.tag,
      version,
      sourceSha: inventory.source_sha,
      founderApproval: input.founderApproval,
      notes: [
        "Unsigned beta: Windows SmartScreen warns, and Smart App Control blocks it; macOS needs Open Anyway once.",
        "Synthetic workspaces and licensed nonpatient audio only. Real patient data and clinical decisions are not authorized.",
      ],
      platforms,
    },
  };
}

/** The manifest, checked as the site will check it with this tag qualified. */
export function checkedManifest(inventory: Inventory, input: GeneratorInput) {
  const manifest = manifestFromInventory(inventory, input);
  const { errors } = validateRelease(manifest, [input.tag]);
  if (errors.length) throw new Error("the generated manifest is invalid:\n- " + errors.join("\n- "));
  return manifest;
}
