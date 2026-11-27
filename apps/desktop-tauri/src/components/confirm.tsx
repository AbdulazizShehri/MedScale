import { useEffect, useId, useRef, type KeyboardEvent, type ReactNode } from "react";

/** Spec 108 step 5: an in-place confirmation with a focus trap. Cancel is
 * focused first (the safe default); Tab cycles inside; Escape cancels; focus
 * returns to the element that opened it. */
export function ConfirmPanel({ title, children, confirmLabel, onConfirm, onCancel, busy = false, destructive = false }: {
  title: string;
  children: ReactNode;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
  busy?: boolean;
  destructive?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const bodyId = useId();
  useEffect(() => {
    const returnTo = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    ref.current?.querySelector<HTMLButtonElement>("button")?.focus();
    // The trigger may be gone once the action succeeds; fall back to the page.
    return () => {
      if (returnTo?.isConnected) returnTo.focus();
      else document.getElementById("main")?.focus();
    };
  }, []);
  function onKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    if (e.key === "Escape") { e.preventDefault(); if (!busy) onCancel(); return; }
    if (e.key !== "Tab") return;
    const buttons = Array.from(ref.current?.querySelectorAll<HTMLButtonElement>("button:not([disabled])") ?? []);
    const first = buttons.at(0), last = buttons.at(-1);
    if (!first || !last) return;
    if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
    else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
  }
  return (
    <div
      ref={ref}
      role="alertdialog"
      aria-labelledby={titleId}
      aria-describedby={bodyId}
      className={`notice confirm-panel ${destructive ? "notice-danger" : "notice-warn"}`}
      onKeyDown={onKeyDown}
    >
      <b id={titleId}>{title}</b>
      <div id={bodyId} className="t-sm confirm-body">{children}</div>
      <div className="row-gap" style={{ gap: 8 }}>
        <button type="button" className="btn btn-g" onClick={onCancel} disabled={busy}>Cancel</button>
        <button type="button" className={destructive ? "btn btn-d-fill btn-inline" : "btn btn-p btn-inline"} onClick={onConfirm} disabled={busy}>{confirmLabel}</button>
      </div>
    </div>
  );
}
