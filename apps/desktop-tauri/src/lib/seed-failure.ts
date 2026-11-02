/** Spec 108 step 2: user guidance for synthetic workspace failures, keyed on
 * the stable reason codes from host.rs `seed_reason`. A busy or unreadable
 * workspace is never described as damaged, and no text includes paths or
 * identifiers. */
export const SEED_REASONS = [
  "seed_manifest_missing",
  "seed_manifest_invalid",
  "seed_manifest_incomplete",
  "core_ledger_incomplete",
  "workspace_io_failure",
  "workspace_busy",
  "writer_active",
  "core_corrupt",
] as const;

export type SeedReason = (typeof SEED_REASONS)[number];

const KNOWN: ReadonlySet<string> = new Set(SEED_REASONS);

export function parseSeedReason(raw: unknown): SeedReason | undefined {
  return typeof raw === "string" && KNOWN.has(raw) ? (raw as SeedReason) : undefined;
}

export type SeedGuidance = {
  /** "retry": nothing is wrong with the data; try again later.
   *  "incomplete": the sample was not fully set up and MedScale refuses to replay it. */
  category: "retry" | "incomplete";
  title: string;
  detail: string;
};

export function seedFailureGuidance(reason: SeedReason | undefined): SeedGuidance | null {
  switch (reason) {
    case "workspace_busy":
    case "writer_active":
      return {
        category: "retry",
        title: "This workspace is in use.",
        detail: "Another MedScale window or process is using it. Close other MedScale windows, then try again. Nothing was changed.",
      };
    case "workspace_io_failure":
      return {
        category: "retry",
        title: "This workspace could not be read.",
        detail: "MedScale could not read its own data folder, for example because of file permissions or security software. Nothing was changed; try again.",
      };
    case "seed_manifest_missing":
    case "seed_manifest_invalid":
    case "seed_manifest_incomplete":
    case "core_corrupt":
      // Also produced for encrypted vaults (Core Corrupt/DigestMismatch), so
      // the text must not assume the sample workspace.
      return {
        category: "incomplete",
        title: "This workspace's stored data could not be decoded.",
        detail: "MedScale refuses to use it so that no records are damaged or duplicated; nothing has been changed or deleted.",
      };
    case "core_ledger_incomplete":
      return {
        category: "incomplete",
        title: "The sample workspace was not finished setting up.",
        detail: "This can happen if MedScale closed while preparing the sample. To avoid duplicating records, MedScale will not reuse it, and nothing has been changed or deleted.",
      };
    default:
      return null;
  }
}
