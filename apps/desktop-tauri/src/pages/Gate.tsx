import { useEffect, useMemo, useState, type FormEvent, type ReactNode } from "react";
import { useApp } from "../lib/app";
import { isReturningToSample } from "../lib/first-launch";
import { useAction, type CmdError } from "../lib/ipc";
import { Logo } from "../components/brand";
import { Glyph, useToast } from "../components/ui";
import { DELETE_SET_ASIDE_DIALOG, MOVE_SESSION_ASIDE_DIALOG, RECOVERED_SEED_NOTICE, SET_ASIDE_DIALOG, SET_ASIDE_EXISTS_NOTICE, recoveredInterruptedSeed, vaultOpenNotice, seedFailureGuidance, setAsideOffered } from "../lib/seed-failure";
import { ConfirmPanel } from "../components/confirm";

/** Spec 108 step 2: one announced alert per failure. With a known reason the
 * plain guidance replaces Core's raw message, which can name internal paths.
 * Next steps are always explicit: nothing here retries, resets or deletes by itself. */
function FailureAlert({ error, glyph = "failed", children }: { error: CmdError; glyph?: "failed" | "denied"; children?: ReactNode }) {
  const g = seedFailureGuidance(error.reason);
  return (
    <div className="recovery" role="alert" data-reason={error.reason}>
      <Glyph kind={glyph} />
      <div className="recovery-body">
        {g ? <><p className="recovery-title">{g.title}</p><p className="recovery-detail">{g.detail}</p></> : <p className="recovery-detail">{error.message}</p>}
        {children && <div className="recovery-actions">{children}</div>}
      </div>
    </div>
  );
}

/** A height field rendered as dots. Decorative. */
function Topology() {
  const dots = useMemo(() => {
    const cols = 58, rows = 30;
    const z = (u: number, v: number) =>
      Math.exp(-(((u - 0.6) ** 2) / 0.022 + ((v - 0.42) ** 2) / 0.05)) +
      0.62 * Math.exp(-(((u - 0.28) ** 2) / 0.03 + ((v - 0.72) ** 2) / 0.035)) +
      0.38 * Math.exp(-(((u - 0.86) ** 2) / 0.018 + ((v - 0.78) ** 2) / 0.03)) +
      0.07 * Math.sin(u * 13 + v * 6);
    const P = (u: number, v: number) => {
      const d = 0.38 + 0.62 * v, h = z(u, v);
      return { x: 450 + (u - 0.5) * 1180 * d, y: 170 + v * 520 - h * 150 * d, d, h };
    };
    const dots: Array<{ x: number; y: number; r: number; o: number }> = [];
    for (let j = 0; j < rows; j++) for (let i = 0; i < cols; i++) {
      const u = i / (cols - 1), v = j / (rows - 1), p = P(u, v);
      dots.push({ x: p.x, y: p.y, r: 0.55 + 1.15 * p.d, o: Math.min(0.95, 0.12 + 0.55 * v + 0.35 * Math.min(1, p.h)) });
    }
    return dots;
  }, []);
  return (
    <svg className="gate-topo" viewBox="0 0 900 760" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
      {dots.map((d, i) => <circle key={i} cx={d.x} cy={d.y} r={d.r} fill="currentColor" fillOpacity={d.o} />)}
    </svg>
  );
}

export function Welcome({ onContinue }: { onContinue: () => void }) {
  const { ws, refreshWorkspace } = useApp();
  const { pending, run } = useAction();
  const [error, setError] = useState<CmdError | null>(null);

  const toast = useToast();
  const [confirming, setConfirming] = useState<null | "set-aside" | "delete">(null);
  const returning = isReturningToSample(ws);
  async function openSynthetic() {
    const r = await run<unknown>("synthetic", "workspace_open_synthetic");
    if (!r.ok) { setError(r.error); return; }
    if (recoveredInterruptedSeed(r.data)) toast({ kind: "completed", ...RECOVERED_SEED_NOTICE });
    await refreshWorkspace();
  }
  async function setAside() {
    const r = await run<unknown>("set-aside", "workspace_set_aside_synthetic");
    setConfirming(null);
    if (!r.ok) { setError(r.error); return; }
    setError(null);
    toast({ kind: "completed", title: "Sample workspace set aside", detail: "A new sample is being created." });
    await openSynthetic();
    await refreshWorkspace();
  }
  async function deleteSetAside() {
    const r = await run<unknown>("delete-set-aside", "workspace_delete_set_aside_synthetic");
    setConfirming(null);
    if (!r.ok) { setError(r.error); return; }
    setError(null);
    toast({ kind: "completed", title: "Set-aside sample deleted" });
    await refreshWorkspace();
  }

  return (
    <main className="gate">
      <Topology />
      <div className="gate-welcome-layout">
        <Logo height={28} />
        <div className="stack" style={{ gap: 24 }}>
          <h1 className="gate-welcome-title">{returning ? <>Return to your<br />sample workspace</> : <>Your local AI<br />workspace</>}</h1>
          <div className="gate-welcome-tag">
            <span>Evidence.</span><span>Privacy.</span><span>Scale.</span>
          </div>
          <p className="gate-welcome-hint">{returning ? "Pick up with the same two fictional subjects. There is nothing to sign in to." : "Start with two fictional subjects. No sign-in, payment, or patient data needed."}</p>
          <p className="gate-welcome-hint">Your workspace stays on this computer unless you choose to export it or connect something.</p>
          <div className="gate-welcome-actions">
            <button type="button" className="btn btn-p lg" onClick={openSynthetic} disabled={!!pending} autoFocus>
              {pending ? <Glyph kind="running" /> : <span className="g g-synthetic" />}{pending ? "Opening sample workspace…" : "Try MedScale with sample data"}
            </button>
            <button type="button" className="btn btn-s lg" onClick={onContinue} disabled={!!pending}>{ws?.encrypted_vault_exists ? "Unlock your encrypted workspace" : "Create an encrypted workspace"}</button>
          </div>
          {pending && <p className="t-sm i3">First open ingests the synthetic FHIR fixtures through Core. This takes a few seconds once.</p>}
          {error && (
            <FailureAlert error={error}>
              {seedFailureGuidance(error.reason)?.category === "retry" && <button type="button" className="btn btn-s" onClick={openSynthetic} disabled={!!pending}>Try again</button>}
              {setAsideOffered(error.reason) && (
                <button type="button" className="btn btn-s" aria-expanded={confirming === "set-aside"} onClick={() => setConfirming("set-aside")} disabled={!!pending}>
                  Set the sample workspace aside…
                </button>
              )}
            </FailureAlert>
          )}
          {(ws?.set_aside_exists || error?.reason === "set_aside_slot_occupied") && (
            <p className="t-sm i2" style={{ marginTop: 10, whiteSpace: "normal" }}>
              {SET_ASIDE_EXISTS_NOTICE}{" "}
              <button type="button" className="btn btn-g" aria-expanded={confirming === "delete"} onClick={() => setConfirming("delete")} disabled={!!pending}>Delete set-aside sample…</button>
            </p>
          )}
          {confirming === "set-aside" && (
            <ConfirmPanel title={SET_ASIDE_DIALOG.title} confirmLabel={SET_ASIDE_DIALOG.confirm} busy={!!pending}
              onConfirm={setAside} onCancel={() => setConfirming(null)}>{SET_ASIDE_DIALOG.body}</ConfirmPanel>
          )}
          {confirming === "delete" && (
            <ConfirmPanel title={DELETE_SET_ASIDE_DIALOG.title} confirmLabel={DELETE_SET_ASIDE_DIALOG.confirm} busy={!!pending}
              onConfirm={deleteSetAside} onCancel={() => setConfirming(null)}>{DELETE_SET_ASIDE_DIALOG.body}</ConfirmPanel>
          )}
        </div>
        <div className="gate-welcome-foot">
          <span className="st weak"><Glyph kind="available" />Beta · synthetic data only · runs on this device</span>
          <span>{ws?.network_default_deny ? "No network access unless you allow it" : "Network status unknown"}</span>
          <span className="mono">v{ws?.version}</span>
        </div>
      </div>
    </main>
  );
}

export function Access({ onBack, onRecoveryCodes }: { onBack: () => void; onRecoveryCodes: (shown: boolean) => void }) {
  const { ws, refreshWorkspace } = useApp();
  const exists = !!ws?.encrypted_vault_exists;
  const [mode, setMode] = useState<"unlock" | "create">(exists ? "unlock" : "create");
  const [pass, setPass] = useState("");
  const [confirm, setConfirm] = useState("");
  const [show, setShow] = useState(false);
  const [useCode, setUseCode] = useState(false);
  const [error, setError] = useState<CmdError | null>(null);
  const [codes, setCodes] = useState<string[] | null>(null);
  const [copy, setCopy] = useState<"idle" | "copied" | "failed">("idle");
  const [createFailed, setCreateFailed] = useState(false);
  // Core can fail after sealing the vault, and then the only way forward is to unlock it.
  useEffect(() => {
    if (exists && createFailed) setMode("unlock");
  }, [exists, createFailed]);
  async function copyCodes() {
    try {
      // Write-only Web Clipboard API on an explicit click.
      await navigator.clipboard.writeText((codes ?? []).join("\n"));
      setCopy("copied");
    } catch {
      setCopy("failed");
    }
  }
  const { pending, run } = useAction();
  const holdingCodes = !!codes || pending === "create";
  // A reload would restart the app with the vault open and the codes gone.
  useEffect(() => {
    if (!holdingCodes) return;
    const block = (e: KeyboardEvent) => {
      if (e.key === "F5" || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "r")) e.preventDefault();
    };
    // The default context menu offers Refresh, which a key block can't stop.
    const noMenu = (e: MouseEvent) => e.preventDefault();
    window.addEventListener("keydown", block, true);
    window.addEventListener("contextmenu", noMenu, true);
    return () => {
      window.removeEventListener("keydown", block, true);
      window.removeEventListener("contextmenu", noMenu, true);
    };
  }, [holdingCodes]);
  // Unlocking is a short, familiar task: the explanation waits behind a disclosure.
  const compact = exists && mode === "unlock" && !codes;

  async function submit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    if (mode === "create" && pass !== confirm) { setError({ kind: "invalid", message: "Invalid: passphrases do not match" }); return; }
    if (mode === "unlock") {
      const r = useCode
        ? await run<unknown>("unlock", "workspace_unlock_with_recovery_code", { code: pass })
        : await run<unknown>("unlock", "workspace_unlock", { passphrase: pass });
      if (r.ok) {
        setPass("");
        const notice = vaultOpenNotice(r.data);
        if (notice) toast(notice);
        await refreshWorkspace();
      } else setError(r.error);
    } else {
      // Held from before the call: leaving now would drop codes that are shown only once.
      onRecoveryCodes(true);
      const r = await run<{ recovery_codes: string[] }>("create", "workspace_create_encrypted", { passphrase: pass });
      if (r.ok) { setPass(""); setConfirm(""); setCodes(r.data.recovery_codes); } else {
        setError(r.error);
        setCreateFailed(true);
        onRecoveryCodes(false);
        // Core can fail after the vault exists, and the screen has to show that.
        await refreshWorkspace();
      }
    }
  }

  const toast = useToast();
  const [moveAside, setMoveAside] = useState(false);
  async function moveSessionAside() {
    setError(null);
    const r = await run<unknown>("unlock", "workspace_unlock_moving_session_aside", useCode ? { code: pass } : { passphrase: pass });
    setMoveAside(false);
    if (!r.ok) { setError(r.error); return; }
    setPass("");
    const notice = vaultOpenNotice(r.data);
    if (notice) toast(notice);
    await refreshWorkspace();
  }
  async function synthetic() {
    const r = await run<unknown>("synthetic", "workspace_open_synthetic");
    if (!r.ok) { setError(r.error); return; }
    if (recoveredInterruptedSeed(r.data)) toast({ kind: "completed", ...RECOVERED_SEED_NOTICE });
    await refreshWorkspace();
  }

  const vault = (
    <div>
      <span className="lbl" style={{ display: "block", fontSize: 10.5, fontWeight: 600, letterSpacing: "0.12em", textTransform: "uppercase", color: "var(--ink-3)", marginBottom: 6 }}>Vault</span>
      <div style={{ display: "flex", alignItems: "center", gap: 10, minHeight: 52, padding: "0 12px", border: "1px solid var(--line)", borderRadius: "var(--r-2)" }}>
        <Glyph kind={exists ? "present" : "absent"} />
        <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
          <span className="mono" style={{ fontSize: 12.5 }}>desktop-encrypted</span>
          <span style={{ fontSize: 11.5, color: "var(--ink-3)" }}>{exists ? "Found in this app's data folder" : "Not created yet"}</span>
        </div>
      </div>
    </div>
  );
  const explain = (
    <div className="stack" style={{ gap: 0 }}>
      {[
        ["present", "Decrypts on this device", "The vault key is derived from your passphrase. The passphrase and key are never sent anywhere."],
        ["unavailable", "Does not identify you", "MedScale has no organizational sign-in. Anyone with the passphrase can open this vault."],
        ["absent", "Recovery codes, not resets", "If you lose your passphrase, a recovery code issued at creation opens the vault. Without either, the vault cannot be opened. The passphrase cannot be changed in this version."],
        [ws?.real_phi_authorized ? "available" : "limited", "Real PHI is not authorized", "Use synthetic or permitted fixtures until private-data authorization is granted."],
      ].map(([k, t, d]) => (
        <div key={t} style={{ display: "grid", gridTemplateColumns: "20px minmax(0,1fr)", gap: 12, padding: "12px 0", borderBottom: "1px solid var(--line-subtle)" }}>
          <span style={{ marginTop: 5 }}><Glyph kind={k as "present"} /></span>
          <div><div style={{ fontSize: 13, fontWeight: 600 }}>{t}</div><div className="t-sm i2" style={{ lineHeight: "18px" }}>{d}</div></div>
        </div>
      ))}
    </div>
  );

  return (
    <main className="gate">
      <div className={compact ? "access access-compact" : "access"}>
        <section>
          {codes || pending === "create" ? <Logo height={18} /> : (
            <button type="button" onClick={onBack} style={{ display: "flex", alignItems: "center", gap: 12, width: "max-content" }} aria-label="Back to Welcome">
              <Logo height={18} />
            </button>
          )}
          <div style={{ flex: 1, display: "flex", alignItems: "center", padding: "32px 0" }}>
            {codes ? (
              <div className="stack" style={{ width: 420, maxWidth: "100%", gap: 18 }}>
                <h1 style={{ margin: 0, fontSize: 32, lineHeight: "36px", fontWeight: 600, letterSpacing: "-0.025em" }}>Save your recovery codes</h1>
                <p className="t-body i2">Each code opens this vault if you lose your passphrase. They are shown once and are not stored anywhere MedScale can show them again, so closing MedScale now loses them. Keep them somewhere safe and separate.</p>
                <div className="codes" aria-label="Recovery codes">{codes.length ? codes.map((c) => <span key={c}>{c}</span>) : <span className="i3">Core issued no recovery codes for this vault.</span>}</div>
                <div className="row-gap" style={{ gap: 8 }}>
                  <button type="button" className="btn btn-s lg" onClick={() => void copyCodes()} disabled={!codes.length}>{copy === "copied" ? "Copied" : "Copy codes"}</button>
                  {copy === "failed" && <span className="t-sm i2" role="status">Couldn&apos;t copy. Select the codes and press Ctrl+C.</span>}
                </div>
                <div><button type="button" className="btn btn-p lg" onClick={() => void refreshWorkspace().finally(() => onRecoveryCodes(false))}>I saved them · continue</button></div>
              </div>
            ) : (
              <form onSubmit={submit} className="stack" style={{ width: 420, maxWidth: "100%", gap: 22 }}>
                <div className="stack" style={{ gap: 8 }}>
                  <h1 style={{ margin: 0, fontSize: 32, lineHeight: "36px", fontWeight: 600, letterSpacing: "-0.025em" }}>{mode === "unlock" ? "Unlock workspace" : "Create encrypted vault"}</h1>
                  <p className="t-body i2">{compact ? "Enter your passphrase to open this workspace on this device." : "Your passphrase decrypts this vault on this device. It is not an account and it is never sent anywhere."}</p>
                </div>
                {!compact && vault}
                <div>
                  <label className="lbl" htmlFor="pp">{useCode ? "Recovery code" : "Passphrase"}</label>
                  <div className="field-row">
                    <input id="pp" type={show ? "text" : "password"} value={pass} onChange={(e) => setPass(e.target.value)} autoComplete={useCode ? "off" : mode === "unlock" ? "current-password" : "new-password"} aria-invalid={error?.kind === "invalid" || error?.kind === "denied"} style={{ flex: 1, height: 36 }} autoFocus />
                    <button type="button" className="btn btn-g" onClick={() => setShow((s) => !s)}>{show ? "Hide" : "Show"}</button>
                  </div>
                  <span className="t-sm i3" style={{ display: "block", marginTop: 6 }}>{useCode ? "A code from when this vault was created. It opens the vault and stays valid; the passphrase cannot be changed in this version." : "8–256 characters. Held in memory only while the vault is open."}</span>
                  {mode === "unlock" && <button type="button" className="link t-sm" style={{ marginTop: 6 }} onClick={() => { setUseCode((c) => !c); setPass(""); setError(null); }}>{useCode ? "Use my passphrase" : "Use a recovery code"}</button>}
                </div>
                {mode === "create" && (
                  <div>
                    <label className="lbl" htmlFor="pp2">Confirm passphrase</label>
                    <input id="pp2" type={show ? "text" : "password"} value={confirm} onChange={(e) => setConfirm(e.target.value)} autoComplete="new-password" style={{ width: "100%", height: 36 }} />
                  </div>
                )}
                {error && <FailureAlert error={error} glyph={error.kind === "denied" ? "denied" : "failed"} />}
                {createFailed && exists && <p className="t-sm i2" role="status" style={{ margin: 0 }}>The vault was created, but its recovery codes couldn&apos;t be shown. Your passphrase is now the only way to open it.</p>}
                {error?.reason === "work_session_check_failed" && !moveAside && (
                  <div><button type="button" className="btn btn-g" onClick={() => setMoveAside(true)} disabled={!!pending}>Move that session aside and open…</button></div>
                )}
                {moveAside && (
                  <ConfirmPanel title={MOVE_SESSION_ASIDE_DIALOG.title} confirmLabel={MOVE_SESSION_ASIDE_DIALOG.confirm} busy={!!pending}
                    onConfirm={moveSessionAside} onCancel={() => setMoveAside(false)}>{MOVE_SESSION_ASIDE_DIALOG.body}</ConfirmPanel>
                )}
                <div className="row-gap" style={{ gap: 14 }}>
                  <button type="submit" className="btn btn-p lg" disabled={!!pending || (useCode ? !pass.trim() : pass.length < 8)}>{pending === "unlock" || pending === "create" ? <><Glyph kind="running" />Working…</> : mode === "unlock" ? "Unlock workspace" : "Create vault"}</button>
                  {exists && !compact
                    ? <button type="button" className="link t-sm" onClick={() => { setMode(mode === "unlock" ? "create" : "unlock"); setError(null); }} disabled={mode === "unlock"} title={mode === "unlock" ? "A vault already exists at the default location" : undefined}>{mode === "unlock" ? "One vault per device in this build" : "Unlock existing vault"}</button>
                    : null}
                </div>
                {compact ? (
                  <>
                    <button type="button" className="link t-sm access-sample" onClick={onBack} disabled={!!pending}>Explore sample data instead</button>
                    <details className="access-details">
                      <summary>How unlocking works</summary>
                      <div className="stack" style={{ gap: 16, marginTop: 12 }}>{vault}{explain}<p className="t-sm i3">This build keeps one vault per device.</p></div>
                    </details>
                  </>
                ) : (
                  <>
                    <div style={{ display: "flex", alignItems: "center", gap: 12 }}><span style={{ flex: 1, height: 1, background: "var(--line)" }} /><span className="t-sm i3">or</span><span style={{ flex: 1, height: 1, background: "var(--line)" }} /></div>
                    <button type="button" className="btn btn-s lg" style={{ justifyContent: "flex-start" }} onClick={synthetic} disabled={!!pending}>
                      {pending === "synthetic" ? <Glyph kind="running" /> : <span className="g g-synthetic" />}Open synthetic workspace
                      <span style={{ marginLeft: "auto", fontSize: 11.5, color: "var(--ink-3)", fontWeight: 400 }}>fixtures only · no passphrase</span>
                    </button>
                  </>
                )}
              </form>
            )}
          </div>
          <div className="row-gap" style={{ gap: 20, fontSize: 12, color: "var(--ink-3)" }}><span>No sign-in · no accounts · no cloud</span>{compact && !ws?.real_phi_authorized && <span>Real patient data is not authorized</span>}<span className="mono" style={{ fontSize: 11 }}>v{ws?.version}</span></div>
        </section>
        {!compact && (
          <aside>
            <div className="sec" style={{ margin: 0 }}><span className="sec-n">01</span><span className="sec-t">What unlocking does</span><span className="sec-rule" /></div>
            {explain}
          </aside>
        )}
      </div>
    </main>
  );
}
