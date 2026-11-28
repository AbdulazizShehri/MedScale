import { Fragment, useEffect, useState, type FormEvent, type ReactNode } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useApp } from "../lib/app";
import { call, useAction, useCommand, type CmdError } from "../lib/ipc";
import { ProjectScope } from "../components/project";
import { MOD } from "../components/shell";
import { Mark, Wordmark } from "../components/brand";
import {
  Authority,
  CopyButton,
  Empty,
  ErrorState,
  Glyph,
  KV,
  Ledger,
  Notice,
  PageHead,
  SearchField,
  Section,
  SegmentedBar,
  Skeleton,
  Status,
  glyphFor,
  useOutcome,
  useToast,
  type SegmentItem,
} from "../components/ui";
import { SUPPORT_DATA_RULE, SUPPORT_ISSUES_URL, supportDiagnostics } from "../support";
import { ConfirmPanel } from "../components/confirm";
import { DELETE_RECOVERY_DIALOG, DELETE_SET_ASIDE_DIALOG, RESTORE_COPY_DIALOG, SET_ASIDE_EXISTS_NOTICE } from "../lib/seed-failure";
import { formatStorage } from "../model-storage";

type Row = { label: string; value: string; detail: string; status: string };
type Gov = {
  synthetic_only: boolean;
  audit_boundary: string;
  export_boundary: string;
  settings_boundary: string;
  integrations_boundary: string;
  audit_rows: Array<{ record_id: string; action: string; subject: string; detail: string; status: string }>;
  export_rows: Row[];
  settings_rows: Row[];
  integration_rows: Row[];
  missing_release_evidence: string;
  fhir_support: { fhir_version: string; synthetic_only: boolean; full_conformance_claimed: boolean; resources: Array<Record<string, unknown>>; limitations: string[] } | null;
};

function useGov() { return useCommand<Gov>("governance_overview"); }

function RowList({ rows }: { rows: Row[] }) {
  return <>{rows.map((r) => (
    <div key={r.label} className="setting-row">
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

const BOUNDARY_LABELS: Record<string, string> = {
  browse: "Browse (web allowlist)",
  data_source_export: "Data source export",
  data_source_write: "Data source write",
  model_external_delegate: "External model delegation",
  network_broker: "Network broker",
  connector: "External connector",
  extension: "Extension",
  analytics_adapter: "Analytics adapter",
  hub: "Hub",
  compute: "External compute",
  r_workspace: "R workspace",
};

export function Privacy() {
  return <ProjectScope title="Privacy" sub="What kinds of data exist here, how they were transformed, and what may leave. Every crossing is checked, never assumed.">{(pid) => <PrivacyBody projectId={pid} />}</ProjectScope>;
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

  const allowedCount = v.decisions.filter((d) => /^(allow|allowed|permitted)$/i.test(d.outcome)).length;
  const deniedCount = v.decisions.length - allowedCount;
  const decisionSegments: SegmentItem[] = [
    { key: "denied", label: "Denied / blocked", count: deniedCount, tone: "danger" },
    { key: "allowed", label: "Allowed", count: allowedCount, tone: "present" },
  ];

  return (
    <>
      <Section title="Check whether data can leave" count="Core decides">
        <form onSubmit={check} className="field-row" style={{ flexWrap: "wrap" }}>
          <select value={artifact} onChange={(e) => setArtifact(e.target.value)} aria-label="Artifact" style={{ flex: "1 1 260px" }}>
            <option value="">Choose an ingested source…</option>
            {docs.state.status === "ready" && docs.state.data.sources.map((s) => <option key={s.source_id} value={s.source_id}>{s.resource_type} · {s.source_id}</option>)}
          </select>
          <select value={boundary} onChange={(e) => setBoundary(e.target.value)} aria-label="Boundary">
            {Object.entries(BOUNDARY_LABELS).map(([k, label]) => <option key={k} value={k}>{label} ({k})</option>)}
          </select>
          <button type="submit" className="btn btn-s" disabled={!artifact || !!pending}>Evaluate</button>
        </form>
      </Section>
      <Section title="Decisions" count={v.decisions.length}>
        {v.decisions.length > 0 && <div style={{ marginBottom: 12 }}><SegmentedBar title="Recorded egress posture" meta={`${v.decisions.length} decision${v.decisions.length === 1 ? "" : "s"}`} items={decisionSegments} /></div>}
        {v.decisions.length ? (
          <table className="tb">
            <thead>
              <tr>
                <th style={{ width: "30%" }}>Artifact</th>
                <th style={{ width: "22%" }}>Boundary</th>
                <th style={{ width: "16%" }}>Outcome</th>
                <th>Reason</th>
              </tr>
            </thead>
            <tbody>{v.decisions.map((d, i) => (
              <tr key={i}>
                <td><span className="row-gap" style={{ gap: 6 }}><span className="mono t-mono-sm trunc">{d.artifact_id}</span><CopyButton text={d.artifact_id} label="Copy" /></span></td>
                <td className="t-sm"><span>{BOUNDARY_LABELS[d.boundary] ?? d.boundary}</span> <span className="mono t-mono-sm i3">{d.boundary}</span></td>
                <td><Status kind={glyphFor(d.outcome)} weight="strong">{d.outcome}</Status></td>
                <td className="t-sm i2"><span>{d.reason}</span></td>
              </tr>
            ))}</tbody>
          </table>
        ) : <p className="t-sm i3">No egress decisions recorded.</p>}
      </Section>
      <Section title="Classifications" count={v.classifications.length}>
        {v.classifications.length ? v.classifications.map((c) => <div key={c.artifact_id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) 160px minmax(0,1fr)" }}><span className="mono t-mono-sm trunc">{c.artifact_id}</span><span className="t-sm">{c.data_class}</span><span className="t-sm i3 trunc">{c.basis}</span></div>) : <p className="t-sm i3">No artifacts classified in this project.</p>}
      </Section>
      <Section title="Transform receipts" count={v.receipts.length}>
        {v.receipts.length ? v.receipts.map((r) => <div key={r.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) 120px minmax(0,1fr)" }}><span className="mono t-mono-sm trunc">{r.id}</span><Status kind={glyphFor(r.status)}>{r.status}</Status><span className="t-sm i3 trunc">{r.residual}</span></div>) : <p className="t-sm i3">No pseudonymization receipts.</p>}
      </Section>
      <PrivacyStudio />
      <Authority>Nothing leaves by default. A yes covers one item at one boundary, and is not a standing permission.</Authority>
    </>
  );
}

type PrivacyPresetSummary = {
  name: string;
  schema_version: number;
  posture: string;
  default_action: string;
  strict_no_leak: boolean;
  safety_sweep_mandatory: boolean;
  receipt?: string;
  sha256: string;
};
type PrivacyPresetsResult = { presets: PrivacyPresetSummary[] };
type DeidentifyResult = {
  redacted_text: string;
  policy_name: string;
  policy_sha256: string;
  base_direction: string;
  rtl_isolated: boolean;
  detected_spans: Array<{ label: string; start: number; end: number; text: string; source: string }>;
  suppressed_clinical_terms: number;
  sweep_added_spans: number;
  shifted_dates: Array<{ original: string; shifted: string; calendar: string; offset_days: number }>;
  receipt: { receipt_sha256: string; total_redacted_spans: number };
  receipt_verified: boolean;
};
type RiskResult = {
  record_count: number;
  k_min: number;
  class_count: number;
  singleton_count: number;
  l_diversity: Record<string, number>;
  t_closeness: Record<string, number>;
  prosecutor_risk: number;
  journalist_risk: number;
  marketer_risk: number;
};

function PrivacyStudio() {
  const { state: presetsState } = useCommand<PrivacyPresetsResult>("privacy_v4_presets", {});
  const [note, setNote] = useState("المريض أحمد حضر بتاريخ 2026-11-28، Diabetes mellitus، SSN 123-45-6789.");
  const [policyName, setPolicyName] = useState("saudi_pdpl");
  const [calendar, setCalendar] = useState("gregorian");
  const [shiftDays, setShiftDays] = useState("14");
  const [rtlIsolate, setRtlIsolate] = useState(true);
  const [clinicalProtect, setClinicalProtect] = useState(true);
  const [safetySweep, setSafetySweep] = useState(true);
  const [deidBusy, setDeidBusy] = useState(false);
  const [deidErr, setDeidErr] = useState<CmdError | null>(null);
  const [deid, setDeid] = useState<DeidentifyResult | null>(null);

  const [riskJson, setRiskJson] = useState(
    '[{"age":"30-39","zip":"100**","dx":"asthma"},{"age":"30-39","zip":"100**","dx":"diabetes"},{"age":"40-49","zip":"200**","dx":"asthma"}]',
  );
  const [quasiCsv, setQuasiCsv] = useState("age,zip");
  const [sensCsv, setSensCsv] = useState("dx");
  const [riskBusy, setRiskBusy] = useState(false);
  const [riskErr, setRiskErr] = useState<CmdError | null>(null);
  const [risk, setRisk] = useState<RiskResult | null>(null);

  const presets = presetsState.status === "ready" ? presetsState.data.presets : [];

  async function runDeidentify(e: FormEvent) {
    e.preventDefault();
    if (!note.trim() || deidBusy) return;
    setDeidBusy(true);
    setDeidErr(null);
    const parsedShift = shiftDays.trim() ? Number.parseInt(shiftDays.trim(), 10) : undefined;
    try {
      const res = await call<DeidentifyResult>("privacy_v4_deidentify", {
        text: note,
        policyName,
        calendar,
        dateShiftDays: Number.isFinite(parsedShift) ? parsedShift : null,
        rtlIsolate,
        clinicalProtect,
        safetySweep,
      });
      setDeid(res);
    } catch (err) {
      setDeidErr(err as CmdError);
    } finally {
      setDeidBusy(false);
    }
  }

  async function runRisk() {
    if (riskBusy) return;
    setRiskBusy(true);
    setRiskErr(null);
    try {
      const records = JSON.parse(riskJson) as Array<Record<string, unknown>>;
      const quasiIdentifiers = quasiCsv.split(",").map((s) => s.trim()).filter(Boolean);
      const sensitiveAttributes = sensCsv.split(",").map((s) => s.trim()).filter(Boolean);
      const res = await call<RiskResult>("privacy_v4_risk", {
        records,
        quasiIdentifiers,
        sensitiveAttributes,
      });
      setRisk(res);
    } catch (err) {
      setRiskErr((err as CmdError).kind ? (err as CmdError) : { kind: "invalid", message: "Invalid JSON table or parameters" });
    } finally {
      setRiskBusy(false);
    }
  }

  return (
    <Section title="De-identification & risk studio" count={presets.length ? `${presets.length} presets` : undefined}>
      <div className="stack" style={{ gap: 14 }}>
        <form onSubmit={runDeidentify} className="viz-frame stack" style={{ gap: 10 }}>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(170px, 1fr))", gap: 10 }}>
            <div>
              <label className="lbl" htmlFor="p4-policy">Policy preset</label>
              <select id="p4-policy" value={policyName} onChange={(e) => setPolicyName(e.target.value)} style={{ width: "100%" }}>
                {(presets.length ? presets : [{ name: "saudi_pdpl" }, { name: "hipaa_safe_harbor" }, { name: "gdpr_art9_health" }, { name: "strict_no_leak" }]).map((p) => (
                  <option key={p.name} value={p.name}>{p.name}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="lbl" htmlFor="p4-cal">Calendar</label>
              <select id="p4-cal" value={calendar} onChange={(e) => setCalendar(e.target.value)} style={{ width: "100%" }}>
                <option value="gregorian">gregorian</option>
                <option value="hijri">hijri (umm al-qura)</option>
                <option value="solar_hijri">solar_hijri</option>
                <option value="bikram_sambat">bikram_sambat</option>
              </select>
            </div>
            <div>
              <label className="lbl" htmlFor="p4-shift">Date shift (days)</label>
              <input id="p4-shift" value={shiftDays} maxLength={8} onChange={(e) => setShiftDays(e.target.value)} style={{ width: "100%" }} />
            </div>
          </div>
          <div>
            <label className="lbl" htmlFor="p4-note">Synthetic input text</label>
            <input id="p4-note" value={note} maxLength={4000} onChange={(e) => setNote(e.target.value)} style={{ width: "100%" }} />
          </div>
          <div className="row-gap" style={{ gap: 14, flexWrap: "wrap", alignItems: "center" }}>
            <label className="t-sm row-gap" style={{ gap: 6 }}><input type="checkbox" checked={rtlIsolate} onChange={(e) => setRtlIsolate(e.target.checked)} />RTL FSI/PDI isolation</label>
            <label className="t-sm row-gap" style={{ gap: 6 }}><input type="checkbox" checked={clinicalProtect} onChange={(e) => setClinicalProtect(e.target.checked)} />Clinical-protect filter</label>
            <label className="t-sm row-gap" style={{ gap: 6 }}><input type="checkbox" checked={safetySweep} onChange={(e) => setSafetySweep(e.target.checked)} />Residual safety sweep</label>
            <button type="submit" className="btn btn-s" disabled={deidBusy}>{deidBusy ? <><Glyph kind="running" />De-identifying</> : "De-identify text"}</button>
          </div>
          {deidErr && <ErrorState error={deidErr} />}
          {deid && (
            <div className="stack" style={{ gap: 8, paddingTop: 6, borderTop: "1px solid var(--line-subtle)" }}>
              <div className="row-gap" style={{ gap: 8, flexWrap: "wrap" }}>
                <Status kind={deid.receipt_verified ? "present" : "failed"} weight="strong">{deid.receipt_verified ? "Receipt verified" : "Receipt invalid"}</Status>
                <Status kind="available" weight="weak">dir={deid.base_direction}{deid.rtl_isolated ? " · isolated" : ""}</Status>
                <span className="mono t-mono-sm i2">clinical suppressed: {deid.suppressed_clinical_terms} · sweep catches: {deid.sweep_added_spans} · redacted: {deid.receipt.total_redacted_spans}</span>
              </div>
              <p className="t-sm i1" dir="auto" style={{ padding: "8px 10px", background: "var(--bg-surface)", borderRadius: "var(--r-2)", border: "1px solid var(--line-subtle)", overflowWrap: "anywhere" }}>{deid.redacted_text}</p>
              <KV keyWidth={128} rows={[
                ["policy_sha256", <span className="mono-break t-mono-sm">{deid.policy_sha256}</span>],
                ["receipt_sha256", <span className="mono-break t-mono-sm">{deid.receipt.receipt_sha256}</span>],
              ]} />
            </div>
          )}
        </form>

        <div className="viz-frame stack" style={{ gap: 10 }}>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(170px, 1fr))", gap: 10 }}>
            <div>
              <label className="lbl" htmlFor="p4-qi">Quasi-identifiers (CSV)</label>
              <input id="p4-qi" value={quasiCsv} maxLength={256} onChange={(e) => setQuasiCsv(e.target.value)} style={{ width: "100%" }} />
            </div>
            <div>
              <label className="lbl" htmlFor="p4-sens">Sensitive attributes (CSV)</label>
              <input id="p4-sens" value={sensCsv} maxLength={256} onChange={(e) => setSensCsv(e.target.value)} style={{ width: "100%" }} />
            </div>
          </div>
          <div>
            <label className="lbl" htmlFor="p4-tbl">Synthetic cohort table (JSON array)</label>
            <input id="p4-tbl" value={riskJson} maxLength={8000} onChange={(e) => setRiskJson(e.target.value)} style={{ width: "100%" }} />
          </div>
          <div className="row-gap" style={{ gap: 10 }}>
            <button type="button" className="btn btn-s" onClick={() => void runRisk()} disabled={riskBusy}>{riskBusy ? <><Glyph kind="running" />Computing</> : "Compute risk"}</button>
          </div>
          {riskErr && <ErrorState error={riskErr} />}
          {risk && (
            <Ledger items={[
              { k: "records", v: risk.record_count, mono: true },
              { k: "k_min", v: risk.k_min, mono: true },
              { k: "singletons", v: risk.singleton_count, mono: true },
              { k: "prosecutor_risk", v: risk.prosecutor_risk.toFixed(3), mono: true },
              { k: "marketer_risk", v: risk.marketer_risk.toFixed(3), mono: true },
            ]} />
          )}
        </div>
      </div>
    </Section>
  );
}

/* ───────────────────────────── Audit, Exports, Integrations ───────────────────────────── */
export function AuditTrail() {
  const { state, reload } = useGov();
  const [q, setQ] = useState("");
  const [expanded, setExpanded] = useState<string | null>(null);

  const rows = state.status === "ready" ? state.data.audit_rows : [];
  const needle = q.trim().toLowerCase();
  const filtered = needle
    ? rows.filter((r) => `${r.record_id} ${r.action} ${r.subject} ${r.status} ${r.detail}`.toLowerCase().includes(needle))
    : rows;

  return (
    <div className="pane">
      <PageHead title="Audit Trail" sub="Every disclosure, recorded by Core. There is no way to edit it here, though restoring an older backup can shorten it." />
      {state.status === "loading" && <Skeleton />}
      {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
      {state.status === "ready" && <>
        {rows.length > 0 && (
          <div className="row-gap" style={{ gap: 10, marginBottom: 14, justifyContent: "space-between", flexWrap: "wrap" }}>
            <SearchField value={q} onChange={setQ} placeholder="Filter disclosures by record, action, subject, or detail" ariaLabel="Filter audit trail" />
            <span className="mono t-mono-sm i3">{filtered.length} of {rows.length}</span>
          </div>
        )}
        {rows.length === 0 ? (
          <Empty icon="audit" title="No disclosures recorded">Core records a disclosure whenever data leaves the vault through an authorized path. None have happened in this workspace.</Empty>
        ) : filtered.length === 0 ? (
          <Empty icon="audit" title="No matching disclosures" action={<button type="button" className="btn btn-s" onClick={() => setQ("")}>Clear filter</button>}>
            No audit entries matched “{q}”.
          </Empty>
        ) : (
          <table className="tb">
            <thead>
              <tr>
                <th style={{ width: "20%" }}>Record</th>
                <th style={{ width: "17%" }}>Action</th>
                <th style={{ width: "20%" }}>Subject</th>
                <th style={{ width: "23%" }}>Status</th>
                <th>Detail</th>
              </tr>
            </thead>
            <tbody>{filtered.map((r) => {
              const isOpen = expanded === r.record_id;
              return (
                <Fragment key={r.record_id}>
                  <tr className={`clickable ${isOpen ? "sel" : ""}`} tabIndex={0} aria-label={`Inspect audit record ${r.record_id}`} aria-expanded={isOpen} onClick={() => setExpanded(isOpen ? null : r.record_id)} onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setExpanded(isOpen ? null : r.record_id); } }}>
                    <td><span className="mono t-mono-sm trunc">{r.record_id}</span></td>
                    <td className="t-sm">{r.action}</td>
                    <td className="t-sm i2"><span className="trunc">{r.subject}</span></td>
                    <td><Status kind={glyphFor(r.status)}>{r.status}</Status></td>
                    <td className="t-sm i3"><span className="trunc" title={r.detail}>{r.detail}</span></td>
                  </tr>
                  {isOpen && (
                    <tr>
                      <td colSpan={5} style={{ padding: "10px 14px", background: "var(--bg-raised)" }}>
                        <div className="stack" style={{ gap: 8 }}>
                          <div className="row-gap" style={{ gap: 8, flexWrap: "wrap", justifyContent: "space-between" }}>
                            <span className="mono t-mono-sm i2 wrap-any">{r.record_id}</span>
                            <span className="row-gap" style={{ gap: 6 }}>
                              <CopyButton text={r.record_id} label="Copy record ID" />
                              <CopyButton text={r.detail} label="Copy detail" />
                            </span>
                          </div>
                          <p className="t-sm i1" style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}>{r.detail}</p>
                        </div>
                      </td>
                    </tr>
                  )}
                </Fragment>
              );
            })}</tbody>
          </table>
        )}
        <p className="t-sm i3" style={{ marginTop: 12 }}>{state.data.audit_boundary}</p>
      </>}
    </div>
  );
}

type SubjectExport = {
  exports: Array<{ resource_type: string; field_losses: Array<{ fhir_path: string; loss_class: string; note: string }>; unsupported_fields_preserved: string[]; full_conformance_claimed: boolean }>;
  disclosure: { disclosure_id: string; purpose: string; artifact_refs: string[]; export_digest: string | null };
};

function SubjectExportForm({ onDone }: { onDone: () => void }) {
  const { navigate } = useApp();
  const patients = useCommand<{ subjects: Array<{ subject_ref: string; display_name: string }> }>("patients_list");
  const [subject, setSubject] = useState("");
  const [purpose, setPurpose] = useState("");
  const [result, setResult] = useState<SubjectExport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const subjects = patients.state.status === "ready" ? patients.state.data.subjects : [];
  useEffect(() => { if (!subject && subjects[0]) setSubject(subjects[0].subject_ref); }, [subjects, subject]);
  async function run(e: FormEvent) {
    e.preventDefault();
    setBusy(true); setError(null);
    try { setResult(await call<SubjectExport>("exports_subject", { subjectRef: subject, purpose })); onDone(); }
    catch (err) { setError((err as CmdError).message ?? String(err)); }
    finally { setBusy(false); }
  }

  const totalLosses = result ? result.exports.reduce((sum, x) => sum + x.field_losses.length, 0) : 0;
  const totalPreserved = result ? result.exports.reduce((sum, x) => sum + x.unsupported_fields_preserved.length, 0) : 0;
  const losslessResources = result ? result.exports.filter((x) => x.field_losses.length === 0).length : 0;

  return (
    <Section n="01" title="Export a subject" count="loss-aware · disclosed">
      <form onSubmit={run} className="stack" style={{ gap: 8, maxWidth: 640 }}>
        <label className="lbl" htmlFor="xs">Subject</label>
        <select id="xs" value={subject} onChange={(e) => setSubject(e.target.value)}>{subjects.map((s) => <option key={s.subject_ref} value={s.subject_ref}>{s.display_name} · {s.subject_ref}</option>)}</select>
        <label className="lbl" htmlFor="xp">Purpose (recorded in the disclosure)</label>
        <input id="xp" value={purpose} maxLength={200} onChange={(e) => setPurpose(e.target.value)} placeholder="For example: synthetic interchange review" />
        <div><button type="submit" className="btn btn-p btn-inline" disabled={!subject || !purpose.trim() || busy}>{busy ? <Glyph kind="running" /> : null}Export FHIR R4 (loss-aware)</button></div>
      </form>
      {error && <Notice kind="unavailable" title="Not exported.">{error}</Notice>}
      {result && <div style={{ marginTop: 14 }}>
        <Ledger items={[
          { k: "disclosure", v: <span className="row-gap" style={{ gap: 6 }}><span className="mono">{result.disclosure.disclosure_id}</span><CopyButton text={result.disclosure.disclosure_id} label="Copy" /></span> },
          { k: "resources", v: result.exports.length, mono: true },
          { k: "digest", v: result.disclosure.export_digest ? <span className="row-gap" style={{ gap: 6 }}><span className="mono">{result.disclosure.export_digest.slice(0, 20)}…</span><CopyButton text={result.disclosure.export_digest} label="Copy" /></span> : "—" },
        ]} />
        <div style={{ marginBottom: 12 }}>
          <SegmentedBar
            title="Export field fidelity"
            meta={`${result.exports.length} resource${result.exports.length === 1 ? "" : "s"}`}
            items={[
              { key: "clean", label: "Lossless resources", count: losslessResources, tone: "present" },
              { key: "preserved", label: "Unsupported preserved", count: totalPreserved, tone: "neutral" },
              { key: "losses", label: "Field losses", count: totalLosses, tone: "attention" },
            ]}
          />
        </div>
        <table className="tb">
          <thead><tr><th style={{ width: "22%" }}>Resource</th><th style={{ width: "14%" }}>Losses</th><th>Lost or unsupported fields</th></tr></thead>
          <tbody>{result.exports.map((x, i) => <tr key={i}><td className="t-sm">{x.resource_type}</td><td className="mono t-mono-sm">{x.field_losses.length}</td><td className="t-sm i2"><span>{[...x.field_losses.map((l) => `${l.fhir_path} (${l.loss_class})`), ...x.unsupported_fields_preserved].join(", ") || "none"}</span></td></tr>)}</tbody>
        </table>
        <div className="row-gap" style={{ justifyContent: "space-between", gap: 10, marginTop: 10, flexWrap: "wrap" }}>
          <p className="t-sm i3" style={{ margin: 0 }}>Full conformance is not claimed. This is a summary of the export: its contents are not displayed or written to disk. The disclosure is in the Audit Trail.</p>
          <button type="button" className="btn btn-s" onClick={() => navigate("Audit Trail")}>Open Audit Trail</button>
        </div>
      </div>}
    </Section>
  );
}

function formatSupportCell(v: unknown): string {
  if (v === null || v === undefined) return "—";
  if (typeof v === "boolean") return v ? "Yes" : "No";
  if (Array.isArray(v)) return v.map((x) => (typeof x === "object" ? Object.values(x as Record<string, unknown>).join(" ") : String(x))).join(", ") || "none";
  if (typeof v === "object") return Object.entries(v as Record<string, unknown>).map(([k, val]) => `${k.replace(/_/g, " ")}: ${String(val)}`).join(", ");
  return String(v);
}

export function Exports() {
  const { state, reload } = useGov();
  return (
    <div className="pane">
      <PageHead title="Exports" sub="Loss-aware interchange. What can leave, in which format, and what would be lost." />
      <SubjectExportForm onDone={reload} />
      {state.status === "loading" && <Skeleton />}
      {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
      {state.status === "ready" && <>
        <RowList rows={state.data.export_rows} />
        {state.data.fhir_support && <Section n="02" title="FHIR support matrix" count={`R${state.data.fhir_support.fhir_version}`}>
          <Ledger items={[{ k: "full_conformance", v: state.data.fhir_support.full_conformance_claimed ? "Claimed" : "Not claimed" }, { k: "synthetic_only", v: state.data.fhir_support.synthetic_only ? "Yes" : "No" }, { k: "resources", v: state.data.fhir_support.resources.length, mono: true }]} />
          <table className="tb">
            <thead><tr><th style={{ width: "22%" }}>Resource</th><th>Declared support</th></tr></thead>
            <tbody>{state.data.fhir_support.resources.map((r, i) => {
              const name = String(r.resource_type ?? r.resource ?? `resource ${i + 1}`);
              const entries = Object.entries(r).filter(([k]) => k !== "resource_type" && k !== "resource");
              return (
                <tr key={i}>
                  <td className="t-sm" style={{ fontWeight: 500 }}>{name}</td>
                  <td className="t-sm i2">
                    <div className="row-gap" style={{ gap: 10, flexWrap: "wrap" }}>
                      {entries.map(([k, val]) => (
                        <span key={k}><span className="i3">{k.replace(/_/g, " ")}:</span> {formatSupportCell(val)}</span>
                      ))}
                    </div>
                  </td>
                </tr>
              );
            })}</tbody>
          </table>
          {state.data.fhir_support.limitations.map((l) => <p key={l} className="t-sm i3" style={{ marginTop: 6 }}>{l}</p>)}
        </Section>}
        <p className="t-sm i3" style={{ marginTop: 12 }}>{state.data.export_boundary}</p>
      </>}
    </div>
  );
}

export function Integrations() {
  const { navigate } = useApp();
  const { state, reload } = useGov();
  return (
    <div className="pane">
      <PageHead title="Integrations" sub="Brokered integration posture. Nothing connects without the network broker and an external gate.">
        <div className="row-gap" style={{ gap: 8 }}>
          <button type="button" className="btn btn-g" onClick={() => navigate("Browse")}>Open Browse</button>
          <button type="button" className="btn btn-g" onClick={() => navigate("Privacy")}>Open Privacy</button>
        </div>
      </PageHead>
      {state.status === "loading" && <Skeleton />}
      {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
      {state.status === "ready" && (state.data.integration_rows.length ? (
        <><RowList rows={state.data.integration_rows} /><p className="t-sm i3" style={{ marginTop: 12 }}>{state.data.integrations_boundary}</p></>
      ) : (
        <Empty icon="integrations" title="No active integrations">MedScale keeps external integrations disabled by default. All outbound traffic must pass through explicit allowlists and egress checks.</Empty>
      ))}
    </div>
  );
}

/* ───────────────────────────── Settings ───────────────────────────── */
// Module scope: a component defined during render remounts on every state
// change, which drops keyboard focus to the document body.
function Seg<T extends string>({ value, options, onChange, label }: { value: T; options: T[]; onChange: (v: T) => void; label: string }) {
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className="seg"
      onKeyDown={(e) => {
        const idx = options.indexOf(value);
        let nextIdx = -1;
        if (e.key === "ArrowRight" || e.key === "ArrowDown") nextIdx = (idx + 1) % options.length;
        else if (e.key === "ArrowLeft" || e.key === "ArrowUp") nextIdx = (idx - 1 + options.length) % options.length;
        if (nextIdx >= 0) {
          e.preventDefault();
          onChange(options[nextIdx]!);
          e.currentTarget.querySelectorAll<HTMLButtonElement>('[role="radio"]')[nextIdx]?.focus();
        }
      }}
    >
      {options.map((o) => (
        <button key={o} type="button" role="radio" aria-checked={value === o} tabIndex={value === o ? 0 : -1} onClick={() => onChange(o)}>
          {o[0]!.toUpperCase() + o.slice(1)}
        </button>
      ))}
    </div>
  );
}
function SettingRow({ label, detail, children }: { label: string; detail: string; children: ReactNode }) {
  return (
    <div className="setting-row">
      <div><div style={{ fontSize: 13, fontWeight: 500 }}>{label}</div><div className="t-sm i3" style={{ marginTop: 2 }}>{detail}</div></div><div>{children}</div>
    </div>
  );
}

type RecoveryItem = { kind: "pre_recovery_copy" | "discarded_session"; id: string; bytes: number; modified_unix: number | null };

/** Spec 109 PR 3: what crash recovery kept for this vault, so it is never hidden. */
function recoveryLabel(item: RecoveryItem): string {
  return item.kind === "pre_recovery_copy" ? "Earlier saved copy" : "Discarded session";
}

function recoveryDate(item: RecoveryItem): string {
  return item.modified_unix ? new Date(item.modified_unix * 1000).toLocaleString() : "Date unknown";
}

function RecoveryKept({ onRestored }: { onRestored: () => void }) {
  const list = useCommand<RecoveryItem[]>("vault_recovery_items");
  const { pending, run } = useAction();
  const outcome = useOutcome();
  const [confirming, setConfirming] = useState<{ action: "restore" | "delete"; item: RecoveryItem } | null>(null);
  async function confirm() {
    if (!confirming) return;
    const { action, item } = confirming;
    if (action === "restore") {
      const r = await run<null>("recovery", "vault_recovery_restore", { id: item.id });
      setConfirming(null);
      if (outcome(r, "Copy restored · unlock to see it")) onRestored();
      else void list.reload();
    } else {
      const r = await run<unknown>("recovery", "vault_recovery_delete", { kind: item.kind, id: item.id });
      setConfirming(null);
      if (outcome(r, "Deleted")) {
        void list.reload();
        document.getElementById("main")?.focus();
      }
    }
  }
  return (
    <Section n="03" title="Crash recovery" count={list.state.status === "ready" ? list.state.data.length : "…"}>
      <p className="t-sm i3" style={{ marginBottom: 8 }}>What MedScale kept after it did not close normally. These stay in the vault folder until you delete them, except that only the three newest copies and stale sessions are kept. Copies are encrypted. A discarded session could not be verified and cannot be opened here.</p>
      {list.state.status === "error" && <ErrorState error={list.state.error} onRetry={list.reload} />}
      {list.state.status === "ready" && !list.state.data.length && <p className="t-sm i3">Nothing kept.</p>}
      {list.state.status === "ready" && list.state.data.map((item) => (
        <div key={item.kind + item.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) 170px 90px auto auto" }}>
          <span className="t-sm">{recoveryLabel(item)}</span>
          <span className="t-sm i3">{recoveryDate(item)}</span>
          <span className="mono t-mono-sm i3">{formatStorage(item.bytes)}</span>
          {item.kind === "pre_recovery_copy"
            ? <button type="button" className="btn btn-g" aria-label={`Restore ${recoveryLabel(item)}, ${recoveryDate(item)}`} disabled={!!pending} onClick={() => setConfirming({ action: "restore", item })}>Restore…</button>
            : <span />}
          <button type="button" className="btn btn-g" aria-label={`Delete ${recoveryLabel(item)}, ${recoveryDate(item)}`} disabled={!!pending} onClick={() => setConfirming({ action: "delete", item })}>Delete…</button>
        </div>
      ))}
      {confirming && (() => {
        const dialog = confirming.action === "restore" ? RESTORE_COPY_DIALOG : DELETE_RECOVERY_DIALOG;
        return (
          <ConfirmPanel
            title={dialog.title}
            confirmLabel={dialog.confirm}
            destructive={confirming.action === "delete"}
            busy={!!pending}
            onConfirm={confirm}
            onCancel={() => setConfirming(null)}
          >
            <b>{recoveryLabel(confirming.item)}, {recoveryDate(confirming.item)}, {formatStorage(confirming.item.bytes)}.</b> {dialog.body}
          </ConfirmPanel>
        );
      })()}
    </Section>
  );
}

/** Copies the sealed vault into a folder the person chooses. Locks first. */
function BackupRow({ onLocked }: { onLocked: () => void }) {
  const { ws } = useApp();
  const { pending, run } = useAction();
  const outcome = useOutcome();
  const toast = useToast();
  async function backUp() {
    let folder: string | null = null;
    try {
      const selected = await open({ multiple: false, directory: true, title: "Choose where to save the vault backup" });
      folder = typeof selected === "string" ? selected : null;
    } catch {
      toast({ kind: "blocked", title: "Cannot open folder chooser", detail: "Try again, or back up from the command line with medscale vault backup." });
      return;
    }
    if (!folder) return;
    const wasOpen = ws?.workspace?.kind === "encrypted";
    const r = await run<{ path: string; files: number; bytes: number }>("backup", "vault_backup", { folder });
    if (r.ok) toast({ kind: "completed", title: "Vault backed up and checked", detail: `${r.data.files} files, ${formatStorage(r.data.bytes)}, in ${r.data.path}` });
    else outcome(r, "");
    if (wasOpen) onLocked();
  }
  return (
    <SettingRow label="Back up vault" detail="Locks the vault, copies its encrypted files into a new folder you choose, and checks every copy. The backup opens with the same passphrase and recovery codes.">
      <button type="button" className="btn btn-s" onClick={() => void backUp()} disabled={!!pending}>{pending === "backup" ? "Backing up…" : "Lock and back up…"}</button>
    </SettingRow>
  );
}

/** B3: change the passphrase or replace every recovery code. Both need the current passphrase. */
function VaultKeys() {
  const { pending, run } = useAction();
  const outcome = useOutcome();
  const [mode, setMode] = useState<"passphrase" | "codes" | null>(null);
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [confirm, setConfirm] = useState("");
  const [showPass, setShowPass] = useState(false);
  const [codes, setCodes] = useState<string[]>([]);
  const mismatch = mode === "passphrase" && confirm.length > 0 && next !== confirm;
  function clearSecrets() { setCurrent(""); setNext(""); setConfirm(""); setShowPass(false); }
  function reset() { setMode(null); clearSecrets(); }
  async function submit(e: FormEvent) {
    e.preventDefault();
    if (mode === "passphrase") {
      if (next !== confirm) return;
      const r = await run<null>("rekey", "vault_change_passphrase", { current, newPassphrase: next });
      clearSecrets();
      if (outcome(r, "Passphrase changed · use the new one next time")) setMode(null);
    } else if (mode === "codes") {
      const r = await run<{ recovery_codes: string[] }>("rekey", "vault_replace_recovery_codes", { current });
      clearSecrets();
      if (r.ok) { setMode(null); setCodes(r.data.recovery_codes); } else outcome(r, "");
    }
  }
  if (codes.length) {
    return (
      <div className="viz-frame" style={{ margin: "12px 0" }}>
        <div className="viz-head">
          <span className="viz-title">New recovery codes · shown once</span>
          <CopyButton text={codes.join("\n")} label="Copy codes" />
        </div>
        <p className="t-sm i2" style={{ marginBottom: 10 }}>Every earlier code no longer opens this vault. Backups made before now still open with the codes they had.</p>
        <div className="codes" aria-label="New recovery codes">{codes.map((c) => <span key={c}>{c}</span>)}</div>
        <div style={{ marginTop: 12 }}><button type="button" className="btn btn-p btn-inline" onClick={() => setCodes([])}>I saved them</button></div>
      </div>
    );
  }
  const inputType = showPass ? "text" : "password";
  return (
    <>
      <SettingRow label="Passphrase and recovery codes" detail="Changing the passphrase or replacing the codes needs the current passphrase. Backups made earlier keep the passphrase and codes they had.">
        <span className="row-gap" style={{ gap: 8, flexWrap: "wrap" }}>
          <button type="button" className="btn btn-s" aria-pressed={mode === "passphrase"} onClick={() => { reset(); setMode("passphrase"); }}>Change passphrase…</button>
          <button type="button" className="btn btn-s" aria-pressed={mode === "codes"} onClick={() => { reset(); setMode("codes"); }}>Replace codes…</button>
        </span>
      </SettingRow>
      {mode && (
        <form onSubmit={(e) => void submit(e)} aria-label={mode === "passphrase" ? "Change passphrase" : "Replace recovery codes"} style={{ display: "grid", gap: 8, padding: "12px 0", borderBottom: "1px solid var(--line-subtle)", maxWidth: 520 }}>
          <label className="t-sm">Current passphrase<input style={{ display: "block", width: "100%", height: 36, marginTop: 4 }} type={inputType} autoComplete="current-password" value={current} onChange={(e) => setCurrent(e.target.value)} required /></label>
          {mode === "passphrase" && <>
            <label className="t-sm">New passphrase (8–256 characters · {next.length}/8 min)<input style={{ display: "block", width: "100%", height: 36, marginTop: 4 }} type={inputType} autoComplete="new-password" minLength={8} maxLength={256} value={next} onChange={(e) => setNext(e.target.value)} required /></label>
            <label className="t-sm">Confirm new passphrase<input style={{ display: "block", width: "100%", height: 36, marginTop: 4 }} type={inputType} autoComplete="new-password" value={confirm} onChange={(e) => setConfirm(e.target.value)} aria-invalid={mismatch} required /></label>
            {mismatch && <span className="t-sm" style={{ color: "var(--sem-danger)" }} role="alert">The new passphrases don&apos;t match.</span>}
          </>}
          <label className="row-gap t-sm i2" style={{ gap: 6, cursor: "pointer" }}>
            <input type="checkbox" checked={showPass} onChange={(e) => setShowPass(e.target.checked)} />
            Show passphrases
          </label>
          {mode === "codes" && <p className="t-sm i3">Every current recovery code stops working. The new codes are shown once.</p>}
          <span className="row-gap" style={{ gap: 8 }}>
            <button type="submit" className="btn btn-p btn-inline" disabled={!!pending || mismatch}>{pending === "rekey" ? "Saving…" : mode === "passphrase" ? "Change passphrase" : "Replace codes"}</button>
            <button type="button" className="btn btn-g" onClick={reset}>Cancel</button>
          </span>
        </form>
      )}
    </>
  );
}

export function Settings() {
  const { theme, setTheme, density, setDensity, ws, lock, vaultClosed } = useApp();
  const gov = useCommand<Gov>(ws?.open ? "governance_overview" : null);
  const [activeSec, setActiveSec] = useState("set-appearance");

  const sections: Array<[string, string]> = [
    ["set-appearance", "Appearance"],
    ["set-workspace", "Workspace"],
    ...(ws?.workspace?.kind === "encrypted" ? [["set-recovery", "Crash recovery"] as [string, string]] : []),
    ...(gov.state.status === "ready" ? [["set-privacy", "Privacy & network"] as [string, string], ["set-qualification", "Qualification"] as [string, string]] : []),
    ["set-keyboard", "Keyboard"],
  ];

  const sectionIds = sections.map(([id]) => id).join(",");
  useEffect(() => {
    const ids = sectionIds.split(",").filter(Boolean);
    const nodes = ids.map((id) => document.getElementById(id)).filter((el): el is HTMLElement => !!el);
    if (!nodes.length || typeof IntersectionObserver === "undefined") return;
    const io = new IntersectionObserver(
      (entries) => {
        const visible = entries.filter((e) => e.isIntersecting).sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top);
        if (visible[0]?.target.id) setActiveSec(visible[0].target.id);
      },
      { rootMargin: "-10% 0px -70% 0px", threshold: 0 },
    );
    nodes.forEach((n) => io.observe(n));
    return () => io.disconnect();
  }, [sectionIds]);

  return (
    <div className="pane">
      <PageHead title="Settings" sub="Read-only facts are shown as status. Anything you can change is a control." />
      <div className="settings-layout">
        <nav className="settings-nav" aria-label="Settings sections">
          {sections.map(([id, label]) => (
            <button
              key={id}
              type="button"
              className={`settings-link ${activeSec === id ? "on" : ""}`}
              aria-current={activeSec === id ? "location" : undefined}
              onClick={() => {
                setActiveSec(id);
                const target = document.getElementById(id);
                target?.focus({ preventScroll: true });
                target?.scrollIntoView({ block: "start" });
              }}
            >
              {label}
            </button>
          ))}
        </nav>
        <div className="settings-body">
          <div id="set-appearance" role="region" aria-label="Appearance" tabIndex={-1} onFocus={() => setActiveSec("set-appearance")}>
            <Section title="Appearance">
              <SettingRow label="Theme" detail="Saved on this device. Also available from the command palette."><Seg label="Theme" value={theme} options={["dark", "light"]} onChange={setTheme} /></SettingRow>
              <SettingRow label="Density" detail="Compact tightens rows, padding and gaps. Type size does not change."><Seg label="Density" value={density} options={["standard", "compact"]} onChange={setDensity} /></SettingRow>
              <SettingRow label="Reduce motion" detail="Follows the operating system. State changes stay immediate."><Status kind="available">Follows system</Status></SettingRow>
            </Section>
          </div>
          <div id="set-workspace" role="region" aria-label="Workspace" tabIndex={-1} onFocus={() => setActiveSec("set-workspace")}>
            <Section title="Workspace">
              <SettingRow label="Open workspace" detail={ws?.workspace ? `${ws.workspace.subjects} subjects · ${ws.workspace.sources} sources promoted by this desktop` : "No vault is open."}>
                {ws?.workspace ? <span className="mono" style={{ fontSize: 12 }}>{ws.workspace.vault_id}</span> : <Status kind="unavailable">None</Status>}
              </SettingRow>
              {ws?.workspace?.kind === "encrypted" && <SettingRow label="Clinical ingest" detail="FHIR imports, the patient list, projects, data, analytics and collaboration stay sealed in this vault. Real patient data is still not authorized: use synthetic or permitted test data only."><Status kind="limited">Synthetic or test data only</Status></SettingRow>}
              {ws?.workspace?.kind === "encrypted" && <VaultKeys />}
              {ws?.encrypted_vault_exists && <BackupRow onLocked={() => void vaultClosed()} />}
              <SettingRow label="Lock workspace" detail="Closes the vault and releases keys from memory.">
                <span className="row-gap" style={{ gap: 8 }}><button type="button" className="btn btn-s" onClick={() => void lock()} disabled={!ws?.open}>Lock now</button><span className="kbd">{MOD} Shift L</span></span>
              </SettingRow>
            </Section>
          </div>
          {ws?.workspace?.kind === "encrypted" && (
            <div id="set-recovery" role="region" aria-label="Crash recovery" tabIndex={-1} onFocus={() => setActiveSec("set-recovery")}>
              <RecoveryKept onRestored={() => void vaultClosed()} />
            </div>
          )}
          {gov.state.status === "ready" && <>
            <div id="set-privacy" role="region" aria-label="Privacy and network" tabIndex={-1} onFocus={() => setActiveSec("set-privacy")}>
              <Section title="Privacy & network"><RowList rows={gov.state.data.settings_rows.filter((r) => !/release|accessib/i.test(r.label))} /></Section>
            </div>
            <div id="set-qualification" role="region" aria-label="Qualification" tabIndex={-1} onFocus={() => setActiveSec("set-qualification")}>
              <Section title="Qualification" count="read-only"><RowList rows={gov.state.data.settings_rows.filter((r) => /release|accessib/i.test(r.label))} /><p className="t-sm i3" style={{ marginTop: 10 }}>{gov.state.data.missing_release_evidence}</p></Section>
            </div>
          </>}
          {gov.state.status === "error" && <ErrorState error={gov.state.error} onRetry={gov.reload} />}
          <div id="set-keyboard" role="region" aria-label="Keyboard shortcuts" tabIndex={-1} onFocus={() => setActiveSec("set-keyboard")}>
            <Section title="Keyboard">
              <KV keyWidth={160} rows={[
                ["Command palette", <span className="row-gap" style={{ gap: 8 }}><span className="kbd">{MOD} K</span><span className="i2">Search routes or run a workspace action</span></span>],
                ["Sidebar", <span className="row-gap" style={{ gap: 8 }}><span className="kbd">{MOD} B</span><span className="i2">Collapse or expand navigation rail</span></span>],
                ["Settings", <span className="row-gap" style={{ gap: 8 }}><span className="kbd">{MOD} ,</span><span className="i2">Open Settings</span></span>],
                ["Lock workspace", <span className="row-gap" style={{ gap: 8 }}><span className="kbd">{MOD} Shift L</span><span className="i2">Lock the open workspace</span></span>],
                ["Back", <span className="row-gap" style={{ gap: 8 }}><span className="kbd">Alt ←</span><span className="i2">Return to Patients from a patient detail view</span></span>],
                ["Page filter", <span className="row-gap" style={{ gap: 8 }}><span className="kbd">/</span><span className="i2">Focus the active page search input</span></span>],
                ["Rows", <span className="row-gap" style={{ gap: 8 }}><span className="kbd">Enter</span><span className="kbd">Space</span><span className="i2">Open or inspect the focused row</span></span>],
                ["Dismiss", <span className="row-gap" style={{ gap: 8 }}><span className="kbd">Esc</span><span className="i2">Close the palette, drawer, or dialog</span></span>],
              ]} />
            </Section>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ───────────────────────────── About ───────────────────────────── */
type AboutVm = { product: string; tagline: string; version: string; shell: string; core: string; local_only: boolean; network_default_deny: boolean; license: string; build_profile: string; target: string; release_ready: boolean };

export function About() {
  const { ws } = useApp();
  const { state, reload } = useCommand<AboutVm>("about_info");
  if (state.status === "loading") return <div className="pane"><Skeleton /></div>;
  if (state.status === "error") return <div className="pane"><ErrorState error={state.error} onRetry={reload} /></div>;
  const a = state.data;
  const isSynthetic = ws?.workspace?.kind !== "encrypted";
  return (
    <div className="pane">
      <div style={{ maxWidth: 980 }} className="stack">
        <h1 className="sr-only">About MedScale</h1>
        <div className="row-gap" style={{ alignItems: "flex-end", gap: 40, paddingBottom: 32, borderBottom: "1px solid var(--line)", marginBottom: 28, flexWrap: "wrap" }}>
          <Mark height={66} />
          <div className="stack" style={{ gap: 6, flex: 1, minWidth: 260 }}><Wordmark height={34} title={a.product} /><span style={{ fontSize: 15, color: "var(--ink-2)" }}>{a.tagline}</span></div>
          <div className="stack" style={{ alignItems: "flex-end", gap: 6 }}><span className="mono" style={{ fontSize: 20 }}>{a.version}</span><span style={{ fontSize: 11, letterSpacing: "0.28em", textTransform: "uppercase", color: "var(--ink-3)" }}>Evidence. Privacy. Scale.</span></div>
        </div>
        <div className="grid-2">
          <Section n="01" title="Build"><KV keyWidth={110} rows={[["version", <span className="mono t-mono-sm">{a.version}</span>], ["profile", <span className="mono t-mono-sm">{a.build_profile}</span>], ["target", <span className="mono t-mono-sm">{a.target}</span>], ["license", <span className="mono t-mono-sm">{a.license}</span>], ["build status", a.release_ready ? <Status kind="available">Release qualified</Status> : <Status kind="limited">Beta build · not clinically validated</Status>]]} /></Section>
          <Section n="02" title="Local runtime"><KV keyWidth={110} rows={[["shell", a.shell], ["core", a.core], ["network", <Status kind={a.network_default_deny ? "blocked" : "unknown"}>{a.network_default_deny ? "Default deny" : "Not verified"}</Status>], ["locality", <Status kind={a.local_only ? "available" : "unknown"}>{a.local_only ? "Local only" : "Unknown"}</Status>]]} /></Section>
          <Section n="03" title="Licenses"><KV keyWidth={110} rows={[["MedScale", <span className="mono t-mono-sm">Apache-2.0</span>], ["Inter", <span className="mono t-mono-sm">OFL-1.1</span>], ["JetBrains Mono", <span className="mono t-mono-sm">OFL-1.1</span>], ["third party", "Bundled third-party license notices"]]} /></Section>
          <Section n="04" title="Provenance"><KV keyWidth={110} rows={[["dependencies", "Audited open-source dependency policy"], ["signing", <Status kind="pending">Not claimed</Status>], ["clinical use", <Status kind="blocked">Not authorized · synthetic only</Status>]]} /></Section>
        </div>
        {isSynthetic ? (
          <Notice kind="synthetic" title="Synthetic fixtures.">Patients, evidence and model packs in the synthetic workspace come from the repository&apos;s synthetic fixtures. They are not real records.</Notice>
        ) : (
          <Notice kind="limited" title="Encrypted beta workspace.">Records in this workspace are encrypted locally on this device. Real patient data and clinical care remain unauthorized in this beta.</Notice>
        )}
        <ReportProblem about={a} />
        <SetAsideNotice />
      </div>
    </div>
  );
}

/** Spec 108 step 5: the set-aside sample copy is always disclosed here with
 * a confirmed, permanent delete. */
function SetAsideNotice() {
  const { ws, refreshWorkspace } = useApp();
  const { pending, run } = useAction();
  const toast = useToast();
  const [confirming, setConfirming] = useState(false);
  if (!ws?.set_aside_exists) return null;
  async function remove() {
    const r = await run<unknown>("delete-set-aside", "workspace_delete_set_aside_synthetic");
    setConfirming(false);
    if (r.ok) toast({ kind: "completed", title: "Set-aside sample deleted" });
    else toast({ kind: "blocked", title: "Not deleted", detail: r.error.message });
    await refreshWorkspace();
  }
  return (
    <Section n="06" title="Set-aside sample">
      <p className="t-sm i2" style={{ margin: "0 0 10px", whiteSpace: "normal" }}>{SET_ASIDE_EXISTS_NOTICE}</p>
      <button type="button" className="btn btn-g" aria-expanded={confirming} onClick={() => setConfirming(true)} disabled={!!pending}>Delete set-aside sample…</button>
      {confirming && (
        <ConfirmPanel title={DELETE_SET_ASIDE_DIALOG.title} confirmLabel={DELETE_SET_ASIDE_DIALOG.confirm} destructive busy={!!pending}
          onConfirm={remove} onCancel={() => setConfirming(false)}>{DELETE_SET_ASIDE_DIALOG.body}</ConfirmPanel>
      )}
    </Section>
  );
}

/** Spec 107 D2: copy-only defect reporting. MedScale does not open the
 * browser itself from About; Help & Feedback offers explicit browser links. */
function ReportProblem({ about }: { about: AboutVm }) {
  const [copied, setCopied] = useState<"link" | "diagnostics" | "failed" | null>(null);
  const diagnostics = supportDiagnostics(about);
  async function copy(kind: "link" | "diagnostics", text: string) {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(kind);
    } catch {
      setCopied("failed");
    }
  }
  return (
    <Section n="05" title="Report a problem">
      <p className="t-sm i2" style={{ margin: "0 0 10px" }}>Copy this address into your web browser to open a bug report, or use Help &amp; Feedback to launch a pre-filled template.</p>
      <div className="row-gap" style={{ gap: 8, marginBottom: 12, flexWrap: "wrap" }}>
        <code className="mono t-mono-sm" style={{ userSelect: "all", overflowWrap: "anywhere" }} aria-label="Support address">{SUPPORT_ISSUES_URL}</code>
        <button type="button" className="btn btn-g" onClick={() => copy("link", SUPPORT_ISSUES_URL)}>Copy link</button>
      </div>
      <div className="lbl" id="support-diag-label">Diagnostics to paste into the report</div>
      <pre className="pre" aria-labelledby="support-diag-label" style={{ margin: "6px 0 8px" }}>{diagnostics}</pre>
      <div className="row-gap" style={{ gap: 8, marginBottom: 10 }}>
        <button type="button" className="btn btn-g" onClick={() => copy("diagnostics", diagnostics)}>Copy diagnostics</button>
        <span className="t-sm i3" role="status" aria-live="polite">
          {copied === "link" ? "Link copied." : copied === "diagnostics" ? "Diagnostics copied." : copied === "failed" ? "Copy failed. Select the text above and copy it manually." : ""}
        </span>
      </div>
      <Notice kind="blocked" title="Synthetic examples only.">{SUPPORT_DATA_RULE}</Notice>
    </Section>
  );
}
