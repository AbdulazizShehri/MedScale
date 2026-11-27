import { useMemo, useState, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";
import { useApp } from "../lib/app";
import { useCommand } from "../lib/ipc";
import {
  Authority, CopyButton, Disclosure, Empty, ErrorState, Glyph, Icon, KV, Notice, PageHead,
  ResizableSplit, SearchField, Section, SegmentedBar, Skeleton, Stamp, Status, humanKey,
  type GlyphKind, type SegmentItem,
} from "../components/ui";

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

export const COVERAGE: Record<string, { glyph: GlyphKind; label: string; tone: SegmentItem["tone"] }> = {
  present: { glyph: "present", label: "Present", tone: "present" },
  absent: { glyph: "absent", label: "Absent", tone: "neutral" },
  unknown: { glyph: "unknown", label: "Unknown", tone: "muted" },
  conflict: { glyph: "conflict", label: "Conflict", tone: "attention" },
  incomparable_units: { glyph: "incomparable", label: "Incomparable units", tone: "hatched" },
  unhealthy_evidence: { glyph: "unhealthy", label: "Unhealthy evidence", tone: "danger" },
  unsupported_resource_type: { glyph: "unsupported", label: "Unsupported resource", tone: "muted" },
};
const cov = (s: string) => COVERAGE[s] ?? { glyph: "unknown" as GlyphKind, label: s, tone: "muted" as const };

const FHIR_LABELS: Record<string, string> = {
  "patient.birthDate": "Birth date",
  "patient.name.family": "Family name",
  "patient.name.given": "Given name",
  "patient.gender": "Gender",
  "condition.clinicalStatus": "Condition status",
  "condition.code": "Condition code",
  "condition.onsetDateTime": "Condition onset",
  "observation.code": "Observation",
  "observation.effective": "Recorded date",
  "observation.valueQuantity": "Measured value",
  "medication.code": "Medication",
  "medicationRequest.status": "Medication status",
};

function labelForField(key: string): string {
  if (FHIR_LABELS[key]) return FHIR_LABELS[key]!;
  if (key.includes(".")) {
    const parts = key.split(".");
    const tail = parts.slice(1).join(" ").replace(/([a-z])([A-Z])/g, "$1 $2").toLowerCase();
    return tail ? tail[0]!.toUpperCase() + tail.slice(1) : key;
  }
  return humanKey(key);
}

function initials(name: string) {
  const parts = name.replace(/[^A-Za-z ]/g, " ").trim().split(/\s+/).filter(Boolean);
  return (parts.length ? parts.slice(0, 2).map((p) => p[0]) : ["?"]).join("").toUpperCase();
}

function value(f: Field): string {
  if (f.value === null || f.value === undefined) return "—";
  if (Array.isArray(f.value)) return f.value.map((x) => (typeof x === "string" || typeof x === "number" ? String(x) : JSON.stringify(x))).join(", ");
  return typeof f.value === "string" ? f.value : typeof f.value === "number" ? String(f.value) : JSON.stringify(f.value);
}

export function CoverageStrip({ slots }: { slots: Slot[] | null }) {
  if (!slots) return <Status kind="unknown" weight="weak">Not read</Status>;
  const present = slots.filter((s) => s.status === "present").length;
  return (
    <span className="cov-cell" title={slots.map((s) => `${labelForField(s.concept_key)}: ${cov(s.status).label}`).join("\n")}>
      <span className="cov" aria-hidden="true">{slots.map((s) => <Glyph key={s.concept_key} kind={cov(s.status).glyph} />)}</span>
      <span className="cov-ratio">{present}/{slots.length}</span>
    </span>
  );
}

function needsReview(row: { coverage_slots: Slot[] | null }) {
  return (row.coverage_slots ?? []).some((s) => s.status !== "present");
}

export function Patients() {
  const { ws, navigate, setInspectorOpen } = useApp();
  const { state, reload } = useCommand<{ subjects: SubjectRow[]; scope: string }>("patients_list");
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const isSynthetic = ws?.workspace?.kind === "synthetic";

  const rows = useMemo(() => {
    if (state.status !== "ready") return [];
    const q = query.trim().toLowerCase();
    return state.data.subjects.filter((r) => !q || r.display_name.toLowerCase().includes(q) || r.subject_ref.toLowerCase().includes(q) || r.condition_summary.toLowerCase().includes(q));
  }, [state, query]);
  const current = rows.find((r) => r.subject_ref === selected) ?? rows[0] ?? null;

  const pane = (
    <div className="pane">
      <PageHead
        title="Patients"
        sub={state.status === "ready" ? `${state.data.subjects.length} subject${state.data.subjects.length === 1 ? "" : "s"} · ${state.data.scope}` : "Reading subjects from the vault…"}
        children={<button type="button" className="btn btn-s" onClick={() => navigate("Documents")}>Import FHIR</button>}
      />
      {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
      {state.status !== "error" && (
        <>
          <div className="field-row" style={{ marginBottom: 14 }}>
            <SearchField
              value={query}
              onChange={setQuery}
              placeholder="Search by name, identifier, or condition"
              ariaLabel="Search patients"
            />
            {query && <span className="mono t-mono-sm i3">{rows.length} match{rows.length === 1 ? "" : "es"}</span>}
          </div>
          <table className="tb" aria-label="Patients">
            <thead>
              <tr>
                <th>Patient</th>
                <th style={{ width: "19%" }} className="col-p2">Condition</th>
                <th style={{ width: 142 }}>Coverage</th>
                <th style={{ width: 74 }} className="col-p3 num" title="Sources">Sources</th>
                <th style={{ width: 122 }}>Review</th>
                <th style={{ width: 98 }} className="num" title="Latest event">Latest</th>
              </tr>
            </thead>
            <tbody>
              {state.status === "loading" && Array.from({ length: 2 }, (_, i) => (
                <tr key={i} className="r2"><td colSpan={6}><span className="sk t" style={{ width: "40%" }} /></td></tr>
              ))}
              {rows.map((r) => (
                <tr key={r.subject_ref} className={`r2 clickable ${current?.subject_ref === r.subject_ref ? "sel" : ""}`} tabIndex={0} aria-label={`Select patient ${r.display_name}`} aria-selected={current?.subject_ref === r.subject_ref} onClick={() => { setSelected(r.subject_ref); }} onDoubleClick={() => navigate("Patients", r.subject_ref)} onKeyDown={(e) => { if (e.key === "Enter") navigate("Patients", r.subject_ref); if (e.key === " ") { e.preventDefault(); setSelected(r.subject_ref); setInspectorOpen(true); } }}>
                  <td>
                    <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                      <span className={`mg ${isSynthetic ? "syn" : ""}`} title={isSynthetic ? "Synthetic subject" : "Vault subject"}><span>{initials(r.display_name)}</span></span>
                      <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
                        <span className="pname trunc">{r.display_name}</span>
                        <span className="mono t-mono-sm i3 trunc">{r.subject_ref}</span>
                      </div>
                    </div>
                  </td>
                  <td className="col-p2 t-sm i2"><span className="trunc">{r.condition_summary}</span></td>
                  <td><CoverageStrip slots={r.coverage_slots} /></td>
                  <td className="col-p3 num mono t-mono-sm i2 tnum">{r.source_count}</td>
                  <td>{needsReview(r) ? <Status kind="needs-review">Needs review</Status> : <Status kind="present" weight="weak">Covered</Status>}</td>
                  <td className="num mono t-mono-sm i3 tnum">{r.latest_event ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {state.status === "ready" && state.data.subjects.length > 0 && rows.length === 0 && (
            <Empty icon="search" title="No matching patients" action={<button type="button" className="btn btn-s" onClick={() => setQuery("")}>Clear search</button>}>
              No subject matched &ldquo;{query}&rdquo;. Search checks names, identifiers, and condition summaries.
            </Empty>
          )}
          {state.status === "ready" && state.data.subjects.length > 0 && rows.length > 0 && (
            <div className="tfoot"><span>{rows.length} of {state.data.subjects.length} shown</span><span style={{ flex: 1 }} /><span>Enter opens · Space previews</span></div>
          )}
          {state.status === "ready" && !state.data.subjects.length && (
            <Empty
              icon="patients"
              title="No subjects in this vault"
              action={<button type="button" className="btn btn-p" onClick={() => navigate("Documents")}>Open Documents to import FHIR</button>}
            >
              Subjects appear here after synthetic or permitted FHIR records are imported and verified in Documents. Real patient data remains unauthorized.
            </Empty>
          )}
        </>
      )}
    </div>
  );

  if (!current) return pane;

  return (
    <ResizableSplit
      pane={pane}
      inspector={<PatientInspector subject={current.subject_ref} isSynthetic={isSynthetic} onOpen={() => navigate("Patients", current.subject_ref)} />}
      inspectorLabel="Selected patient"
    />
  );
}

function PatientInspector({ subject, isSynthetic, onOpen }: { subject: string; isSynthetic: boolean; onOpen: () => void }) {
  const { state, reload } = useCommand<Detail>("patient_detail", { subjectRef: subject });
  return (
    <div className="ins-pad">
      {state.status === "loading" && <Skeleton rows={6} />}
      {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
      {state.status === "ready" && (() => {
        const d = state.data;
        return <>
          <div style={{ display: "flex", gap: 12, alignItems: "flex-start" }}>
            <span className={`mg ${isSynthetic ? "syn" : ""} lg`}><span>{initials(d.display_name)}</span></span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div className="t-h2" style={{ fontSize: 16 }}>{d.display_name}</div>
              <div className="mono t-mono-sm i3 trunc" title={d.subject_ref}>{d.subject_ref}</div>
            </div>
            <button type="button" className="btn btn-p btn-inline" onClick={onOpen}>Open</button>
          </div>
          <div className="row-gap" style={{ gap: 16 }}>
            <Status kind={isSynthetic ? "synthetic" : "available"}>{isSynthetic ? "Synthetic" : "Vault record"}</Status>
            <Status kind={d.coverage_state.toLowerCase().includes("review") ? "needs-review" : glyphState(d.coverage_state)}>{d.coverage_state}</Status>
          </div>
          <Section title="Summary">
            <KV keyWidth={118} rows={[
              ...d.contracts.brief.sections.identity.map((f): [string, ReactNode] => [labelForField(f.field_key), <FieldValue f={f} />]),
              ...d.contracts.brief.sections.conditions.map((f): [string, ReactNode] => [labelForField(f.field_key), <FieldValue f={f} />]),
            ]} />
          </Section>
          <Section title="Coverage" count={`${d.contracts.coverage.slots.length} areas`}>
            {d.contracts.coverage.slots.map((s) => (
              <div key={s.concept_key} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", minHeight: 30, borderBottom: "1px solid var(--line-subtle)", gap: 12 }}>
                <span className="lk trunc" title={s.concept_key}>{labelForField(s.concept_key)}</span>
                <Status kind={cov(s.status).glyph} weight={s.status === "present" ? "default" : "weak"}>{cov(s.status).label}</Status>
              </div>
            ))}
          </Section>
          <Section title="Sources" count={d.sources.length}>
            <div className="stack" style={{ gap: 6 }}>
              {d.sources.map((s) => (
                <span key={s.source_id + s.fixture} className="stamp">
                  <span className={`g ${isSynthetic ? "g-synthetic" : "g-present"}`} />
                  {s.resource_type} · {s.source_id}
                </span>
              ))}
            </div>
          </Section>
          <Authority>Read-only. Missing or conflicting evidence is never presented as fact.</Authority>
        </>;
      })()}
    </div>
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
    <span style={{ display: "inline-flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
      <span className={typeof f.value === "string" && /^\d{4}-\d{2}/.test(f.value) ? "mono" : ""}>{value(f)}</span>
      {f.unit && <span className="mono t-mono-sm i3">{f.unit}</span>}
      {f.status !== "present" && <Status kind={c.glyph} weight="weak">{c.label}</Status>}
    </span>
  );
}

const TABS = ["Overview", "Timeline", "Labs", "Coverage", "Sources"] as const;
type Tab = (typeof TABS)[number];

export function PatientDetail({ subject }: { subject: string }) {
  const { ws, navigate } = useApp();
  const { state, reload } = useCommand<Detail>("patient_detail", { subjectRef: subject });
  const [tab, setTab] = useState<Tab>("Overview");
  const isSynthetic = ws?.workspace?.kind === "synthetic";

  if (state.status === "loading") return <div className="pane"><Skeleton rows={8} /></div>;
  if (state.status === "error") return <div className="pane"><ErrorState error={state.error} onRetry={reload} /></div>;
  const d = state.data;
  const b = d.contracts.brief;
  const slots = d.contracts.coverage.slots;
  const tally = b.sections.coverage_summary;
  const notShown = slots.filter((s) => s.status !== "present");
  const nonUnsupportedNotShown = notShown.filter((s) => s.status !== "unsupported_resource_type");
  const notShownNoticeCount = 2 + nonUnsupportedNotShown.length;

  const coverageBarItems: SegmentItem[] = Object.entries(COVERAGE).map(([key, c]) => ({
    key,
    label: c.label,
    count: tally[key] ?? 0,
    tone: c.tone,
  }));

  function onTabKey(e: ReactKeyboardEvent<HTMLDivElement>) {
    const idx = TABS.indexOf(tab);
    let nextIdx = -1;
    if (e.key === "ArrowRight") nextIdx = (idx + 1) % TABS.length;
    else if (e.key === "ArrowLeft") nextIdx = (idx - 1 + TABS.length) % TABS.length;
    else if (e.key === "Home") nextIdx = 0;
    else if (e.key === "End") nextIdx = TABS.length - 1;
    if (nextIdx >= 0) {
      e.preventDefault();
      setTab(TABS[nextIdx]!);
      const tabs = e.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]:not([disabled])');
      tabs[nextIdx]?.focus();
    }
  }

  const pane = (
    <div className="pane" style={{ paddingTop: 20 }}>
      {tab === "Overview" && <>
        <Section title="Summary">
          <BriefTable groups={[["Identity", b.sections.identity], ["Vitals and labs", b.sections.vitals], ["Conditions", b.sections.conditions]]} />
        </Section>
        <Section title="Timeline" count={`${d.timeline.length} events`} actions={<button type="button" className="btn btn-g" onClick={() => setTab("Timeline")}>Open timeline</button>}>
          <Timeline rows={d.timeline.slice(0, 5)} />
        </Section>
        <Section title="Not shown" count={notShownNoticeCount}>
          <div className="stack" style={{ gap: 8 }}>
            <Notice kind="unsupported" title="Medications.">{d.medications_state}</Notice>
            <Notice kind="unavailable" title="Documents.">{d.documents_state}</Notice>
            {nonUnsupportedNotShown.map((s) => (
              <Notice key={s.concept_key} kind={cov(s.status).glyph} title={`${labelForField(s.concept_key)}: ${cov(s.status).label}.`}>
                {s.notes.join(" ") || (s.conflict ? `${s.conflict.members.length} conflicting values (${s.conflict.resolution.toLowerCase()}).` : "Reported directly from source records; nothing is inferred.")}
              </Notice>
            ))}
          </div>
        </Section>
      </>}
      {tab === "Timeline" && (
        <Section title="Longitudinal timeline" count={`${d.timeline.length} events`}>
          <TimelineChart rows={d.timeline} />
          <Timeline rows={d.timeline} />
        </Section>
      )}
      {tab === "Labs" && (
        <Section title="Labs and vitals" count={d.labs.length}>
          {d.labs.length ? (
            <table className="tb">
              <thead><tr><th style={{ width: "32%" }}>Observation</th><th style={{ width: "22%" }}>Value</th><th style={{ width: "20%" }}>Status</th><th>Source</th></tr></thead>
              <tbody>
                {d.labs.map((l, i) => (
                  <tr key={i}>
                    <td className="t-sm">{l.label}</td>
                    <td><span className="mono tnum">{l.value}</span> <span className="mono t-mono-sm i3">{l.unit}</span></td>
                    <td><Status kind={glyphState(l.status) === "unknown" ? cov(l.status.toLowerCase()).glyph : glyphState(l.status)}>{l.status}</Status></td>
                    <td><Stamp k="src" v={l.source} /></td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <Empty icon="data" title="No supported observations">No observation passed the verified extractor for this subject.</Empty>
          )}
        </Section>
      )}
      {tab === "Coverage" && (
        <Section title="Coverage areas" count={`${slots.length} areas`}>
          <table className="tb">
            <thead><tr><th style={{ width: "30%" }}>Area</th><th style={{ width: "22%" }}>Status</th><th>Values &amp; notes</th></tr></thead>
            <tbody>
              {slots.map((s) => (
                <tr key={s.concept_key}>
                  <td><span className="lk" title={s.concept_key}>{labelForField(s.concept_key)}</span></td>
                  <td><Status kind={cov(s.status).glyph}>{cov(s.status).label}</Status></td>
                  <td className="t-sm i2"><span>{s.values.map(value).join(" · ") || s.notes.join(" ") || (s.conflict ? `Conflict: ${s.conflict.members.map(value).join(" vs ")}` : "—")}</span></td>
                </tr>
              ))}
            </tbody>
          </table>
        </Section>
      )}
      {tab === "Sources" && (
        <Section title="Sources" count={d.sources.length}>
          <table className="tb">
            <thead><tr><th style={{ width: "34%" }}>Source</th><th style={{ width: "18%" }}>Resource</th><th style={{ width: "18%" }}>Claim kind</th><th>Origin</th></tr></thead>
            <tbody>
              {d.sources.map((s, i) => (
                <tr key={i}>
                  <td><span className="mono t-mono-sm trunc" title={s.source_id}>{s.source_id}</span></td>
                  <td className="t-sm">{s.resource_type}</td>
                  <td className="t-sm i2">{s.claim_kind}</td>
                  <td><Status kind={isSynthetic ? "synthetic" : "present"} weight="weak"><span className="mono">{s.fixture}</span></Status></td>
                </tr>
              ))}
            </tbody>
          </table>
        </Section>
      )}
    </div>
  );

  const inspector = (
    <div className="ins-pad">
      <Section title="Coverage">
        <div style={{ marginBottom: 14 }}>
          <SegmentedBar
            title="Verified coverage breakdown"
            meta={`${slots.filter((s) => s.status === "present").length} / ${slots.length} present`}
            items={coverageBarItems}
            showLegend={false}
          />
        </div>
        {Object.entries(COVERAGE).map(([key, c]) => (
          <div key={key} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", height: 28, borderBottom: "1px solid var(--line-subtle)" }}>
            <Status kind={c.glyph} weight={(tally[key] ?? 0) ? "strong" : "weak"}>{c.label}</Status>
            <span className={`mono t-mono-sm tnum ${(tally[key] ?? 0) ? "" : "i3"}`}>{tally[key] ?? 0}</span>
          </div>
        ))}
      </Section>
      <Section title="Unavailable sections">
        <div className="stack" style={{ gap: 8 }}>
          <div className="t-sm i2"><b>Medications:</b> {d.medications_state}</div>
          <div className="t-sm i2"><b>Documents:</b> {d.documents_state}</div>
          <div className="t-sm i2"><b>Care Plan:</b> {d.care_plan_state}</div>
        </div>
      </Section>
      <Disclosure summary="Technical presentation rules">
        <KV keyWidth={84} rows={[
          ["extractors", <span className="mono t-mono-sm">{b.built_rules_version.extractors.join(" · ")}</span>],
          ["order", <span className="mono-break">{b.built_rules_version.order_rules}</span>],
          ["schema", <span className="mono t-mono-sm">{b.built_rules_version.brief_schema}</span>],
        ]} />
      </Disclosure>
    </div>
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, minWidth: 0 }}>
      <div style={{ padding: "16px var(--pad-page-x) 0", flex: "none" }}>
        <div className="row-gap" style={{ gap: 14, rowGap: 10 }}>
          <button type="button" className="btn btn-g" onClick={() => navigate("Patients")} aria-label="Back to Patients">
            <Icon name="arrowLeft" size={14} /> Patients
          </button>
          <span className={`mg ${isSynthetic ? "syn" : ""} lg`}><span>{initials(d.display_name)}</span></span>
          <div className="stack" style={{ gap: 2, minWidth: 0 }}>
            <h1 className="t-h1" style={{ fontSize: 20, lineHeight: "26px" }}>{d.display_name}</h1>
            <div className="row-gap" style={{ gap: 14 }}>
              <Status kind={isSynthetic ? "synthetic" : "available"}>{isSynthetic ? "Synthetic" : "Vault record"}</Status>
              {notShown.length ? <Status kind="needs-review">Needs review</Status> : <Status kind="present">Covered</Status>}
            </div>
          </div>
          <div className="ledger" style={{ marginLeft: 8 }}>
            <div>
              <span className="lk">Subject ID</span>
              <span className="lv mono copy-wrap">{d.subject_ref} <CopyButton text={d.subject_ref} label="Copy" ariaLabel="Copy subject identifier" /></span>
            </div>
            <div><span className="lk">Birth date</span><span className="lv mono">{d.birth_date}</span></div>
            <div><span className="lk">Known condition</span><span className="lv">{d.condition_summary}</span></div>
          </div>
        </div>
        <div className="auth" style={{ marginTop: 12, borderBottom: 0 }}>
          <Glyph kind="proposal" />
          <span className="t-label">Authority</span>
          <span>Read-only presentation. Evidence and provenance stay visible; relevance never becomes clinical authority and nothing is committed silently.</span>
        </div>
        <div className="tabs" role="tablist" aria-label="Patient sections" onKeyDown={onTabKey}>
          {TABS.map((t) => (
            <button key={t} type="button" role="tab" aria-selected={tab === t} tabIndex={tab === t ? 0 : -1} className={`tab ${tab === t ? "on" : ""}`} onClick={() => setTab(t)}>
              {t}
              <span className="mono">{t === "Timeline" ? d.timeline.length : t === "Labs" ? d.labs.length : t === "Coverage" ? slots.length : t === "Sources" ? d.sources.length : ""}</span>
            </button>
          ))}
          <button type="button" role="tab" aria-disabled="true" aria-selected={false} tabIndex={-1} className="tab na" title={d.medications_state} disabled>Medications <Glyph kind="unsupported" /></button>
          <button type="button" role="tab" aria-disabled="true" aria-selected={false} tabIndex={-1} className="tab na" title={d.documents_state} disabled>Documents <Glyph kind="unavailable" /></button>
          <button type="button" role="tab" aria-disabled="true" aria-selected={false} tabIndex={-1} className="tab na" title={d.care_plan_state} disabled>Care Plan <Glyph kind="unavailable" /></button>
        </div>
      </div>

      <div className="split">
        <ResizableSplit pane={pane} inspector={inspector} inspectorLabel="Coverage and provenance" />
      </div>
    </div>
  );
}

/** Chronological SVG strip plotting real promoted timeline events along their actual dates. */
function TimelineChart({ rows }: { rows: Detail["timeline"] }) {
  const dated = rows
    .map((r, idx) => ({ ...r, idx, ms: Date.parse(r.date) }))
    .filter((r) => Number.isFinite(r.ms))
    .sort((a, b) => a.ms - b.ms);
  if (dated.length < 2) return null;
  const minMs = Math.min(...dated.map((d) => d.ms));
  const maxMs = Math.max(...dated.map((d) => d.ms));
  const span = Math.max(1, maxMs - minMs);
  return (
    <div className="viz-frame" style={{ marginBottom: 16 }}>
      <div className="viz-head">
        <span className="viz-title">Event chronology</span>
        <span className="viz-meta">{dated[0]!.date} → {dated[dated.length - 1]!.date}</span>
      </div>
      <svg viewBox="0 0 600 48" style={{ width: "100%", height: 48, display: "block" }} role="img" aria-label={`Chronological plot of ${dated.length} promoted events`}>
        <line x1="20" y1="24" x2="580" y2="24" stroke="var(--line-strong)" strokeWidth="1" />
        {dated.map((pt, i) => {
          const x = span === 1 ? 20 + (i / Math.max(1, dated.length - 1)) * 560 : 20 + ((pt.ms - minMs) / span) * 560;
          return (
            <g key={pt.idx}>
              <line x1={x} y1="16" x2={x} y2="32" stroke="var(--accent)" strokeWidth="1.5" />
              <circle cx={x} cy="24" r="4" fill="var(--accent)" stroke="var(--bg-raised)" strokeWidth="1.5">
                <title>{`${pt.date} · ${pt.title}: ${summarizeDetail(pt.detail)}`}</title>
              </circle>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

function BriefTable({ groups }: { groups: Array<[string, Field[]]> }) {
  return (
    <table className="tb">
      <thead><tr><th style={{ width: "28%" }}>Field</th><th style={{ width: "32%" }}>Value</th><th style={{ width: "18%" }}>Status</th><th>Evidence</th></tr></thead>
      <tbody>
        {groups.map(([group, fields]) => [
          <tr key={`${group}-h`}><td colSpan={4} style={{ height: 30, paddingTop: 10, borderBottom: 0 }}><span className="t-label">{group}</span></td></tr>,
          ...(fields.length ? fields.map((f, i) => (
            <tr key={`${group}-${i}`}>
              <td><span className="lk" title={f.field_key}>{labelForField(f.field_key)}</span></td>
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
        <div key={i} className="lr" style={{ gridTemplateColumns: "136px 18px 96px minmax(0,1fr) auto", columnGap: 12, paddingBlock: 8 }}>
          <span className="mono t-mono-sm i2 tnum" style={{ whiteSpace: "nowrap" }}>{e.date}</span>
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
      .map(([k, x]) => `${labelForField(k)} ${typeof x === "object" ? Object.values(x as object).join(" ") : String(x)}`)
      .join(" · ");
  } catch {
    return detail;
  }
}
