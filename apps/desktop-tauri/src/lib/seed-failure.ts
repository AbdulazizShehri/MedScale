/** Spec 108 step 2: user guidance for workspace failures, keyed on the stable
 * reason codes in host.rs `seed_reason`. A busy or unreadable workspace is
 * never described as damaged, and no text includes paths or identifiers.
 * "Nothing has been changed or deleted" appears only for reasons produced by
 * the read-only preflight, before Core opens or writes anything. */
export const SEED_REASONS = [
  "seed_manifest_missing",
  "seed_manifest_invalid",
  "seed_manifest_incomplete",
  "core_ledger_incomplete",
  "workspace_io_failure",
  "workspace_busy",
  "writer_active",
  "seed_vault_missing",
  "seed_ledger_undecodable",
  "seed_ledger_interrupted",
  "core_integrity_error",
  "seed_recovery_refused",
  "seed_stale_sidecar",
] as const;

export type SeedReason = (typeof SEED_REASONS)[number];

const KNOWN: ReadonlySet<string> = new Set(SEED_REASONS);

export function parseSeedReason(raw: unknown): SeedReason | undefined {
  return typeof raw === "string" && KNOWN.has(raw) ? (raw as SeedReason) : undefined;
}

export type SeedGuidance = {
  /** retry: nothing is wrong with the data; try again.
   *  incomplete: the sample was not fully set up; MedScale refuses to reuse it.
   *  stopped: Core refused to continue; no claim about earlier writes. */
  category: "retry" | "incomplete" | "stopped";
  title: string;
  detail: string;
};

const UNCHANGED = "Nothing has been changed or deleted.";

export function seedFailureGuidance(reason: SeedReason | undefined): SeedGuidance | null {
  switch (reason) {
    case "workspace_busy":
    case "writer_active":
      return {
        category: "retry",
        title: "This workspace is in use.",
        detail: "Another MedScale window or process is using it. Close other MedScale windows, then try again.",
      };
    case "workspace_io_failure":
      return {
        category: "retry",
        title: "This workspace could not be read.",
        detail: `MedScale could not read its own data folder, for example because of file permissions or security software. Try again. ${UNCHANGED}`,
      };
    case "seed_manifest_missing":
    case "seed_manifest_incomplete":
    case "seed_vault_missing":
    case "core_ledger_incomplete":
      return {
        category: "incomplete",
        title: "The sample workspace was not finished setting up.",
        detail: `MedScale may have closed while preparing it, or another MedScale window may still be preparing it. To avoid duplicating records, MedScale will not reuse it. ${UNCHANGED}`,
      };
    case "seed_manifest_invalid":
      return {
        category: "incomplete",
        title: "The sample workspace's setup record could not be read.",
        detail: `It may have been cut off when MedScale closed. To avoid duplicating records, MedScale will not reuse the sample. ${UNCHANGED}`,
      };
    case "seed_ledger_undecodable":
      return {
        category: "incomplete",
        title: "The sample workspace's stored data could not be decoded.",
        detail: `To avoid damaging or duplicating records, MedScale will not reuse it. ${UNCHANGED}`,
      };
    case "seed_ledger_interrupted":
      return {
        category: "incomplete",
        title: "The sample workspace was interrupted while saving.",
        detail: `MedScale does not repair this automatically yet. To avoid duplicating records, it will not reuse the sample. ${UNCHANGED}`,
      };
    case "seed_recovery_refused":
      return {
        category: "incomplete",
        title: "The sample workspace could not be rebuilt automatically.",
        detail: "MedScale found data it could not prove was only the bundled sample, so it left that data in place. Nothing that could be yours was changed or deleted.",
      };
    case "seed_stale_sidecar":
      return {
        category: "incomplete",
        title: "Settings from an earlier sample workspace remain.",
        detail: `MedScale will not start a new sample over them. ${UNCHANGED}`,
      };
    case "core_integrity_error":
      return {
        category: "stopped",
        title: "MedScale stopped because stored data failed an integrity check.",
        detail: "It did not continue, to protect your records. Please report this from About → Report a problem, using synthetic examples only.",
      };
    default:
      return null;
  }
}

/** Spec 108 step 4: true only when `workspace_open_synthetic` reports that an
 * interrupted first seed was proven fixture-only and rebuilt. */
export function recoveredInterruptedSeed(data: unknown): boolean {
  return typeof data === "object" && data !== null
    && (data as { recovered_interrupted_seed?: unknown }).recovered_interrupted_seed === true;
}

export const RECOVERED_SEED_NOTICE = {
  title: "Sample workspace rebuilt",
  detail: "It had not finished setting up the last time MedScale ran, so it was set up again from the bundled sample.",
} as const;
