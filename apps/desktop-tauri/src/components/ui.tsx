import { createContext, useCallback, useContext, useState, type ReactNode } from "react";
import type { CmdError } from "../lib/ipc";

/* ───────────── Logo (measured two-triangle M, even-odd knockout) ───────────── */
export function Mark({ width = 28, className }: { width?: number; className?: string }) {
  return (
    <svg width={width} height={Math.round((width * 76) / 160)} viewBox="0 0 160 76" className={className} aria-hidden="true">
      <path d="M0 76 50 0 100 76Z M60 76 110 0 160 76Z" fill="currentColor" fillRule="evenodd" />
    </svg>
  );
}

/* ───────────── Icons: solid geometric, 20-unit grid ───────────── */
const ICONS = {
  home: "M10 2 18 9V18H12V13H8V18H2V9Z",
  patients: "M10 2.2a3.8 3.8 0 1 1 0 7.6 3.8 3.8 0 0 1 0-7.6Z M2.5 18a7.5 7.5 0 0 1 15 0Z",
  documents: "M7 2H17V14H15.5V3.5H7Z M3 5.5H13V18H3Z",
  insights: "M10 2a8 8 0 1 1 0 16 8 8 0 0 1 0-16Zm0 1.6a6.4 6.4 0 1 0 0 12.8 6.4 6.4 0 0 0 0-12.8Z M10 10V5.2A4.8 4.8 0 0 1 14.8 10Z",
  evidence: "M2 3h4v4H2z M8 4.25h10v1.5H8z M2 8h4v4H2z M8 9.25h10v1.5H8z M2 13h4v4H2z M8 14.25h7v1.5H8z",
  projects: "M2 4H8L10 6H18V17H2Z",
  data: "M2.5 12H6V18H2.5Z M8.25 7.5H11.75V18H8.25Z M14 2.5H17.5V18H14Z",
  browse: "M2 3H18V17H2Z M3.5 7.5V15.5H16.5V7.5Z",
  analytics: "M10 2 14.5 8.5H5.5Z M10 9.5 17 18H3Z",
  knowledge: "M4 2H12.5L16 5.5V18H4Z M7 9V10.5H13V9Z M7 12V13.5H13V12Z M7 15V16.5H11V15Z",
  research: "M7.5 3 14 18H1Z M14.6 8.2 19 18H12.4L14.6 13Z",
  models: "M10 2 18 10 10 18 2 10Z M10 6.8 6.8 10 10 13.2 13.2 10Z",
  medagent: "M10 2.5 18 17H2Z M10 6.9 4.9 15.5H15.1Z M10 10.6a1.6 1.6 0 1 1 0 3.2 1.6 1.6 0 0 1 0-3.2Z",
  fleet: "M2 6.5 6 2.5 10 6.5 6 10.5Z M10 13.5 14 9.5 18 13.5 14 17.5Z M10.6 5.8h1.6v4.6h4.6V12H10.6Z",
  workflows: "M2 2h6v6H2z M12 12h6v6h-6z M4.25 8h1.5v5.25H12v1.5H4.25Z",
  tasks: "M3 3h14v14H3z M5.6 10.4 8.6 13.4 14.4 7.6 13.3 6.5 8.6 11.2 6.7 9.3Z",
  messages: "M2 3H18V14H8.5L4.5 17.5V14H2Z",
  collaboration: "M6 5.5a4.5 4.5 0 1 1 0 9 4.5 4.5 0 0 1 0-9Z M14 5.5a4.5 4.5 0 1 1 0 9 4.5 4.5 0 0 1 0-9Zm0 1.6a2.9 2.9 0 1 0 0 5.8 2.9 2.9 0 0 0 0-5.8Z",
  audio: "M3 7.5h1.6v5H3Z M6.6 5h1.6v10H6.6Z M10.2 2.5h1.6v15h-1.6Z M13.8 6h1.6v8h-1.6Z M17.4 8.5H19v3h-1.6Z",
  privacy: "M10 2a8 8 0 1 1 0 16 8 8 0 0 1 0-16Zm0 1.6a6.4 6.4 0 1 0 0 12.8Z",
  audit: "M3 2h14v16H3Z M4.6 3.6v12.8h10.8V3.6Z M6.5 6h7v1.4h-7Z M6.5 9.3h7v1.4h-7Z M6.5 12.6h4.5V14H6.5Z",
  exports: "M2 12h1.6v4.4h12.8V12H18v6H2Z M9.2 2h1.6v8.4l2.6-2.6 1.1 1.1L10 13.4 5.5 8.9l1.1-1.1 2.6 2.6Z",
  integrations: "M2 2h7v7H2z M11 2h7v7h-7z M2 11h7v7H2z M11 11h7v7h-7z",
  settings: "M2 5.25h16v1.5H2z M2 13.25h16v1.5H2z M11 3h4v5h-4z M5 11h4v5H5z",
  about: "M10 2a8 8 0 1 1 0 16 8 8 0 0 1 0-16Z M9.1 8.6V14.6H10.9V8.6Z M9.1 5.2V7H10.9V5.2Z",
  search: "M8.5 2.5a6 6 0 1 1 0 12 6 6 0 0 1 0-12Zm0 1.7a4.3 4.3 0 1 0 0 8.6 4.3 4.3 0 0 0 0-8.6Z M12.7 13.9 13.9 12.7 18 16.8 16.8 18Z",
  commands: "M3 5.1 4.3 3.7 11.1 10 4.3 16.3 3 14.9 8.2 10Z M11 14.5h6.5v2H11z",
  runtime: "M3 3H17V17H3Z M4.6 4.6V15.4H15.4V4.6Z M7 7H13V13H7Z",
  workspace: "M2 2h7v7H2z M11 2h7v7h-7z M2 11h7v7H2z M11 11h7v7h-7z",
  more: "M4 8.5h3v3H4z M8.5 8.5h3v3h-3z M13 8.5h3v3h-3z",
} as const;
export type IconName = keyof typeof ICONS;

export function Icon({ name, size = 18, className }: { name: IconName; size?: number; className?: string }) {
  return (
    <svg width={size} height={size} viewBox="0 0 20 20" className={className} aria-hidden="true" focusable="false">
      <path d={ICONS[name]} fill="currentColor" fillRule="evenodd" />
    </svg>
  );
}

/* ───────────── Status grammar: shape = category, fill = certainty ───────────── */
export type GlyphKind =
  | "present" | "partial" | "absent" | "unknown" | "conflict" | "incomparable" | "unhealthy" | "unsupported" | "synthetic"
  | "available" | "running" | "pending" | "completed" | "unavailable" | "cancelled" | "failed" | "blocked"
  | "needs-review" | "reviewed" | "proposal" | "limited" | "denied";

export function Glyph({ kind, size }: { kind: GlyphKind; size?: number }) {
  const style = size ? { width: size, height: size } : undefined;
  if (kind === "needs-review" || kind === "reviewed" || kind === "proposal" || kind === "limited" || kind === "denied") {
    const fill = kind === "needs-review" ? "var(--sem-review)" : kind === "reviewed" ? "currentColor" : "none";
    const stroke = kind === "needs-review" ? "var(--sem-review)" : kind === "denied" ? "var(--sem-danger)" : kind === "limited" ? "var(--ink-3)" : "currentColor";
    return (
      <svg className="gt" viewBox="0 0 10 10" aria-hidden="true" style={style}>
        <path d="M5 1 9.3 9H.7Z" fill={fill} stroke={stroke} strokeWidth={1.1} strokeDasharray={kind === "limited" ? "1.6 1.2" : undefined} strokeLinejoin="round" />
      </svg>
    );
  }
  const cls = kind === "completed" ? "g-completed" : `g-${kind}`;
  return <span className={`g ${cls}`} aria-hidden="true" style={style} />;
}

export function Status({ kind, children, weight = "default" }: { kind: GlyphKind; children: ReactNode; weight?: "strong" | "default" | "weak" }) {
  return <span className={`st ${weight === "default" ? "" : weight}`}><Glyph kind={kind} /><span className="st-t">{children}</span></span>;
}

/** Maps literal Core strings onto the grammar. Unknown strings stay "unknown". */
export function glyphFor(raw: string | null | undefined): GlyphKind {
  const v = (raw ?? "").toLowerCase();
  if (!v) return "unknown";
  if (/(unsupported|outside trusted)/.test(v)) return "unsupported";
  if (/(conflict)/.test(v)) return "conflict";
  if (/(incomparable)/.test(v)) return "incomparable";
  if (/(unhealthy)/.test(v)) return "unhealthy";
  if (/^(absent)|explicitly absent/.test(v)) return "absent";
  if (/(unknown|not queried|not computed)/.test(v)) return "unknown";
  if (/(partial|partially)/.test(v)) return "partial";
  if (/(present|healthy|complete\b|admitted|bound|trusted presentation)/.test(v)) return "present";
  if (/(running|in_progress|in progress|dispatched)/.test(v)) return "running";
  if (/(pending|queued|open\b|draft|candidate)/.test(v)) return "pending";
  if (/(completed|done|confirmed|accepted|succeeded|resolved)/.test(v)) return "completed";
  if (/(failed|corrupt|timed ?out|error)/.test(v)) return "failed";
  if (/(denied|deny|blocked|quarantined|fail closed|default deny|not live)/.test(v)) return "blocked";
  if (/^(allow|allowed|permitted)$/.test(v)) return "available";
  if (/(cancel|retired|archived|retracted|revoked|tombstoned)/.test(v)) return "cancelled";
  if (/(review|required|attention)/.test(v)) return "needs-review";
  if (/(proposal|proposed)/.test(v)) return "proposal";
  if (/(limited|read-only|gated)/.test(v)) return "limited";
  if (/(active|available|local|live|enabled|ready)/.test(v)) return "available";
  if (/(unavailable|offline|idle)/.test(v)) return "unavailable";
  return "unknown";
}

/* ───────────── Layout primitives ───────────── */
export function Section({ n, title, count, actions, children }: { n?: string; title: string; count?: ReactNode; actions?: ReactNode; children: ReactNode }) {
  return (
    <section className="section">
      <div className="sec">
        {n && <span className="sec-n">{n}</span>}
        <span className="sec-t">{title}</span>
        {count !== undefined && <span className="sec-c">{count}</span>}
        <span className="sec-rule" />
        {actions && <span className="sec-a">{actions}</span>}
      </div>
      {children}
    </section>
  );
}

export function PageHead({ title, sub, children }: { title: string; sub?: ReactNode; children?: ReactNode }) {
  return (
    <div className="page-head">
      <div className="grow">
        <h1 className="t-h1">{title}</h1>
        {sub && <p className="ph-sub">{sub}</p>}
      </div>
      {children}
    </div>
  );
}

export function Ledger({ items }: { items: Array<{ k: string; v: ReactNode; mono?: boolean }> }) {
  return (
    <div className="ledger" style={{ padding: "12px 0", borderTop: "1px solid var(--line-subtle)", borderBottom: "1px solid var(--line-subtle)", marginBottom: 16, rowGap: 10 }}>
      {items.map((item) => (
        <div key={item.k}><span className="lk">{item.k}</span><span className={`lv ${item.mono ? "mono" : ""}`}>{item.v}</span></div>
      ))}
    </div>
  );
}

export function KV({ rows, keyWidth = 140 }: { rows: Array<[string, ReactNode]>; keyWidth?: number }) {
  return (
    <div className="kv" style={{ gridTemplateColumns: `${keyWidth}px minmax(0, 1fr)` }}>
      {rows.map(([k, v]) => [<div key={`${k}-k`}>{k}</div>, <div key={`${k}-v`} className="t-sm">{v}</div>])}
    </div>
  );
}

export function Stamp({ k, v }: { k?: string; v: ReactNode }) {
  return <span className="stamp trunc">{k && <i>{k}</i>}{v}</span>;
}

export function Authority({ children, label = "Authority" }: { children: ReactNode; label?: string }) {
  return (
    <div className="auth">
      <Glyph kind="proposal" />
      <span className="t-label">{label}</span>
      <span style={{ minWidth: 0 }}>{children}</span>
    </div>
  );
}

export function Notice({ kind, title, children }: { kind: GlyphKind; title: string; children?: ReactNode }) {
  return <div className="notice"><Glyph kind={kind} /><div><b>{title}</b> {children}</div></div>;
}

export function Empty({ icon, title, children, action }: { icon: IconName; title: string; children?: ReactNode; action?: ReactNode }) {
  return (
    <div className="empty">
      <Icon name={icon} size={20} />
      <div className="stack" style={{ gap: 12 }}>
        <div><div className="t-h2">{title}</div>{children && <p className="t-body i2" style={{ marginTop: 4 }}>{children}</p>}</div>
        {action && <div className="row-gap" style={{ gap: 8 }}>{action}</div>}
      </div>
    </div>
  );
}

const ERROR_GLYPH: Record<CmdError["kind"], GlyphKind> = {
  unavailable: "unavailable", disconnected: "pending", denied: "denied", blocked: "blocked", missing: "absent",
  conflict: "conflict", stale: "partial", invalid: "absent", corrupt: "failed", unsupported: "unsupported",
  cancelled: "cancelled", internal: "failed",
};

/** Explicit error state. The Core message is shown verbatim. */
export function ErrorState({ error, onRetry }: { error: CmdError; onRetry?: () => void }) {
  const retryable = error.kind === "unavailable" || error.kind === "disconnected" || error.kind === "stale" || error.kind === "conflict";
  return (
    <div className="err" role="alert">
      <Glyph kind={ERROR_GLYPH[error.kind]} size={12} />
      <div className="stack" style={{ gap: 8 }}>
        <div className="t-h3" style={{ textTransform: "capitalize" }}>{error.kind}</div>
        <p className="t-sm i2">{error.message}</p>
        {retryable && onRetry && <div><button type="button" className="btn btn-s" onClick={onRetry}>{error.kind === "stale" || error.kind === "conflict" ? "Reload" : "Retry"}</button></div>}
      </div>
    </div>
  );
}

export function Skeleton({ rows = 4 }: { rows?: number }) {
  return (
    <div aria-busy="true" aria-label="Loading" className="stack" style={{ gap: 14, paddingTop: 12 }}>
      {Array.from({ length: rows }, (_, i) => (
        <div key={i} style={{ display: "flex", gap: 12, alignItems: "center" }}>
          <span className="sk t" style={{ width: `${28 + ((i * 17) % 30)}%` }} />
          <span className="sk" style={{ width: `${18 + ((i * 11) % 22)}%` }} />
        </div>
      ))}
    </div>
  );
}

/* ───────────── Toasts ───────────── */
type Toast = { id: number; kind: GlyphKind; title: string; detail?: string };
const ToastCtx = createContext<(t: Omit<Toast, "id">) => void>(() => undefined);
export const useToast = () => useContext(ToastCtx);

export function ToastHost({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const push = useCallback((t: Omit<Toast, "id">) => {
    const id = Date.now() + Math.random();
    setToasts((all) => [...all.slice(-3), { ...t, id }]);
    window.setTimeout(() => setToasts((all) => all.filter((x) => x.id !== id)), 6000);
  }, []);
  return (
    <ToastCtx.Provider value={push}>
      {children}
      <div className="toasts" role="status" aria-live="polite">
        {toasts.map((t) => (
          <div key={t.id} className="overlay toast anim"><Glyph kind={t.kind} /><div><b>{t.title}</b>{t.detail && <> {t.detail}</>}</div></div>
        ))}
      </div>
    </ToastCtx.Provider>
  );
}

/** Reports an action outcome as a toast and returns whether it succeeded. */
export function useOutcome() {
  const toast = useToast();
  return useCallback(<T,>(result: { ok: true; data: T } | { ok: false; error: CmdError }, success: string): result is { ok: true; data: T } => {
    if (result.ok) toast({ kind: "completed", title: success });
    else toast({ kind: ERROR_GLYPH[result.error.kind], title: result.error.kind[0]!.toUpperCase() + result.error.kind.slice(1), detail: result.error.message });
    return result.ok;
  }, [toast]);
}
