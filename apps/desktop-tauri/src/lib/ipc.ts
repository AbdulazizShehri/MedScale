import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

/** Error classes rendered by the interface (board 25). Mirrors host.rs. */
export type ErrorKind =
  | "unavailable" | "denied" | "blocked" | "missing" | "conflict" | "stale"
  | "invalid" | "corrupt" | "unsupported" | "cancelled" | "internal" | "disconnected";

export type CmdError = { kind: ErrorKind; message: string };

const KINDS: ReadonlySet<string> = new Set([
  "unavailable", "denied", "blocked", "missing", "conflict", "stale",
  "invalid", "corrupt", "unsupported", "cancelled", "internal", "disconnected",
]);

/** Normalizes anything thrown by `invoke` into a typed CmdError. A command
 *  that is not registered or not granted surfaces as "denied", never as data. */
export function toCmdError(raw: unknown): CmdError {
  if (raw && typeof raw === "object" && "kind" in raw && "message" in raw) {
    const { kind, message } = raw as { kind: unknown; message: unknown };
    if (typeof kind === "string" && KINDS.has(kind) && typeof message === "string") {
      return { kind: kind as ErrorKind, message: message.slice(0, 400) };
    }
  }
  const text = typeof raw === "string" ? raw : raw instanceof Error ? raw.message : "";
  if (/not allowed|not found|denied|forbidden/i.test(text)) {
    return { kind: "denied", message: "Denied: this window may not call that command" };
  }
  if (/__TAURI|invoke|window\./.test(text) || !text) {
    return { kind: "disconnected", message: "Local runtime disconnected" };
  }
  return { kind: "internal", message: "Internal: unexpected desktop failure" };
}

export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (raw) {
    throw toCmdError(raw);
  }
}

export type Loadable<T> =
  | { status: "loading" }
  | { status: "ready"; data: T }
  | { status: "error"; error: CmdError };

/** Runs a read command and keeps the last good value while reloading, so the
 *  layout never jumps. Errors stay errors; they never become empty success. */
export function useCommand<T>(command: string | null, args?: Record<string, unknown>) {
  const [state, setState] = useState<Loadable<T>>({ status: "loading" });
  const key = command ? `${command}:${JSON.stringify(args ?? {})}` : null;
  const seq = useRef(0);

  const load = useCallback(async () => {
    if (!command) return;
    const mine = ++seq.current;
    try {
      const data = await call<T>(command, args);
      if (mine === seq.current) setState({ status: "ready", data });
    } catch (error) {
      if (mine === seq.current) setState({ status: "error", error: error as CmdError });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  useEffect(() => {
    if (!command) return;
    setState((prev) => (prev.status === "ready" ? prev : { status: "loading" }));
    void load();
  }, [load, command]);

  return { state, reload: load };
}

/** Wraps a mutating command with pending state and a typed outcome. */
export function useAction() {
  const [pending, setPending] = useState<string | null>(null);
  const run = useCallback(async <T,>(label: string, command: string, args?: Record<string, unknown>): Promise<{ ok: true; data: T } | { ok: false; error: CmdError }> => {
    setPending(label);
    try {
      const data = await call<T>(command, args);
      return { ok: true, data };
    } catch (error) {
      return { ok: false, error: error as CmdError };
    } finally {
      setPending(null);
    }
  }, []);
  return { pending, run };
}
