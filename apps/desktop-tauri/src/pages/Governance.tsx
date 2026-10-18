import { useState, type FormEvent, type ReactNode } from "react";
import { useApp } from "../lib/app";
import { useAction, useCommand } from "../lib/ipc";
import { ProjectScope } from "../components/project";
import { MOD } from "../components/shell";
import { Authority, Empty, ErrorState, KV, Ledger, Mark, Notice, PageHead, Section, Skeleton, Status, glyphFor, useOutcome } from "../components/ui";

type Row = { label: string; value: string; detail: string; status: string };
type Gov = {
  synthetic_only: boolean; audit_boundary: string; export_boundary: string; settings_boundary: string; integrations_boundary: string;
  audit_rows: Array<{ record_id: string; action: string; subject: string; detail: string; status: string }>;
  export_rows: Row[]; settings_rows: Row[]; integration_rows: Row[]; missing_release_evidence: string;
  fhir_support: { fhir_version: string; synthetic_only: boolean; full_conformance_claimed: boolean; resources: Array<Record<string, unknown>>; limitations: string[] } | null;
};

function useGov() { return useCommand<Gov>("governance_overview"); }

function RowList({ rows }: { rows: Row[] }) {
  return <>{rows.map((r) => (
    <div key={r.label} style={{ display: "grid", gridTemplateColumns: "minmax(0,1fr) 240px", gap: 24, alignItems: "center", minHeight: 56, padding: "8px 0", borderBottom: "1px solid var(--line-subtle)" }}>
      <div><div style={{ fontSize: 13, fontWeight: 500 }}>{r.label}</div><div className="t-sm i3" style={{ marginTop: 2 }}>{r.detail}</div></div>
      <div className="stack" style={{ gap: 2 }}><Status kind={glyphFor(r.status) === "unknown" ? glyphFor(r.value) : glyphFor(r.status)} weight="strong">{r.value}</Status><span className="t-sm i3">{r.status}</span></div>
    </div>
  ))}</>;
}

/* ───────────────────────────── Privacy ───────────────────────────── */
type PrivacyVm = {
  classifications: Array<{ artifact_id: string; data_class: string; basis: string; revision: number }>;
  receipts: Array<{ id: string; source: string; output: string; status: string; residual: string; revision: number }>;
  decisions: Array<{ artifact_id: string; boundary: string; outcome: string; reason: string }>;
};

export function Privacy() {
  return <ProjectScope title="Privacy" sub="Data classes, transform receipts and egress decisions. Every boundary crossing is evaluated, never assumed.">{(pid) => <PrivacyBody projectId={pid} />}</ProjectScope>;
}

function PrivacyBody({ projectId }: { projectId: string }) {
  const o = useCommand<PrivacyVm>("privacy_overview", { projectId });
  const docs = useCommand<{ sources: Array<{ source_id: string; resource_type: string }> }>("documents_list");
  const [artifact, setArtifact] = useState("");
  const [boundary, setBoundary] = useState("browse");
  const { pending, run } = useAction();
  const outcome = useOutcome();
  async function check(e: FormEvent) { e.preventDefault(); const r = await run("check", "privacy_check_egress", { projectId, artifactId: artifact, boundary }); if (outcome(r, "Egress decision recorded")) await o.reload(); }
  if (o.state.status === "loading") return <Skeleton />;
  if (o.state.status === "error") return <ErrorState error={o.state.error} onRetry={o.reload} />;
  const v = o.state.data;
  return (
    <>
      <Section n="01" title="Check egress" count="Core evaluates; Desktop only asks">
        <form onSubmit={check} className="field-row">
          <select value={artifact} onChange={(e) => setArtifact(e.target.value)} aria-label="Artifact" style={{ flex: 1 }}>
            <option value="">Choose an ingested source…</option>
            {docs.state.status === "ready" && docs.state.data.sources.map((s) => <option key={s.source_id} value={s.source_id}>{s.resource_type} · {s.source_id}</option>)}
          </select>
          <select value={boundary} onChange={(e) => setBoundary(e.target.value)} aria-label="Boundary"><option value="browse">browse</option><option value="data_source_export">data_source_export</option><option value="data_source_write">data_source_write</option><option value="model_external_delegate">model_external_delegate</option><option value="network_broker">network_broker</option><option value="connector">connector</option><option value="extension">extension</option><option value="analytics_adapter">analytics_adapter</option><option value="hub">hub</option><option value="compute">compute</option><option value="r_workspace">r_workspace</option></select>
          <button type="submit" className="btn btn-s" disabled={!artifact || !!pending}>Evaluate</button>
        </form>
      </Section>
      <Section n="02" title="Decisions" count={v.decisions.length}>
        {v.decisions.length ? <table className="tb"><thead><tr><th style={{ width: "34%" }}>Artifact</th><th style={{ width: "14%" }}>Boundary</th><th style={{ width: "16%" }}>Outcome</th><th>Reason</th></tr></thead><tbody>{v.decisions.map((d, i) => <tr key={i}><td><span className="mono t-mono-sm trunc">{d.artifact_id}</span></td><td className="t-sm">{d.boundary}</td><td><Status kind={glyphFor(d.outcome)} weight="strong">{d.outcome}</Status></td><td className="t-sm i2"><span className="trunc">{d.reason}</span></td></tr>)}</tbody></table> : <p className="t-sm i3">No egress decisions recorded.</p>}
      </Section>
      <Section n="03" title="Classifications" count={v.classifications.length}>
        {v.classifications.length ? v.classifications.map((c) => <div key={c.artifact_id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) 160px minmax(0,1fr)" }}><span className="mono t-mono-sm trunc">{c.artifact_id}</span><span className="t-sm">{c.data_class}</span><span className="t-sm i3 trunc">{c.basis}</span></div>) : <p className="t-sm i3">No artifacts classified in this project.</p>}
      </Section>
      <Section n="04" title="Transform receipts" count={v.receipts.length}>
        {v.receipts.length ? v.receipts.map((r) => <div key={r.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) 120px minmax(0,1fr)" }}><span className="mono t-mono-sm trunc">{r.id}</span><Status kind={glyphFor(r.status)}>{r.status}</Status><span className="t-sm i3 trunc">{r.residual}</span></div>) : <p className="t-sm i3">No pseudonymization receipts.</p>}
      </Section>
      <Authority>Default deny. A positive decision covers one artifact at one boundary; it is not a standing permission.</Authority>
    </>
  );
}

/* ───────────────────────────── Audit, Exports, Integrations ───────────────────────────── */
export function AuditTrail() {
  const { state, reload } = useGov();
  return (
    <div className="pane">
      <PageHead title="Audit Trail" sub="Disclosure history recorded by Core. Nothing here is editable." />
      {state.status === "loading" && <Skeleton />}
      {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
      {state.status === "ready" && <>
        {state.data.audit_rows.length ? <table className="tb"><thead><tr><th style={{ width: "24%" }}>Record</th><th style={{ width: "18%" }}>Action</th><th style={{ width: "20%" }}>Subject</th><th style={{ width: "14%" }}>Status</th><th>Detail</th></tr></thead>
          <tbody>{state.data.audit_rows.map((r) => <tr key={r.record_id}><td><span className="mono t-mono-sm trunc">{r.record_id}</span></td><td className="t-sm">{r.action}</td><td className="t-sm i2"><span className="trunc">{r.subject}</span></td><td><Status kind={glyphFor(r.status)}>{r.status}</Status></td><td className="t-sm i3"><span className="trunc">{r.detail}</span></td></tr>)}</tbody></table>
          : <Empty icon="audit" title="No disclosures recorded">Core records a disclosure whenever data leaves the vault through an authorized path. None have happened in this workspace.</Empty>}
        <p className="t-sm i3" style={{ marginTop: 12 }}>{state.data.audit_boundary}</p>
      </>}
    </div>
  );
}

export function Exports() {
  const { state, reload } = useGov();
  return (
    <div className="pane">
      <PageHead title="Exports" sub="Loss-aware interchange. What can leave, in which format, and what would be lost." />
      {state.status === "loading" && <Skeleton />}
      {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
      {state.status === "ready" && <>
        <RowList rows={state.data.export_rows} />
        {state.data.fhir_support && <Section n="02" title="FHIR support matrix" count={`R${state.data.fhir_support.fhir_version}`}>
          <Ledger items={[{ k: "full_conformance", v: state.data.fhir_support.full_conformance_claimed ? "Claimed" : "Not claimed" }, { k: "synthetic_only", v: String(state.data.fhir_support.synthetic_only), mono: true }, { k: "resources", v: state.data.fhir_support.resources.length, mono: true }]} />
          <table className="tb"><thead><tr><th style={{ width: "26%" }}>Resource</th><th>Support</th></tr></thead>
            <tbody>{state.data.fhir_support.resources.map((r, i) => <tr key={i}><td className="t-sm">{String(r.resource_type ?? r.resource ?? `resource ${i + 1}`)}</td><td className="t-sm i2"><span className="trunc">{Object.entries(r).filter(([k]) => k !== "resource_type").map(([k, v]) => `${k}: ${typeof v === "object" ? JSON.stringify(v) : String(v)}`).join(" · ")}</span></td></tr>)}</tbody></table>
          {state.data.fhir_support.limitations.map((l) => <p key={l} className="t-sm i3" style={{ marginTop: 6 }}>{l}</p>)}
        </Section>}
        <p className="t-sm i3" style={{ marginTop: 12 }}>{state.data.export_boundary}</p>
      </>}
    </div>
  );
}

export function Integrations() {
  const { state, reload } = useGov();
  return (
    <div className="pane">
      <PageHead title="Integrations" sub="Brokered integration posture. Nothing connects without the network broker and an external gate." />
      {state.status === "loading" && <Skeleton />}
      {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
      {state.status === "ready" && <><RowList rows={state.data.integration_rows} /><p className="t-sm i3" style={{ marginTop: 12 }}>{state.data.integrations_boundary}</p></>}
    </div>
  );
}

/* ───────────────────────────── Settings ───────────────────────────── */
export function Settings() {
  const { theme, setTheme, density, setDensity, ws, lock } = useApp();
  const gov = useCommand<Gov>(ws?.open ? "governance_overview" : null);

  const Seg = <T extends string>({ value, options, onChange, label }: { value: T; options: T[]; onChange: (v: T) => void; label: string }) => (
    <div role="radiogroup" aria-label={label} className="seg">{options.map((o) => <button key={o} type="button" role="radio" aria-checked={value === o} onClick={() => onChange(o)}>{o[0]!.toUpperCase() + o.slice(1)}</button>)}</div>
  );
  const SettingRow = ({ label, detail, children }: { label: string; detail: string; children: ReactNode }) => (
    <div style={{ display: "grid", gridTemplateColumns: "minmax(0,1fr) 280px", gap: 24, alignItems: "center", minHeight: 56, borderBottom: "1px solid var(--line-subtle)" }}>
      <div><div style={{ fontSize: 13, fontWeight: 500 }}>{label}</div><div className="t-sm i3" style={{ marginTop: 2 }}>{detail}</div></div><div>{children}</div>
    </div>
  );

  return (
    <div className="pane">
      <div style={{ maxWidth: 860 }}>
        <PageHead title="Settings" sub="Read-only facts are shown as status. Anything you can change is a control." />
        <Section n="01" title="Appearance">
          <SettingRow label="Theme" detail="Applies until MedScale restarts; this preference is not saved. Also available from the command palette."><Seg label="Theme" value={theme} options={["dark", "light"]} onChange={setTheme} /></SettingRow>
          <SettingRow label="Density" detail="Compact tightens rows, padding and gaps. Type size does not change."><Seg label="Density" value={density} options={["standard", "compact"]} onChange={setDensity} /></SettingRow>
          <SettingRow label="Reduce motion" detail="Follows the operating system. State changes stay immediate."><Status kind="available">Follows system</Status></SettingRow>
        </Section>
        <Section n="02" title="Workspace">
          <SettingRow label="Open workspace" detail={ws?.workspace ? `${ws.workspace.subjects} subjects · ${ws.workspace.sources} sources promoted by this desktop` : "No vault is open."}>
            {ws?.workspace ? <span className="mono" style={{ fontSize: 12 }}>{ws.workspace.vault_id}</span> : <Status kind="unavailable">None</Status>}
          </SettingRow>
          {ws?.workspace?.kind === "encrypted" && <SettingRow label="Clinical ingest" detail="Core admits FHIR ingest into synthetic vaults only until private-data authorization is granted. This encrypted vault holds projects, data, analytics and collaboration."><Status kind="limited">Not authorized</Status></SettingRow>}
          <SettingRow label="Lock workspace" detail="Closes the vault and releases keys from memory.">
            <span className="row-gap" style={{ gap: 8 }}><button type="button" className="btn btn-s" onClick={() => void lock()} disabled={!ws?.open}>Lock now</button><span className="kbd">{MOD} Shift L</span></span>
          </SettingRow>
        </Section>
        {gov.state.status === "ready" && <>
          <Section n="03" title="Privacy & network"><RowList rows={gov.state.data.settings_rows.filter((r) => !/release|accessib/i.test(r.label))} /></Section>
          <Section n="04" title="Qualification" count="read-only"><RowList rows={gov.state.data.settings_rows.filter((r) => /release|accessib/i.test(r.label))} /><p className="t-sm i3" style={{ marginTop: 10 }}>{gov.state.data.missing_release_evidence}</p></Section>
        </>}
        {gov.state.status === "error" && <ErrorState error={gov.state.error} onRetry={gov.reload} />}
        <Section n="05" title="Keyboard">
          <KV keyWidth={150} rows={[[`${MOD} K`, "Search or run a command"], [`${MOD} ,`, "Settings"], [`${MOD} Shift L`, "Lock workspace"], ["Enter", "Open the focused row"], ["Space", "Preview the focused row"], ["Esc", "Close the palette or dialog"]]} />
        </Section>
      </div>
    </div>
  );
}

/* ───────────────────────────── About ───────────────────────────── */
type AboutVm = { product: string; tagline: string; version: string; shell: string; core: string; local_only: boolean; network_default_deny: boolean; license: string; build_profile: string; target: string; release_ready: boolean };

export function About() {
  const { state, reload } = useCommand<AboutVm>("about_info");
  if (state.status === "loading") return <div className="pane"><Skeleton /></div>;
  if (state.status === "error") return <div className="pane"><ErrorState error={state.error} onRetry={reload} /></div>;
  const a = state.data;
  return (
    <div className="pane">
      <div style={{ maxWidth: 980 }} className="stack">
        <div className="row-gap" style={{ alignItems: "flex-end", gap: 40, paddingBottom: 32, borderBottom: "1px solid var(--line)", marginBottom: 28 }}>
          <Mark width={132} />
          <div className="stack" style={{ gap: 6, flex: 1, minWidth: 260 }}><span style={{ fontSize: 40, lineHeight: "40px", fontWeight: 600, letterSpacing: "-0.035em" }}>{a.product}</span><span style={{ fontSize: 15, color: "var(--ink-2)" }}>{a.tagline}</span></div>
          <div className="stack" style={{ alignItems: "flex-end", gap: 6 }}><span className="mono" style={{ fontSize: 20 }}>{a.version}</span><span style={{ fontSize: 11, letterSpacing: "0.28em", textTransform: "uppercase", color: "var(--ink-3)" }}>Evidence. Privacy. Scale.</span></div>
        </div>
        <div className="grid-2">
          <Section n="01" title="Build"><KV keyWidth={110} rows={[["version", <span className="mono t-mono-sm">{a.version}</span>], ["profile", <span className="mono t-mono-sm">{a.build_profile}</span>], ["target", <span className="mono t-mono-sm">{a.target}</span>], ["license", <span className="mono t-mono-sm">{a.license}</span>], ["build status", a.release_ready ? <Status kind="available">Release qualified</Status> : <Status kind="limited">Preview build · not release-qualified</Status>]]} /></Section>
          <Section n="02" title="Local runtime"><KV keyWidth={110} rows={[["shell", a.shell], ["core", a.core], ["network", <Status kind={a.network_default_deny ? "blocked" : "unknown"}>{a.network_default_deny ? "Default deny" : "Not verified"}</Status>], ["locality", <Status kind={a.local_only ? "available" : "unknown"}>{a.local_only ? "Local only" : "Unknown"}</Status>]]} /></Section>
          <Section n="03" title="Licenses"><KV keyWidth={110} rows={[["MedScale", <span className="mono t-mono-sm">Apache-2.0</span>], ["Inter", <span className="mono t-mono-sm">OFL-1.1</span>], ["JetBrains Mono", <span className="mono t-mono-sm">OFL-1.1</span>], ["third party", "See docs/legal/NOTICE_INVENTORY.md"]]} /></Section>
          <Section n="04" title="Provenance"><KV keyWidth={110} rows={[["dependencies", "cargo-deny policy and supply-chain audits"], ["signing", <Status kind="pending">Not claimed</Status>], ["clinical use", <Status kind="blocked">Not authorized · synthetic only</Status>]]} /></Section>
        </div>
        <Notice kind="synthetic" title="Synthetic fixtures.">Patients, evidence and model packs in the synthetic workspace come from the repository's synthetic fixtures. They are not real records.</Notice>
      </div>
    </div>
  );
}
