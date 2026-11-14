/** In-app defect reporting (Spec 107 D2). The app never opens the browser:
 * there is no opener/shell plugin and navigation_policy.rs admits only the
 * app origin. The user copies the fixed URL and a fixed-key diagnostics
 * block built only from about_info fields, which carry no paths or IDs. */
export const SUPPORT_ISSUES_URL = "https://github.com/AbdulazizShehri/MedScale/issues/new/choose";

/** Mirrors the bolded rule in SUPPORT.md; a test keeps the two in sync. */
export const SUPPORT_DATA_RULE =
  "Public bug reports and discussions must use synthetic, non-identifying examples only. Never upload real patient records, recordings, transcripts, identifying health information, confidential material or private vault logs.";

export type SupportAbout = {
  version: string;
  build_profile: string;
  target: string;
  shell: string;
  local_only: boolean;
  network_default_deny: boolean;
};

export const SUPPORT_DIAGNOSTIC_KEYS = ["MedScale version", "Build profile", "Target", "Shell", "Processing", "Network"] as const;

export function supportDiagnostics(about: SupportAbout): string {
  const values: Record<(typeof SUPPORT_DIAGNOSTIC_KEYS)[number], string> = {
    "MedScale version": about.version,
    "Build profile": about.build_profile,
    Target: about.target,
    Shell: about.shell,
    Processing: about.local_only ? "local only" : "not local-only",
    // Broker egress is default-deny; user-consented model downloads can still occur.
    Network: about.network_default_deny ? "default deny (consented model downloads excepted)" : "not default deny",
  };
  return SUPPORT_DIAGNOSTIC_KEYS.map((k) => `${k}: ${values[k].replace(/[\r\n]+/g, " ").slice(0, 120)}`).join("\n") + "\n";
}
