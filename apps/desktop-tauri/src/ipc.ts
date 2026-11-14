export type ShellStatus = {
  schemaVersion: 2;
  coreConnection: "connected" | "idle";
  detail: string;
  syntheticOnly: boolean;
};

// Reject a mismatched native protocol instead of rendering an assumed safe state.
export function parseShellStatus(value: unknown): ShellStatus {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("Invalid shell status response");
  }
  const status = value as Record<string, unknown>;
  if (status.schemaVersion !== 2 || (status.coreConnection !== "connected" && status.coreConnection !== "idle") ||
      typeof status.syntheticOnly !== "boolean" || typeof status.detail !== "string" ||
      status.detail.length === 0 || status.detail.length > 256) {
    throw new Error("Unsupported shell status response");
  }
  return { schemaVersion: 2, coreConnection: status.coreConnection, detail: status.detail, syntheticOnly: status.syntheticOnly };
}
