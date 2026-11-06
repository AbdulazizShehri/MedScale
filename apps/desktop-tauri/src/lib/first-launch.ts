/** Which screen the app shows before a workspace is open. This only chooses a
 * screen: Core alone decides whether a workspace opens, so nothing here can get
 * past a passphrase. A closed encrypted vault always leads to unlock, unless the
 * person explicitly chose to look at the sample workspace instead. */
export type GateRequest = "auto" | "welcome" | "access";
export type GateView = "loading" | "runtime-unavailable" | "workspace" | "welcome" | "unlock" | "create";

export function gateView(input: {
  runtimeError: boolean;
  status: { open: boolean; encrypted_vault_exists: boolean } | null;
  /** Settings and About stay reachable with no workspace open. */
  routeNeedsWorkspace: boolean;
  request: GateRequest;
  /** Shown-once recovery codes are on screen: nothing may replace them until confirmed. */
  recoveryCodesShown?: boolean;
}): GateView {
  const { runtimeError, status, routeNeedsWorkspace, request, recoveryCodesShown } = input;
  if (recoveryCodesShown) return "create";
  if (!status) return runtimeError ? "runtime-unavailable" : "loading";
  if (status.open || !routeNeedsWorkspace) return "workspace";
  if (request === "welcome") return "welcome";
  if (status.encrypted_vault_exists) return "unlock";
  return request === "access" ? "create" : "welcome";
}

/** Core reports whether the sample is on disk, because the production WebView runs
 * InPrivate and keeps no browser storage across restarts. This only words the
 * Welcome screen and never opens anything by itself. */
export function isReturningToSample(status: { sample_workspace_exists?: boolean } | null | undefined): boolean {
  return status?.sample_workspace_exists === true;
}
