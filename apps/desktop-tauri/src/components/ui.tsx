import { createContext, useCallback, useContext, useEffect, useRef, useState, type CSSProperties, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent, type ReactNode } from "react";
import { useApp } from "../lib/app";
import type { CmdError } from "../lib/ipc";

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
  help: "M3 3h14v10H9.5L5.5 16.5V13H3Z M4.6 4.6v6.8h2.5v1.8l2-1.8h6.3V4.6Z M6.5 7h7v1.5h-7z M6.5 9.5h4.5V11H6.5z",
  about: "M10 2a8 8 0 1 1 0 16 8 8 0 0 1 0-16Z M9.1 8.6V14.6H10.9V8.6Z M9.1 5.2V7H10.9V5.2Z",
  search: "M8.5 2.5a6 6 0 1 1 0 12 6 6 0 0 1 0-12Zm0 1.7a4.3 4.3 0 1 0 0 8.6 4.3 4.3 0 0 0 0-8.6Z M12.7 13.9 13.9 12.7 18 16.8 16.8 18Z",
  commands: "M3 5.1 4.3 3.7 11.1 10 4.3 16.3 3 14.9 8.2 10Z M11 14.5h6.5v2H11z",
  runtime: "M3 3H17V17H3Z M4.6 4.6V15.4H15.4V4.6Z M7 7H13V13H7Z",
  workspace: "M2 2h7v7H2z M11 2h7v7h-7z M2 11h7v7H2z M11 11h7v7h-7z",
  more: "M4 8.5h3v3H4z M8.5 8.5h3v3h-3z M13 8.5h3v3h-3z",
  panelLeft: "M2.5 3.5H17.5V16.5H2.5ZM4 5V15H7V5ZM8.5 5V15H16V5Z",
  panelRight: "M2.5 3.5H17.5V16.5H2.5ZM4 5V15H11.5V5ZM13 5V15H16V5Z",
  copy: "M6.5 3H16.5V13.5H15V4.5H6.5ZM3.5 6H13.5V17H3.5ZM5 7.5V15.5H12V7.5Z",
  check: "M4 10.6 8 14.6 16.4 6.2 15.2 5 8 12.2 5.2 9.4Z",
  close: "M5.3 4.2 10 8.9 14.7 4.2 15.8 5.3 11.1 10 15.8 14.7 14.7 15.8 10 11.1 5.3 15.8 4.2 14.7 8.9 10 4.2 5.3Z",
  arrowLeft: "M9.6 4.2 3.8 10 9.6 15.8 10.8 14.6 7 10.8H16.5V9.2H7L10.8 5.4Z",
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
  // Status text may be truncated by table columns; keep the full value reachable on hover.
  return <span className={`st ${weight === "default" ? "" : weight}`} title={typeof children === "string" ? children : undefined}><Glyph kind={kind} /><span className="st-t">{children}</span></span>;
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
  if (/(denied| deny |blocked|quarantined|fail closed|default deny|not live)/.test(v)) return "blocked";
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
        <h2 className="sec-t">{title}</h2>
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

/** "content_digest" reads as "Content digest", and acronyms like ID/URI/SQL stay uppercase.
 * Labels that aren't plain keys, such as "real PHI" or "Ctrl K", pass through unchanged. */
export function humanKey(key: string): string {
  if (!/^[a-z][a-z0-9_ ]*$/i.test(key) || /^[A-Z]/.test(key)) return key;
  const words = key
    .replace(/_/g, " ")
    .replace(/\b(id|uri|url|fhir|sql)\b/gi, (m) => m.toUpperCase());
  return words[0]!.toUpperCase() + words.slice(1);
}

export function Ledger({ items }: { items: Array<{ k: string; v: ReactNode; mono?: boolean }> }) {
  return (
    <div className="ledger" style={{ padding: "12px 0", borderTop: "1px solid var(--line-subtle)", borderBottom: "1px solid var(--line-subtle)", marginBottom: 16, rowGap: 10 }}>
      {items.map((item) => (
        <div key={item.k}><span className="lk" title={item.k}>{humanKey(item.k)}</span><span className={`lv ${item.mono ? "mono" : ""}`}>{item.v}</span></div>
      ))}
    </div>
  );
}

export function KV({ rows, keyWidth = 140 }: { rows: Array<[string, ReactNode]>; keyWidth?: number }) {
  return (
    <div className="kv" style={{ gridTemplateColumns: `minmax(0, ${keyWidth}px) minmax(0, 1fr)` }}>
      {rows.map(([k, v]) => [<div key={`${k}-k`} title={k}>{humanKey(k)}</div>, <div key={`${k}-v`} className="t-sm">{v}</div>])}
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
  const tone =
    kind === "failed" || kind === "blocked" || kind === "denied"
      ? "notice-danger"
      : kind === "needs-review" || kind === "conflict" || kind === "synthetic"
        ? "notice-warn"
        : kind === "available" || kind === "present"
          ? "notice-info"
          : "";
  return <div className={`notice ${tone}`}><Glyph kind={kind} /><div><b>{title}</b> {children}</div></div>;
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

/* ───────────── Reusable Desktop Controls & Data Visualizations ───────────── */

export function SearchField({
  value,
  onChange,
  placeholder,
  ariaLabel,
  id,
  className = "",
}: {
  value: string;
  onChange: (next: string) => void;
  placeholder: string;
  ariaLabel: string;
  id?: string;
  className?: string;
}) {
  return (
    <div className={`field search-field ${className}`}>
      <Icon name="search" size={16} />
      <input
        id={id}
        type="text"
        value={value}
        placeholder={placeholder}
        aria-label={ariaLabel}
        data-page-search="true"
        onChange={(e) => onChange(e.target.value)}
      />
      {value && (
        <button type="button" className="search-clear" aria-label={`Clear ${ariaLabel.toLowerCase()}`} onClick={() => onChange("")}>
          <Icon name="close" size={13} />
        </button>
      )}
    </div>
  );
}

export function CopyButton({ text, label = "Copy", ariaLabel }: { text: string; label?: string; ariaLabel?: string }) {
  const [status, setStatus] = useState<"idle" | "copied" | "failed">("idle");
  const timer = useRef<number | null>(null);
  useEffect(() => () => { if (timer.current) window.clearTimeout(timer.current); }, []);
  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      setStatus("copied");
    } catch {
      setStatus("failed");
    }
    if (timer.current) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setStatus("idle"), 1800);
  }
  return (
    <button type="button" className={`copy-btn ${status === "copied" ? "copied" : status === "failed" ? "failed" : ""}`} aria-label={ariaLabel ?? label} onClick={() => void copy()}>
      <Icon name={status === "copied" ? "check" : status === "failed" ? "close" : "copy"} size={12} />
      <span aria-live="polite">{status === "copied" ? "Copied" : status === "failed" ? "Copy failed" : label}</span>
    </button>
  );
}

export type SegmentItem = {
  key: string;
  label: string;
  count: number;
  tone: "present" | "attention" | "danger" | "neutral" | "muted" | "hatched";
};

/** Truthful proportional distribution bar strictly backed by real Core counts. */
export function SegmentedBar({ title, meta, items, showLegend = true }: { title: string; meta?: string; items: SegmentItem[]; showLegend?: boolean }) {
  const visible = items.filter((i) => i.count > 0);
  const total = visible.reduce((sum, i) => sum + i.count, 0);
  if (total === 0) return null;
  return (
    <div className="viz-frame" role="group" aria-label={title}>
      <div className="viz-head">
        <span className="viz-title">{title}</span>
        {meta && <span className="viz-meta">{meta}</span>}
      </div>
      <div className="segbar-track" aria-hidden="true">
        {visible.map((seg) => (
          <span
            key={seg.key}
            className={`segbar-seg tone-${seg.tone}`}
            style={{ flexGrow: seg.count }}
            title={`${seg.label}: ${seg.count} of ${total}`}
          />
        ))}
      </div>
      {showLegend && (
        <div className="segbar-legend">
          {visible.map((seg) => (
            <span key={seg.key} className="segbar-legend-item">
              <span className={`segbar-dot segbar-seg tone-${seg.tone}`} aria-hidden="true" />
              <span>{seg.label}</span>
              <span className="mono t-mono-sm i3 tnum">{seg.count}</span>
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

export function ProgressBar({ label, detail, ratio, tone = "default" }: { label: string; detail?: string; ratio: number; tone?: "default" | "warn" | "danger" }) {
  const pct = Math.max(0, Math.min(100, Math.round(ratio * 100)));
  return (
    <div className="prog-wrap" role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct} aria-valuetext={detail ?? `${pct}%`}>
      <div className="prog-head">
        <span className="t-sm i1" style={{ fontWeight: 500 }}>{label}</span>
        <span className="mono t-mono-sm i3 tnum">{detail ?? `${pct}%`}</span>
      </div>
      <div className="prog-track">
        <div className={`prog-fill ${tone === "default" ? "" : tone}`} style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}

export function LifecycleStepper({ steps }: { steps: Array<{ label: string; state: "done" | "active" | "pending" | "failed" }> }) {
  return (
    <ol className="stepper" aria-label="Lifecycle progress" style={{ listStyle: "none", margin: "0 0 16px" }}>
      {steps.map((s, i) => (
        <li key={s.label} style={{ display: "inline-flex", alignItems: "center" }}>
          {i > 0 && <span className={`step-sep ${s.state === "done" || s.state === "active" ? "done" : ""}`} aria-hidden="true" />}
          <span className={`step-item ${s.state}`}>
            <Glyph kind={s.state === "done" ? "completed" : s.state === "active" ? "running" : s.state === "failed" ? "failed" : "pending"} />
            <span>{s.label}</span>
          </span>
        </li>
      ))}
    </ol>
  );
}

export function Disclosure({ summary, children, defaultOpen = false }: { summary: string; children: ReactNode; defaultOpen?: boolean }) {
  return (
    <details className="tech-details" open={defaultOpen || undefined}>
      <summary>
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M3 1v8l5-4Z" fill="currentColor" /></svg>
        <span>{summary}</span>
      </summary>
      <div style={{ marginTop: 10 }}>{children}</div>
    </details>
  );
}

/** Resizable two-pane layout with pointer-captured drag divider and responsive drawer
 *  so inspectors remain reachable at 1100x720 and narrower desktop windows. */
export function ResizableSplit({
  pane,
  inspector,
  inspectorLabel,
  defaultWidth = 360,
  minWidth = 280,
  maxWidth = 520,
}: {
  pane: ReactNode;
  inspector: ReactNode;
  inspectorLabel: string;
  defaultWidth?: number;
  minWidth?: number;
  maxWidth?: number;
}) {
  const { inspectorOpen, setInspectorOpen, setHasInspector } = useApp();
  const [width, setWidth] = useState(defaultWidth);
  const [dragging, setDragging] = useState(false);
  const startRef = useRef<{ x: number; w: number } | null>(null);
  const drawerCloseRef = useRef<HTMLButtonElement>(null);
  const returnFocusRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    setHasInspector(true);
    return () => {
      setHasInspector(false);
      setInspectorOpen(false);
    };
  }, [setHasInspector, setInspectorOpen]);

  useEffect(() => {
    if (window.innerWidth > 1179) return;
    if (inspectorOpen) {
      returnFocusRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      drawerCloseRef.current?.focus();
    } else if (returnFocusRef.current?.isConnected) {
      returnFocusRef.current.focus();
      returnFocusRef.current = null;
    }
  }, [inspectorOpen]);

  function onPointerDown(e: ReactPointerEvent<HTMLDivElement>) {
    e.currentTarget.setPointerCapture(e.pointerId);
    startRef.current = { x: e.clientX, w: width };
    setDragging(true);
  }
  function onPointerMove(e: ReactPointerEvent<HTMLDivElement>) {
    if (!startRef.current) return;
    const dx = startRef.current.x - e.clientX;
    setWidth(Math.max(minWidth, Math.min(maxWidth, Math.round(startRef.current.w + dx))));
  }
  function onPointerUp(e: ReactPointerEvent<HTMLDivElement>) {
    if (startRef.current) {
      try { e.currentTarget.releasePointerCapture(e.pointerId); } catch { /* released */ }
      startRef.current = null;
      setDragging(false);
    }
  }
  function onResizerKey(e: ReactKeyboardEvent<HTMLDivElement>) {
    if (e.key === "ArrowLeft") { e.preventDefault(); setWidth((w) => Math.min(maxWidth, w + 24)); }
    else if (e.key === "ArrowRight") { e.preventDefault(); setWidth((w) => Math.max(minWidth, w - 24)); }
    else if (e.key === "Home") { e.preventDefault(); setWidth(defaultWidth); }
  }

  return (
    <>
      {pane}
      <div
        role="separator"
        aria-orientation="vertical"
        aria-label={`Resize ${inspectorLabel.toLowerCase()} panel`}
        aria-valuemin={minWidth}
        aria-valuemax={maxWidth}
        aria-valuenow={width}
        tabIndex={0}
        className={`split-resizer ${dragging ? "dragging" : ""}`}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onDoubleClick={() => setWidth(defaultWidth)}
        onKeyDown={onResizerKey}
      />
      <div className={`inspector-scrim ${inspectorOpen ? "drawer-open" : ""}`} onClick={() => setInspectorOpen(false)} aria-hidden="true" />
      <aside className={`inspector drawer-managed ${inspectorOpen ? "drawer-open" : ""}`} aria-label={inspectorLabel} style={{ "--ins-w": `${width}px` } as CSSProperties}>
        <div className="inspector-drawer-bar">
          <span className="t-h3">{inspectorLabel}</span>
          <button ref={drawerCloseRef} type="button" className="ibtn" aria-label={`Close ${inspectorLabel.toLowerCase()}`} onClick={() => setInspectorOpen(false)}>
            <Icon name="close" size={14} />
          </button>
        </div>
        {inspector}
      </aside>
    </>
  );
}

const ERROR_GLYPH: Record<CmdError["kind"], GlyphKind> = {
  unavailable: "unavailable", disconnected: "pending", denied: "denied", blocked: "blocked", missing: "absent",
  conflict: "conflict", stale: "partial", invalid: "absent", corrupt: "failed", unsupported: "unsupported",
  cancelled: "cancelled", internal: "failed",
};

const ERROR_TITLE: Record<CmdError["kind"], string> = {
  unavailable: "Temporarily unavailable",
  disconnected: "Runtime disconnected",
  denied: "Action not permitted",
  blocked: "Blocked by safety boundary",
  missing: "Item not found",
  conflict: "Version conflict",
  stale: "Data changed on disk",
  invalid: "Check your input",
  corrupt: "Integrity check failed",
  unsupported: "Not supported in this build",
  cancelled: "Cancelled",
  internal: "Unexpected error",
};

/** Explicit error state. The Core message is shown verbatim. */
export function LoadError({ cmd }: { cmd: { state: { status: string; error?: CmdError }; reload: () => void } }) {
  return cmd.state.status === "error" && cmd.state.error ? <ErrorState error={cmd.state.error} onRetry={cmd.reload} /> : null;
}

export function ErrorState({ error, onRetry }: { error: CmdError; onRetry?: () => void }) {
  const label = error.kind === "stale" || error.kind === "conflict" ? "Reload" : "Retry";
  const showAction = !!onRetry && error.kind !== "denied" && error.kind !== "blocked" && error.kind !== "unsupported";
  return (
    <div className="err" role="alert">
      <Glyph kind={ERROR_GLYPH[error.kind]} size={12} />
      <div className="stack" style={{ gap: 8 }}>
        <div className="t-h3">{ERROR_TITLE[error.kind] ?? error.kind}</div>
        <p className="t-sm i2">{error.message}</p>
        {showAction && <div><button type="button" className="btn btn-s" onClick={onRetry}>{label}</button></div>}
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
  const timers = useRef<Map<number, number>>(new Map());

  const dismiss = useCallback((id: number) => {
    const t = timers.current.get(id);
    if (t) { window.clearTimeout(t); timers.current.delete(id); }
    setToasts((all) => all.filter((x) => x.id !== id));
  }, []);

  const schedule = useCallback((id: number, ms: number) => {
    const prev = timers.current.get(id);
    if (prev) window.clearTimeout(prev);
    timers.current.set(id, window.setTimeout(() => dismiss(id), ms));
  }, [dismiss]);

  const push = useCallback((t: Omit<Toast, "id">) => {
    const id = Date.now() + Math.random();
    // Retain up to three toasts so error notices are not immediately overwritten.
    setToasts((all) => [...all.slice(-2), { ...t, id }]);
    const duration = t.kind === "failed" || t.kind === "blocked" || t.kind === "denied" ? 9000 : 5500;
    schedule(id, duration);
  }, [schedule]);

  useEffect(() => () => { timers.current.forEach((h) => window.clearTimeout(h)); }, []);

  return (
    <ToastCtx.Provider value={push}>
      {children}
      <div className="toasts" role="status" aria-live="polite">
        {toasts.map((t) => (
          <div
            key={t.id}
            className="overlay toast anim"
            onMouseEnter={() => { const h = timers.current.get(t.id); if (h) window.clearTimeout(h); }}
            onMouseLeave={() => schedule(t.id, 3000)}
          >
            <Glyph kind={t.kind} />
            <div className="toast-body"><b>{t.title}</b>{t.detail && <> {t.detail}</>}</div>
            <button type="button" className="toast-x" aria-label="Dismiss notification" onClick={() => dismiss(t.id)}>
              <Icon name="close" size={12} />
            </button>
          </div>
        ))}
      </div>
    </ToastCtx.Provider>
  );
}

/** Core messages often start with their kind ("Invalid: …"); the toast title already says it. */
export function withoutKindPrefix(message: string, title: string): string {
  const prefix = `${title}:`;
  return message.toLowerCase().startsWith(prefix.toLowerCase()) ? message.slice(prefix.length).trimStart() : message;
}

/** Reports an action outcome as a toast and returns whether it succeeded. */
export function useOutcome() {
  const toast = useToast();
  return useCallback(<T,>(result: { ok: true; data: T } | { ok: false; error: CmdError }, success: string): result is { ok: true; data: T } => {
    if (result.ok) toast({ kind: "completed", title: success });
    else {
      const title = result.error.kind[0]!.toUpperCase() + result.error.kind.slice(1);
      toast({ kind: ERROR_GLYPH[result.error.kind], title, detail: withoutKindPrefix(result.error.message, title) });
    }
    return result.ok;
  }, [toast]);
}
