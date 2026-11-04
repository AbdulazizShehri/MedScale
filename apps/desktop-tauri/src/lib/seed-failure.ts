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
  "set_aside_slot_occupied",
  "set_aside_incomplete",
  "set_aside_refused",
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
        detail: `MedScale may have closed while preparing it. To avoid duplicating records, MedScale will not reuse it. ${UNCHANGED}`,
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
        detail: `MedScale could not repair this automatically. To avoid duplicating records, it will not reuse the sample. ${UNCHANGED}`,
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
    case "set_aside_slot_occupied":
      return {
        category: "incomplete",
        title: "A previously set-aside sample still exists.",
        detail: "Delete it first to set this one aside. Deleting it is permanent.",
      };
    case "set_aside_incomplete":
      return {
        category: "retry",
        title: "Setting the sample aside did not finish.",
        detail: "Some sample files were moved to the set-aside folder. Try again to finish; nothing was deleted.",
      };
    case "set_aside_refused":
      return {
        category: "retry",
        title: "The sample workspace was not set aside.",
        detail: "This action only applies to a sample workspace that cannot be opened. Nothing was changed.",
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

/** Spec 109: true only when `workspace_unlock` reports that it discarded a
 * session that never sealed. */
export function discardedInterruptedSession(data: unknown): boolean {
  return typeof data === "object" && data !== null
    && (data as { discarded_interrupted_session?: unknown }).discarded_interrupted_session === true;
}

export const DISCARDED_SESSION_NOTICE = {
  title: "Recent changes may be lost",
  detail: "MedScale did not close normally last time. The vault opened from its last locked state, so changes made after that may be gone.",
} as const;

export const RECOVERED_SEED_NOTICE = {
  title: "Sample workspace rebuilt",
  detail: "It had not finished setting up the last time MedScale ran, so it was set up again from the bundled sample.",
} as const;

/** Spec 108 step 5: offer a manual set-aside only for sample-setup states the
 * backend also re-checks; never for integrity errors or set-aside outcomes. */
export function setAsideOffered(reason: SeedReason | undefined): boolean {
  if (reason === "set_aside_incomplete") return true; // retry finishes the move
  if (!reason || reason === "core_integrity_error" || reason.startsWith("set_aside_")) return false;
  return seedFailureGuidance(reason)?.category === "incomplete";
}

export const SET_ASIDE_DIALOG = {
  title: "Set the sample workspace aside?",
  body: "MedScale will move the sample workspace and its settings into a folder inside MedScale's own data, then create a new sample. Nothing is deleted. The set-aside copy is not encrypted, cannot be opened in MedScale, may include FHIR files you imported into the sample, and is kept until you delete it. Your original source files elsewhere are untouched. Audio Pack settings for the sample need to be set up again; downloaded models stay.",
  confirm: "Set aside and start a new sample",
} as const;

export const DELETE_SET_ASIDE_DIALOG = {
  title: "Delete the set-aside sample?",
  body: "This permanently deletes the set-aside copy, including any FHIR files you imported into it. It cannot be undone. Your current sample and your original source files are not affected.",
  confirm: "Delete permanently",
} as const;

export const SET_ASIDE_EXISTS_NOTICE =
  "A set-aside sample workspace is stored on this device. It is not encrypted and cannot be opened in MedScale.";
