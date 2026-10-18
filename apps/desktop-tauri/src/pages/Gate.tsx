import { useMemo, useState, type FormEvent } from "react";
import { useApp } from "../lib/app";
import { useAction, type CmdError } from "../lib/ipc";
import { Glyph, Mark } from "../components/ui";

/** Provenance topology: a height field rendered as dots, with a source →
 *  digest → assertion → projection → review chain. Decorative; the chain uses
 *  the sky-blue accent and the review node the orange attention accent. */
function Topology() {
  const { dots, nodes, chain } = useMemo(() => {
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
    const spec = [
      { u: 0.3, v: 0.72, label: "source", fill: true, dash: false },
      { u: 0.47, v: 0.52, label: "digest", fill: true, dash: false },
      { u: 0.6, v: 0.42, label: "assertion", fill: true, dash: false },
      { u: 0.76, v: 0.55, label: "projection", fill: false, dash: false },
      { u: 0.86, v: 0.78, label: "review", fill: false, dash: true },
    ];
    const nodes = spec.map((s) => ({ ...s, ...P(s.u, s.v) }));
    return { dots, nodes, chain: nodes.map((n) => `${n.x},${n.y}`).join(" ") };
  }, []);
  return (
    <svg className="gate-topo" viewBox="0 0 900 760" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
      {dots.map((d, i) => <circle key={i} cx={d.x} cy={d.y} r={d.r} fill="currentColor" fillOpacity={d.o} />)}
      <polyline points={chain} fill="none" stroke="var(--brand-blue)" strokeOpacity={0.9} strokeWidth={1.1} />
      {nodes.map((n) => (
        <g key={n.label}>
          <rect x={n.x - 4} y={n.y - 4} width={8} height={8} fill={n.fill ? (n.label === "assertion" ? "var(--brand-blue)" : "currentColor") : "var(--bg-app)"} stroke={n.label === "review" ? "var(--brand-orange)" : n.label === "assertion" ? "var(--brand-blue)" : "currentColor"} strokeWidth={1.2} strokeDasharray={n.dash ? "2 1.6" : undefined} />
          <line x1={n.x} y1={n.y - 4} x2={n.x} y2={n.y - 50} stroke="currentColor" strokeOpacity={0.35} strokeWidth={0.8} />
          <text x={n.x + 6} y={n.y - 46} fill="currentColor" fillOpacity={0.75} style={{ fontFamily: "var(--font-mono)", fontSize: 10.5 }}>{n.label}</text>
        </g>
      ))}
    </svg>
  );
}

export function Welcome({ onContinue }: { onContinue: () => void }) {
  const { ws, refreshWorkspace } = useApp();
  const { pending, run } = useAction();
  const [error, setError] = useState<CmdError | null>(null);

  async function openSynthetic() {
    const r = await run("synthetic", "workspace_open_synthetic");
    if (r.ok) await refreshWorkspace(); else setError(r.error);
  }

  return (
    <main className="gate">
      <Topology />
      <div className="gate-fade" />
      <div style={{ position: "relative", height: "100%", minHeight: 600, display: "flex", flexDirection: "column", justifyContent: "space-between", padding: "56px 64px 48px", maxWidth: 640 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
          <Mark width={46} />
          <span style={{ fontSize: 24, fontWeight: 600, letterSpacing: "-0.03em" }}>MedScale</span>
        </div>
        <div className="stack" style={{ gap: 28 }}>
          <h1 style={{ margin: 0, fontSize: 60, lineHeight: "60px", fontWeight: 600, letterSpacing: "-0.04em" }}>Local Clinical<br />Intelligence</h1>
          <span className="brand-rule" aria-hidden="true" />
          <div style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: 22, lineHeight: "30px", fontWeight: 500, letterSpacing: "-0.015em", color: "var(--ink-2)" }}>
            <span>Evidence.</span><span>Privacy.</span><span>Scale.</span>
          </div>
          <div className="row-gap" style={{ gap: 10, marginTop: 12 }}>
            <button type="button" className="btn btn-p lg" onClick={onContinue} autoFocus>Open workspace</button>
            <button type="button" className="btn btn-s lg" onClick={openSynthetic} disabled={!!pending}>
              {pending ? <Glyph kind="running" /> : <span className="g g-synthetic" />}{pending ? "Opening synthetic workspace…" : "Open synthetic workspace"}
            </button>
          </div>
          {pending && <p className="t-sm i3">First open ingests the synthetic FHIR fixtures through Core. This takes a few seconds once.</p>}
          {error && <p className="st" role="alert"><Glyph kind="failed" />{error.message}</p>}
        </div>
        <div className="row-gap" style={{ gap: 20, fontSize: 12, color: "var(--ink-3)" }}>
          <span className="st weak"><Glyph kind="available" />Runs on this device</span>
          <span>{ws?.network_default_deny ? "Network egress off by default" : "Network posture unknown"}</span>
          <span className="mono" style={{ fontSize: 11 }}>v{ws?.version}</span>
        </div>
      </div>
    </main>
  );
}

export function Access({ onBack }: { onBack: () => void }) {
  const { ws, refreshWorkspace } = useApp();
  const exists = !!ws?.encrypted_vault_exists;
  const [mode, setMode] = useState<"unlock" | "create">(exists ? "unlock" : "create");
  const [pass, setPass] = useState("");
  const [confirm, setConfirm] = useState("");
  const [show, setShow] = useState(false);
  const [error, setError] = useState<CmdError | null>(null);
  const [codes, setCodes] = useState<string[] | null>(null);
  const { pending, run } = useAction();

  async function submit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    if (mode === "create" && pass !== confirm) { setError({ kind: "invalid", message: "Invalid: passphrases do not match" }); return; }
    if (mode === "unlock") {
      const r = await run("unlock", "workspace_unlock", { passphrase: pass });
      if (r.ok) { setPass(""); await refreshWorkspace(); } else setError(r.error);
    } else {
      const r = await run<{ recovery_codes: string[] }>("create", "workspace_create_encrypted", { passphrase: pass });
      if (r.ok) { setPass(""); setConfirm(""); setCodes(r.data.recovery_codes); } else setError(r.error);
    }
  }

  async function synthetic() {
    const r = await run("synthetic", "workspace_open_synthetic");
    if (r.ok) await refreshWorkspace(); else setError(r.error);
  }

  return (
    <main className="gate">
      <div className="access">
        <section>
          <button type="button" onClick={onBack} style={{ display: "flex", alignItems: "center", gap: 12, width: "max-content" }} aria-label="Back to Welcome">
            <Mark width={32} /><span style={{ fontSize: 16, fontWeight: 600, letterSpacing: "-0.02em" }}>MedScale</span>
          </button>
          <div style={{ flex: 1, display: "flex", alignItems: "center", padding: "32px 0" }}>
            {codes ? (
              <div className="stack" style={{ width: 420, maxWidth: "100%", gap: 18 }}>
                <h1 style={{ margin: 0, fontSize: 32, lineHeight: "36px", fontWeight: 600, letterSpacing: "-0.025em" }}>Save your recovery codes</h1>
                <p className="t-body i2">These replace a lost passphrase. They are shown once and are not stored anywhere MedScale can show them again.</p>
                <div className="codes" aria-label="Recovery codes">{codes.length ? codes.map((c) => <span key={c}>{c}</span>) : <span className="i3">Core issued no recovery codes for this vault.</span>}</div>
                <div><button type="button" className="btn btn-p lg" onClick={() => void refreshWorkspace()}>I saved them · continue</button></div>
              </div>
            ) : (
              <form onSubmit={submit} className="stack" style={{ width: 420, maxWidth: "100%", gap: 22 }}>
                <div className="stack" style={{ gap: 8 }}>
                  <h1 style={{ margin: 0, fontSize: 32, lineHeight: "36px", fontWeight: 600, letterSpacing: "-0.025em" }}>{mode === "unlock" ? "Unlock workspace" : "Create encrypted vault"}</h1>
                  <p className="t-body i2">Your passphrase decrypts this vault on this device. It is not an account and it is never sent anywhere.</p>
                </div>
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
                <div>
                  <label className="lbl" htmlFor="pp">Passphrase</label>
                  <div className="field-row">
                    <input id="pp" type={show ? "text" : "password"} value={pass} onChange={(e) => setPass(e.target.value)} autoComplete={mode === "unlock" ? "current-password" : "new-password"} aria-invalid={error?.kind === "invalid" || error?.kind === "denied"} style={{ flex: 1, height: 40 }} autoFocus />
                    <button type="button" className="btn btn-g" onClick={() => setShow((s) => !s)}>{show ? "Hide" : "Show"}</button>
                  </div>
                  <span className="t-sm i3" style={{ display: "block", marginTop: 6 }}>8–256 characters. Held in memory only while the vault is open.</span>
                </div>
                {mode === "create" && (
                  <div>
                    <label className="lbl" htmlFor="pp2">Confirm passphrase</label>
                    <input id="pp2" type={show ? "text" : "password"} value={confirm} onChange={(e) => setConfirm(e.target.value)} autoComplete="new-password" style={{ width: "100%", height: 40 }} />
                  </div>
                )}
                {error && <p className="st" role="alert" style={{ whiteSpace: "normal" }}><Glyph kind={error.kind === "denied" ? "denied" : "failed"} />{error.message}</p>}
                <div className="row-gap" style={{ gap: 14 }}>
                  <button type="submit" className="btn btn-p lg" disabled={!!pending || pass.length < 8}>{pending === "unlock" || pending === "create" ? <><Glyph kind="running" />Working…</> : mode === "unlock" ? "Unlock workspace" : "Create vault"}</button>
                  {exists
                    ? <button type="button" className="link t-sm" onClick={() => { setMode(mode === "unlock" ? "create" : "unlock"); setError(null); }} disabled={mode === "unlock"} title={mode === "unlock" ? "A vault already exists at the default location" : undefined}>{mode === "unlock" ? "One vault per device in this build" : "Unlock existing vault"}</button>
                    : null}
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 12 }}><span style={{ flex: 1, height: 1, background: "var(--line)" }} /><span className="t-sm i3">or</span><span style={{ flex: 1, height: 1, background: "var(--line)" }} /></div>
                <button type="button" className="btn btn-s lg" style={{ justifyContent: "flex-start" }} onClick={synthetic} disabled={!!pending}>
                  {pending === "synthetic" ? <Glyph kind="running" /> : <span className="g g-synthetic" />}Open synthetic workspace
                  <span style={{ marginLeft: "auto", fontSize: 11.5, color: "var(--ink-3)", fontWeight: 400 }}>fixtures only · no passphrase</span>
                </button>
              </form>
            )}
          </div>
          <div className="row-gap" style={{ gap: 20, fontSize: 12, color: "var(--ink-3)" }}><span>No sign-in · no accounts · no cloud</span><span className="mono" style={{ fontSize: 11 }}>v{ws?.version}</span></div>
        </section>
        <aside>
          <div className="sec" style={{ margin: 0 }}><span className="sec-n">01</span><span className="sec-t">What unlocking does</span><span className="sec-rule" /></div>
          <div className="stack" style={{ gap: 0 }}>
            {[
              ["present", "Decrypts on this device", "The vault key is derived from your passphrase. Nothing leaves the machine."],
              ["unavailable", "Does not identify you", "MedScale has no organizational sign-in. Anyone with the passphrase can open this vault."],
              ["absent", "Recovery codes, not resets", "A lost passphrase can only be replaced with a recovery code issued at creation."],
              [ws?.real_phi_authorized ? "available" : "limited", "Real PHI is not authorized", "Use synthetic or permitted fixtures until private-data authorization is granted."],
            ].map(([k, t, d]) => (
              <div key={t} style={{ display: "grid", gridTemplateColumns: "20px minmax(0,1fr)", gap: 12, padding: "12px 0", borderBottom: "1px solid var(--line-subtle)" }}>
                <span style={{ marginTop: 5 }}><Glyph kind={k as "present"} /></span>
                <div><div style={{ fontSize: 13, fontWeight: 600 }}>{t}</div><div className="t-sm i2" style={{ lineHeight: "18px" }}>{d}</div></div>
              </div>
            ))}
          </div>
        </aside>
      </div>
    </main>
  );
}
