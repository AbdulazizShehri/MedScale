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
}): GateView {
  const { runtimeError, status, routeNeedsWorkspace, request } = input;
  if (!status) return runtimeError ? "runtime-unavailable" : "loading";
  if (status.open || !routeNeedsWorkspace) return "workspace";
  if (request === "welcome") return "welcome";
  if (status.encrypted_vault_exists) return "unlock";
  return request === "access" ? "create" : "welcome";
}

/** Set after the sample opens, to word the return path. It holds no identifier,
 * says nothing about who is using the app, and never opens anything by itself. */
export const SAMPLE_OPENED_PREF = "sample-opened";

export function isReturningToSample(pref: string | null): boolean {
  return pref === "1";
}
