import { useEffect, useId, useRef, type KeyboardEvent, type ReactNode } from "react";

/** Spec 108 step 5: an in-place confirmation with a focus trap. Cancel is
 * focused first (the safe default); Tab cycles inside; Escape cancels; focus
 * returns to the element that opened it. */
export function ConfirmPanel({ title, children, confirmLabel, onConfirm, onCancel, busy = false }: {
  title: string;
  children: ReactNode;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
  busy?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const titleId = useId();
  useEffect(() => {
    const returnTo = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    ref.current?.querySelector<HTMLButtonElement>("button")?.focus();
    return () => returnTo?.focus();
  }, []);
  function onKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    if (e.key === "Escape") { e.preventDefault(); onCancel(); return; }
    if (e.key !== "Tab") return;
    const buttons = Array.from(ref.current?.querySelectorAll<HTMLButtonElement>("button:not([disabled])") ?? []);
    const first = buttons.at(0), last = buttons.at(-1);
    if (!first || !last) return;
    if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
    else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
  }
  return (
    <div ref={ref} role="dialog" aria-modal="true" aria-labelledby={titleId} className="notice" onKeyDown={onKeyDown}
      style={{ display: "block", whiteSpace: "normal", marginTop: 12 }}>
      <b id={titleId}>{title}</b>
      <div className="t-sm" style={{ margin: "6px 0 10px" }}>{children}</div>
      <div className="row-gap" style={{ gap: 8 }}>
        <button type="button" className="btn btn-g" onClick={onCancel}>Cancel</button>
        <button type="button" className="btn btn-p" onClick={onConfirm} disabled={busy}>{confirmLabel}</button>
      </div>
    </div>
  );
}
