import { useMemo, useState, type ReactNode } from "react";
import { useApp } from "../lib/app";
import { useCommand } from "../lib/ipc";
import { Authority, Empty, ErrorState, Glyph, KV, Notice, Section, Skeleton, Stamp, Status, type GlyphKind } from "../components/ui";

type Slot = { concept_key: string; status: string };
type SubjectRow = {
  subject_ref: string; display_name: string; birth_date: string; condition_summary: string;
  coverage_summary: string; coverage_state: string; coverage_slots: Slot[] | null;
  latest_event: string | null; event_count: number; source_count: number;
};
type Field = { field_key: string; value: unknown; status: string; evidence_refs: string[]; unit?: string | null };
type CoverageSlot = { concept_key: string; status: string; values: Field[]; notes: string[]; conflict?: { claim_key: string; members: Field[]; resolution: string } | null };
type Detail = {
  subject_ref: string; display_name: string; birth_date: string; condition_summary: string; coverage_summary: string; coverage_state: string;
  timeline: Array<{ date: string; title: string; detail: string; source: string; status: string }>;
  labs: Array<{ label: string; value: string; unit: string; status: string; source: string }>;
  medications_state: string; documents_state: string; care_plan_state: string;
  sources: Array<{ source_id: string; resource_type: string; fixture: string; claim_kind: string }>;
  contracts: {
    brief: { sections: { identity: Field[]; vitals: Field[]; conditions: Field[]; coverage_summary: Record<string, number> }; built_rules_version: { extractors: string[]; order_rules: string; brief_schema: string } };
    coverage: { slots: CoverageSlot[] };
  };
};

export const COVERAGE: Record<string, { glyph: GlyphKind; label: string }> = {
  present: { glyph: "present", label: "Present" },
  absent: { glyph: "absent", label: "Absent" },
  unknown: { glyph: "unknown", label: "Unknown" },
  conflict: { glyph: "conflict", label: "Conflict" },
  incomparable_units: { glyph: "incomparable", label: "Incomparable units" },
  unhealthy_evidence: { glyph: "unhealthy", label: "Unhealthy evidence" },
  unsupported_resource_type: { glyph: "unsupported", label: "Unsupported resource" },
};
const cov = (s: string) => COVERAGE[s] ?? { glyph: "unknown" as GlyphKind, label: s };

function initials(name: string) {
  const parts = name.replace(/[^A-Za-z ]/g, " ").trim().split(/\s+/).filter(Boolean);
  return (parts.length ? parts.slice(0, 2).map((p) => p[0]) : ["?"]).join("").toUpperCase();
}

function value(f: Field): string {
  if (f.value === null || f.value === undefined) return "—";
  return typeof f.value === "string" ? f.value : typeof f.value === "number" ? String(f.value) : JSON.stringify(f.value);
}

export function CoverageStrip({ slots }: { slots: Slot[] | null }) {
  if (!slots) return <Status kind="unknown" weight="weak">Not read</Status>;
  const present = slots.filter((s) => s.status === "present").length;
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }} title={slots.map((s) => `${s.concept_key}: ${cov(s.status).label}`).join("\n")}>
      <span className="cov" aria-hidden="true">{slots.map((s) => <Glyph key={s.concept_key} kind={cov(s.status).glyph} />)}</span>
      <span className="mono t-mono-sm i3">{present}/{slots.length}</span>
    </span>
  );
}

function needsReview(row: { coverage_slots: Slot[] | null }) {
  return (row.coverage_slots ?? []).some((s) => s.status !== "present");
}

export function Patients() {
  const { navigate } = useApp();
  const { state, reload } = useCommand<{ subjects: SubjectRow[]; scope: string }>("patients_list");
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<string | null>(null);

  const rows = useMemo(() => {
    if (state.status !== "ready") return [];
    const q = query.trim().toLowerCase();
    return state.data.subjects.filter((r) => !q || r.display_name.toLowerCase().includes(q) || r.subject_ref.toLowerCase().includes(q));
  }, [state, query]);
  const current = rows.find((r) => r.subject_ref === selected) ?? rows[0] ?? null;

  return (
    <>
      <div className="pane">
        <div className="page-head">
          <div className="grow">
            <h1 className="t-h1">Patients</h1>
            <p className="ph-sub">{state.status === "ready" ? `${state.data.subjects.length} subject${state.data.subjects.length === 1 ? "" : "s"} · ${state.data.scope}` : "Reading subjects from the vault…"}</p>
          </div>
        </div>
        {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
        {state.status !== "error" && (
          <>
            <div className="field-row" style={{ marginBottom: 12 }}>
              <label className="field" style={{ flex: "1 1 280px", maxWidth: 460 }}>
                <svg width="16" height="16" viewBox="0 0 20 20" aria-hidden="true"><path d="M8.5 2.5a6 6 0 1 1 0 12 6 6 0 0 1 0-12Zm0 1.7a4.3 4.3 0 1 0 0 8.6 4.3 4.3 0 0 0 0-8.6Z M12.7 13.9 13.9 12.7 18 16.8 16.8 18Z" fill="currentColor" fillRule="evenodd" /></svg>
                <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search by name or identifier" aria-label="Search patients" maxLength={128} />
                {query && <span className="mono t-mono-sm i3">{rows.length} match{rows.length === 1 ? "" : "es"}</span>}
              </label>
            </div>
            <table className="tb" aria-label="Patients">
              <thead><tr>
                <th style={{ width: "28%" }} className="sorted">Patient</th>
                <th style={{ width: "16%" }} className="col-p2">Known condition</th>
                <th style={{ width: "20%" }}>Coverage</th>
                <th style={{ width: "7%" }} className="col-p3 num" title="Sources">Src</th>
                <th style={{ width: "15%" }}>Review</th>
                <th className="num" title="Latest event">Latest</th>
              </tr></thead>
              <tbody>
                {state.status === "loading" && Array.from({ length: 2 }, (_, i) => (
                  <tr key={i} className="r2"><td colSpan={6}><span className="sk t" style={{ width: "40%" }} /></td></tr>
                ))}
                {rows.map((r) => (
                  <tr key={r.subject_ref} className={`r2 clickable ${current?.subject_ref === r.subject_ref ? "sel" : ""}`} tabIndex={0} aria-selected={current?.subject_ref === r.subject_ref}
                    onClick={() => setSelected(r.subject_ref)} onDoubleClick={() => navigate("Patients", r.subject_ref)}
                    onKeyDown={(e) => { if (e.key === "Enter") navigate("Patients", r.subject_ref); if (e.key === " ") { e.preventDefault(); setSelected(r.subject_ref); } }}>
                    <td><div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                      <span className="mg syn" title="Synthetic subject"><span>{initials(r.display_name)}</span></span>
                      <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="pname trunc">{r.display_name}</span><span className="mono t-mono-sm i3 trunc">{r.subject_ref}</span></div>
                    </div></td>
                    <td className="col-p2 t-sm i2"><span className="trunc">{r.condition_summary}</span></td>
                    <td><CoverageStrip slots={r.coverage_slots} /></td>
                    <td className="col-p3 num mono t-mono-sm i2">{r.source_count}</td>
                    <td>{needsReview(r) ? <Status kind="needs-review">Needs review</Status> : <Status kind="present" weight="weak">Covered</Status>}</td>
                    <td className="num mono t-mono-sm i3">{r.latest_event ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            {state.status === "ready" && (
              <div className="tfoot"><span>{rows.length} of {state.data.subjects.length} shown</span><span style={{ flex: 1 }} /><span>Enter opens · Space previews</span></div>
            )}
            {state.status === "ready" && !state.data.subjects.length && (
              <Empty icon="patients" title="No subjects in this vault">Subjects appear after records are ingested and promoted through Core. The synthetic workspace does this on first open. Core does not admit clinical ingest into an encrypted vault until private-data authorization is granted.</Empty>
            )}
          </>
        )}
      </div>
      {current && <PatientInspector subject={current.subject_ref} onOpen={() => navigate("Patients", current.subject_ref)} />}
    </>
  );
}

function PatientInspector({ subject, onOpen }: { subject: string; onOpen: () => void }) {
  const { state } = useCommand<Detail>("patient_detail", { subjectRef: subject });
  return (
    <aside className="inspector collapsible" aria-label="Selected patient">
      <div className="ins-pad">
        {state.status === "loading" && <Skeleton rows={6} />}
        {state.status === "error" && <ErrorState error={state.error} />}
        {state.status === "ready" && (() => {
          const d = state.data;
          return <>
            <div style={{ display: "flex", gap: 12, alignItems: "flex-start" }}>
              <span className="mg syn lg"><span>{initials(d.display_name)}</span></span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div className="t-h2" style={{ fontSize: 16 }}>{d.display_name}</div>
                <div className="mono t-mono-sm i3 trunc">{d.subject_ref}</div>
              </div>
              <button type="button" className="btn btn-p" onClick={onOpen}>Open</button>
            </div>
            <div className="row-gap" style={{ gap: 18 }}><Status kind="synthetic">Synthetic</Status><Status kind={d.coverage_state.toLowerCase().includes("review") ? "needs-review" : glyphState(d.coverage_state)}>{d.coverage_state}</Status></div>
            <Section title="Brief" count="SubjectBriefV1">
              <KV keyWidth={150} rows={[
                ...d.contracts.brief.sections.identity.map((f): [string, ReactNode] => [f.field_key, <FieldValue f={f} />]),
                ...d.contracts.brief.sections.conditions.map((f): [string, ReactNode] => [f.field_key, <FieldValue f={f} />]),
              ]} />
            </Section>
            <Section title="Coverage" count={`${d.contracts.coverage.slots.length} slots`}>
              {d.contracts.coverage.slots.map((s) => (
                <div key={s.concept_key} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", minHeight: 30, borderBottom: "1px solid var(--line-subtle)", gap: 12 }}>
                  <span className="lk trunc">{s.concept_key}</span>
                  <Status kind={cov(s.status).glyph} weight={s.status === "present" ? "default" : "weak"}>{cov(s.status).label}</Status>
                </div>
              ))}
              <p className="mono" style={{ fontSize: 10.5, color: "var(--ink-3)", marginTop: 8 }}>{d.coverage_summary}</p>
            </Section>
            <Section title="Sources" count={d.sources.length}>
              <div className="stack" style={{ gap: 8 }}>{d.sources.map((s) => <span key={s.source_id + s.fixture} className="stamp"><span className="g g-synthetic" />{s.resource_type} · {s.source_id}</span>)}</div>
            </Section>
            <Authority>Read-only presentation. Missing or conflicting evidence is never promoted into an affirmative fact.</Authority>
          </>;
        })()}
      </div>
    </aside>
  );
}

function glyphState(s: string): GlyphKind {
  const v = s.toLowerCase();
  if (v.includes("present") || v.includes("trusted")) return "present";
  if (v.includes("conflict")) return "conflict";
  return "unknown";
}

function FieldValue({ f }: { f: Field }) {
  const c = cov(f.status);
  return (
    <span style={{ display: "inline-flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
      <span className={typeof f.value === "string" && /^\d{4}-\d{2}/.test(f.value) ? "mono" : ""}>{value(f)}</span>
      {f.unit && <span className="mono t-mono-sm i3">{f.unit}</span>}
      {f.status !== "present" && <Status kind={c.glyph} weight="weak">{c.label}</Status>}
    </span>
  );
}

type Tab = "Overview" | "Timeline" | "Labs" | "Coverage" | "Sources";

export function PatientDetail({ subject }: { subject: string }) {
  const { navigate } = useApp();
  const { state, reload } = useCommand<Detail>("patient_detail", { subjectRef: subject });
  const [tab, setTab] = useState<Tab>("Overview");

  if (state.status === "loading") return <div className="pane"><Skeleton rows={8} /></div>;
  if (state.status === "error") return <div className="pane"><ErrorState error={state.error} onRetry={reload} /></div>;
  const d = state.data;
  const b = d.contracts.brief;
  const slots = d.contracts.coverage.slots;
  const tally = b.sections.coverage_summary;
  const notShown = slots.filter((s) => s.status !== "present");

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, minWidth: 0 }}>
      <div style={{ padding: "18px var(--pad-page-x) 0", flex: "none" }}>
        <div className="row-gap" style={{ gap: 14, rowGap: 12 }}>
          <button type="button" className="btn btn-g" onClick={() => navigate("Patients")} aria-label="Back to Patients">← Patients</button>
          <span className="mg syn lg"><span>{initials(d.display_name)}</span></span>
          <div className="stack" style={{ gap: 2, minWidth: 0 }}>
            <h1 className="t-h1" style={{ fontSize: 20, lineHeight: "26px" }}>{d.display_name}</h1>
            <div className="row-gap" style={{ gap: 16 }}><Status kind="synthetic">Synthetic</Status>{notShown.length ? <Status kind="needs-review">Needs review</Status> : <Status kind="present">Covered</Status>}</div>
          </div>
          <div className="ledger hide-md" style={{ marginLeft: 20 }}>
            <div><span className="lk">subject_ref</span><span className="lv mono">{d.subject_ref}</span></div>
            <div><span className="lk">patient.birthDate</span><span className="lv mono">{d.birth_date}</span></div>
            <div><span className="lk">condition.code</span><span className="lv">{d.condition_summary}</span></div>
          </div>
        </div>
        <div className="auth" style={{ marginTop: 14, borderBottom: 0 }}><Glyph kind="proposal" /><span className="t-label">Authority</span><span className="trunc">Read-only presentation. Evidence and provenance stay visible; relevance never becomes clinical authority and nothing is committed silently.</span></div>
        <div className="tabs" role="tablist" aria-label="Patient sections">
          {(["Overview", "Timeline", "Labs", "Coverage", "Sources"] as Tab[]).map((t) => (
            <button key={t} type="button" role="tab" aria-selected={tab === t} className={`tab ${tab === t ? "on" : ""}`} onClick={() => setTab(t)}>
              {t}
              <span className="mono">{t === "Timeline" ? d.timeline.length : t === "Labs" ? d.labs.length : t === "Coverage" ? slots.length : t === "Sources" ? d.sources.length : ""}</span>
            </button>
          ))}
          <span className="tab na" title={d.medications_state}>Medications <Glyph kind="unsupported" /></span>
          <span className="tab na" title={d.documents_state}>Documents <Glyph kind="unavailable" /></span>
          <span className="tab na" title={d.care_plan_state}>Care Plan <Glyph kind="unavailable" /></span>
        </div>
      </div>

      <div className="split">
        <div className="pane" style={{ paddingTop: 20 }}>
          {(tab === "Overview") && <>
            <Section n="01" title="Brief" count="SubjectBriefV1">
              <BriefTable groups={[["Identity", b.sections.identity], ["Vitals and labs", b.sections.vitals], ["Conditions", b.sections.conditions]]} />
            </Section>
            <Section n="02" title="Timeline" count={`${d.timeline.length} events`} actions={<button type="button" className="btn btn-g" style={{ height: 24 }} onClick={() => setTab("Timeline")}>Open timeline</button>}>
              <Timeline rows={d.timeline.slice(0, 5)} />
            </Section>
            <Section n="03" title="Not shown" count={notShown.length + 1}>
              <div className="stack" style={{ gap: 8 }}>
                <Notice kind="unsupported" title="Medications.">{d.medications_state}</Notice>
                <Notice kind="unavailable" title="Documents.">{d.documents_state}</Notice>
                {notShown.filter((s) => s.status !== "unsupported_resource_type").map((s) => (
                  <Notice key={s.concept_key} kind={cov(s.status).glyph} title={`${s.concept_key}: ${cov(s.status).label}.`}>{s.notes.join(" ") || (s.conflict ? `${s.conflict.members.length} conflicting values, ${s.conflict.resolution.toLowerCase()}.` : "Shown as Core reports it; nothing is inferred.")}</Notice>
                ))}
              </div>
            </Section>
          </>}
          {tab === "Timeline" && <Section title="Timeline" count={b.built_rules_version.order_rules}><Timeline rows={d.timeline} /></Section>}
          {tab === "Labs" && <Section title="Labs and vitals" count={d.labs.length}>
            {d.labs.length ? <table className="tb"><thead><tr><th style={{ width: "34%" }}>Observation</th><th style={{ width: "22%" }}>Value</th><th style={{ width: "18%" }}>Status</th><th>Source</th></tr></thead>
              <tbody>{d.labs.map((l, i) => <tr key={i}><td className="t-sm">{l.label}</td><td><span className="mono">{l.value}</span> <span className="mono t-mono-sm i3">{l.unit}</span></td><td><Status kind={glyphState(l.status) === "unknown" ? cov(l.status.toLowerCase()).glyph : glyphState(l.status)}>{l.status}</Status></td><td><Stamp k="src" v={l.source} /></td></tr>)}</tbody></table>
              : <Empty icon="data" title="No supported observations">No observation passed the trusted extractor for this subject.</Empty>}
          </Section>}
          {tab === "Coverage" && <Section title="Coverage slots" count="SubjectCoverageV1">
            <table className="tb"><thead><tr><th style={{ width: "32%" }}>Concept</th><th style={{ width: "22%" }}>Status</th><th>Values · notes</th></tr></thead>
              <tbody>{slots.map((s) => <tr key={s.concept_key}><td><span className="lk" style={{ fontSize: 11.5 }}>{s.concept_key}</span></td><td><Status kind={cov(s.status).glyph}>{cov(s.status).label}</Status></td>
                <td className="t-sm i2"><span className="trunc">{s.values.map(value).join(" · ") || s.notes.join(" ") || (s.conflict ? `conflict: ${s.conflict.members.map(value).join(" vs ")}` : "—")}</span></td></tr>)}</tbody></table>
          </Section>}
          {tab === "Sources" && <Section title="Sources" count={d.sources.length}>
            <table className="tb"><thead><tr><th style={{ width: "34%" }}>Source</th><th style={{ width: "18%" }}>Resource</th><th style={{ width: "18%" }}>Claim kind</th><th>Fixture</th></tr></thead>
              <tbody>{d.sources.map((s, i) => <tr key={i}><td><span className="mono t-mono-sm trunc">{s.source_id}</span></td><td className="t-sm">{s.resource_type}</td><td className="t-sm i2">{s.claim_kind}</td><td><Status kind="synthetic" weight="weak"><span className="mono">{s.fixture}</span></Status></td></tr>)}</tbody></table>
          </Section>}
        </div>
        <aside className="inspector" aria-label="Coverage and provenance">
          <div className="ins-pad">
            <Section title="Coverage" count="SubjectCoverageV1">
              <div style={{ display: "flex", alignItems: "center", gap: 10, margin: "4px 0 12px" }}>
                <CoverageStrip slots={slots.map((s) => ({ concept_key: s.concept_key, status: s.status }))} />
                <span style={{ fontSize: 13, fontWeight: 600 }}>{slots.filter((s) => s.status === "present").length} of {slots.length} slots present</span>
              </div>
              {Object.entries(COVERAGE).map(([key, c]) => (
                <div key={key} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", height: 26 }}>
                  <Status kind={c.glyph} weight={(tally[key] ?? 0) ? "strong" : "weak"}>{c.label}</Status>
                  <span className={`mono t-mono-sm ${(tally[key] ?? 0) ? "" : "i3"}`}>{tally[key] ?? 0}</span>
                </div>
              ))}
            </Section>
            <Section title="Presentation rules">
              <KV keyWidth={84} rows={[["extractors", <span className="mono t-mono-sm">{b.built_rules_version.extractors.join(" · ")}</span>], ["order", <span className="mono-break">{b.built_rules_version.order_rules}</span>], ["schema", <span className="mono t-mono-sm">{b.built_rules_version.brief_schema}</span>]]} />
            </Section>
          </div>
        </aside>
      </div>
    </div>
  );
}

function BriefTable({ groups }: { groups: Array<[string, Field[]]> }) {
  return (
    <table className="tb">
      <thead><tr><th style={{ width: "30%" }}>Field</th><th style={{ width: "30%" }}>Value</th><th style={{ width: "16%" }}>Status</th><th>Evidence</th></tr></thead>
      <tbody>
        {groups.map(([group, fields]) => [
          <tr key={`${group}-h`}><td colSpan={4} style={{ height: 30, paddingTop: 10, borderBottom: 0 }}><span className="t-label">{group}</span></td></tr>,
          ...(fields.length ? fields.map((f, i) => (
            <tr key={`${group}-${i}`}>
              <td><span className="lk" style={{ fontSize: 11.5 }}>{f.field_key}</span></td>
              <td><span className={typeof f.value === "number" || (typeof f.value === "string" && /^\d{4}-/.test(f.value)) ? "mono" : ""}>{value(f)}</span> {f.unit && <span className="mono t-mono-sm i3">{f.unit}</span>}</td>
              <td><Status kind={cov(f.status).glyph}>{cov(f.status).label}</Status></td>
              <td><Stamp k="ref" v={f.evidence_refs[0] ?? "—"} /></td>
            </tr>
          )) : [<tr key={`${group}-e`}><td colSpan={4} className="t-sm i3">No supported field in this section.</td></tr>]),
        ])}
      </tbody>
    </table>
  );
}

function Timeline({ rows }: { rows: Detail["timeline"] }) {
  if (!rows.length) return <p className="t-sm i3">No promoted events.</p>;
  return (
    <div>
      {rows.map((e, i) => (
        <div key={i} style={{ display: "grid", gridTemplateColumns: "144px 20px 96px minmax(0,1fr) minmax(0, 200px)", alignItems: "center", columnGap: 12, minHeight: "var(--row)" }}>
          <span className="mono t-mono-sm i2" style={{ whiteSpace: "nowrap" }}>{e.date}</span>
          <span style={{ position: "relative", alignSelf: "stretch", display: "flex", alignItems: "center", justifyContent: "center" }}>
            <span style={{ position: "absolute", top: i === 0 ? "50%" : 0, bottom: i === rows.length - 1 ? "50%" : 0, left: "50%", width: 1, background: "var(--line-strong)" }} />
            <span className="g g-present" style={{ position: "relative", boxShadow: "0 0 0 3px var(--bg-surface)" }} />
          </span>
          <span className="t-sm i3">{e.title}</span>
          <span className="trunc" style={{ fontSize: 13 }} title={e.detail}>{summarizeDetail(e.detail)}</span>
          <span className="hide-md" style={{ minWidth: 0, overflow: "hidden", justifySelf: "end" }}><Stamp k={/^sourceb/i.test(e.source) ? undefined : "src"} v={e.source} /></span>
        </div>
      ))}
    </div>
  );
}

/** Timeline details arrive as the trusted projection's JSON; show its fields as a readable line, keep the raw JSON in the tooltip. */
function summarizeDetail(detail: string): string {
  try {
    const v: unknown = JSON.parse(detail);
    if (!v || typeof v !== "object" || Array.isArray(v)) return detail;
    return Object.entries(v as Record<string, unknown>)
      .filter(([k, x]) => k !== "resourceType" && x !== null && x !== undefined && x !== "")
      .map(([k, x]) => `${k} ${typeof x === "object" ? Object.values(x as object).join(" ") : String(x)}`)
      .join(" · ");
  } catch {
    return detail;
  }
}
