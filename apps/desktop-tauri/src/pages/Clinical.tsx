import { useMemo, useState, type FormEvent } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { selectedPathOrPrevious } from "../audio-picker";
import { useApp } from "../lib/app";
import { call, useCommand, type CmdError } from "../lib/ipc";
import {
  Authority, CopyButton, Empty, ErrorState, Glyph, KV, Ledger, Notice, PageHead,
  ResizableSplit, SearchField, Section, SegmentedBar, Skeleton, Stamp, Status, glyphFor,
  type SegmentItem,
} from "../components/ui";

/* ───────────────────────────── Documents ───────────────────────────── */
type Sources = { sources: Array<{ source_id: string; resource_type: string; fixture: string; claim_kind: string; subject_ref: string }>; document_understanding: string };

export function Documents() {
  const { navigate, ws, lock } = useApp();
  const { state, reload } = useCommand<Sources>("documents_list");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [type, setType] = useState("all");
  const rows = useMemo(() => state.status === "ready" ? state.data.sources.filter((s) => type === "all" || s.resource_type === type) : [], [state, type]);
  const types = state.status === "ready" ? [...new Set(state.data.sources.map((s) => s.resource_type))] : [];
  const cur = rows.find((s) => s.source_id === selectedId) ?? rows[0];
  const [imp, setImp] = useState<{ path: string; subject: string } | null>(null);
  const sampleWorkspace = ws?.workspace?.kind !== "encrypted";
  const [impState, setImpState] = useState<{ pending: boolean; error: string | null; done: string | null }>({ pending: false, error: null, done: null });
  const [choosingFhir, setChoosingFhir] = useState(false);

  const typeSegments = useMemo<SegmentItem[]>(() => {
    if (state.status !== "ready") return [];
    const tones: Array<SegmentItem["tone"]> = ["present", "neutral", "attention", "hatched", "muted"];
    return types.map((t, i) => ({
      key: t,
      label: t,
      count: state.data.sources.filter((s) => s.resource_type === t).length,
      tone: tones[i % tones.length]!,
    }));
  }, [state, types]);

  async function browseFhir() {
    if (!imp) return;
    setChoosingFhir(true);
    try {
      const selected = await open({ multiple: false, directory: false, title: "Choose a FHIR R4 JSON or NDJSON file", filters: [{ name: "FHIR R4 JSON or NDJSON", extensions: ["json", "ndjson"] }] });
      // Cancel is not an error and never overwrites a previously entered path.
      setImp((c) => (c ? { ...c, path: selectedPathOrPrevious(c.path, selected) } : c));
    } catch {
      setImpState({ pending: false, error: "The file chooser could not open. You can still enter the file's absolute path.", done: null });
    } finally { setChoosingFhir(false); }
  }
  async function importFhir(e: FormEvent) {
    e.preventDefault();
    if (!imp) return;
    setImpState({ pending: true, error: null, done: null });
    try {
      const r = await call<{ source_id: string; resource_type: string }>("documents_import_fhir", { path: imp.path, subjectRef: imp.subject });
      setImpState({ pending: false, error: null, done: `${r.resource_type} ingested and promoted as ${r.source_id}` });
      setImp(null);
      setSelectedId(r.source_id);
      await reload();
    } catch (err) {
      setImpState({ pending: false, error: (err as CmdError).message ?? String(err), done: null });
    }
  }

  const pane = (
    <div className="pane">
      <PageHead title="Documents" sub="Source files in this workspace, and exactly what was extracted from each.">
        <button
          type="button"
          className="btn btn-s"
          aria-expanded={!!imp}
          onClick={() => setImp(imp ? null : { path: "", subject: sampleWorkspace ? "synthetic-subject-ada" : "" })}
        >
          Import synthetic FHIR
        </button>
      </PageHead>
      {imp && sampleWorkspace && (
        <div style={{ marginBottom: 14 }}>
          <Notice kind="limited" title="Imports go into an encrypted vault.">
            The sample workspace isn&apos;t encrypted and holds only its bundled synthetic records. Lock it, then create or unlock an encrypted vault to import a FHIR file.
            <div style={{ marginTop: 8 }}>
              <button type="button" className="btn btn-s" onClick={() => void lock()}>Lock and go to the vault</button>
            </div>
          </Notice>
        </div>
      )}
      {imp && !sampleWorkspace && (
        <form onSubmit={importFhir} className="viz-frame stack" style={{ gap: 10, maxWidth: 660, marginBottom: 16 }}>
          <div>
            <label className="lbl" htmlFor="ip">FHIR R4 JSON or NDJSON file</label>
            <div className="row-gap" style={{ gap: 8 }}>
              <input id="ip" value={imp.path} maxLength={1024} placeholder="Choose a file or paste its absolute path" style={{ flex: 1, minWidth: 0 }} onChange={(e) => setImp({ ...imp, path: e.target.value })} />
              <button type="button" className="btn btn-s" onClick={browseFhir} disabled={choosingFhir || impState.pending}>Browse JSON…</button>
            </div>
            <p className="t-sm i3" style={{ marginTop: 6 }}>Up to 64 MiB for a Bundle or NDJSON (8 MiB per resource). Synthetic or permitted test data only; sealed in this vault. The original file is read once and never modified.</p>
          </div>
          <div>
            <label className="lbl" htmlFor="is">Subject</label>
            <input id="is" value={imp.subject} maxLength={128} placeholder="e.g. synthetic-subject-ada" style={{ width: "100%" }} onChange={(e) => setImp({ ...imp, subject: e.target.value })} />
          </div>
          <div className="row-gap" style={{ gap: 8 }}>
            <button type="submit" className="btn btn-p btn-inline" disabled={!imp.path.trim() || !imp.subject.trim() || impState.pending}>
              {impState.pending ? <Glyph kind="running" /> : null}Ingest through Core
            </button>
            <button type="button" className="btn btn-g" onClick={() => setImp(null)} disabled={impState.pending}>Cancel</button>
          </div>
        </form>
      )}
      {impState.error && <div style={{ marginBottom: 12 }}><Notice kind="failed" title="Not imported.">{impState.error}</Notice></div>}
      {impState.done && <div style={{ marginBottom: 12 }}><Notice kind="present" title="Imported.">{impState.done}</Notice></div>}
      {state.status === "loading" && <Skeleton />}
      {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
      {state.status === "ready" && <>
        {state.data.sources.length > 0 && (
          <div style={{ marginBottom: 14 }}>
            <SegmentedBar
              title="Sources by FHIR resource type"
              meta={`${state.data.sources.length} total`}
              items={typeSegments}
            />
          </div>
        )}
        <div className="field-row" style={{ marginBottom: 12 }}>
          <button type="button" className={`flt ${type === "all" ? "on" : ""}`} aria-pressed={type === "all"} onClick={() => { setType("all"); setSelectedId(null); }}>
            All <span className="mono">{state.data.sources.length}</span>
          </button>
          {types.map((t) => (
            <button key={t} type="button" className={`flt ${type === t ? "on" : ""}`} aria-pressed={type === t} onClick={() => { setType(t); setSelectedId(null); }}>
              {t} <span className="mono">{state.data.sources.filter((s) => s.resource_type === t).length}</span>
            </button>
          ))}
        </div>
        {rows.length ? (
          <table className="tb" aria-label="Sources">
            <thead>
              <tr>
                <th>Source</th>
                <th style={{ width: 152 }}>Resource</th>
                <th style={{ width: 132 }}>Extraction</th>
                <th className="col-p3" style={{ width: "20%" }}>Subject</th>
                <th style={{ width: 108 }}>Origin</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((s, i) => {
                const imported = s.fixture.startsWith("imported:");
                return (
                  <tr key={s.source_id + i} className={`r2 clickable ${cur?.source_id === s.source_id ? "sel" : ""}`} tabIndex={0} aria-label={`Select source ${s.source_id}`} aria-selected={cur?.source_id === s.source_id} onClick={() => setSelectedId(s.source_id)} onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setSelectedId(s.source_id); } }}>
                    <td>
                      <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
                        <span className="mono trunc" style={{ fontSize: 12 }}>{s.source_id}</span>
                        <span className="t-sm i3 trunc">FHIR R4 · {s.fixture}</span>
                      </div>
                    </td>
                    <td className="t-sm">{s.resource_type}</td>
                    <td>
                      <div style={{ display: "flex", flexDirection: "column", gap: 2, minWidth: 0 }}>
                        <Status kind="present">Structured</Status>
                        <span className="mono t-mono-sm i3 trunc">{s.claim_kind}</span>
                      </div>
                    </td>
                    <td className="col-p3" title={s.subject_ref}><span className="mono t-mono-sm i2 trunc">{s.subject_ref}</span></td>
                    <td title={imported ? "Imported synthetic FHIR file" : "Bundled synthetic FHIR fixture"}>
                      <Status kind={imported ? "available" : "synthetic"} weight="weak">{imported ? "Imported" : "Synthetic"}</Status>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        ) : (
          <Empty
            icon="documents"
            title="No sources in this vault"
            action={!sampleWorkspace ? <button type="button" className="btn btn-p" onClick={() => setImp({ path: "", subject: "" })}>Import synthetic FHIR</button> : undefined}
          >
            {sampleWorkspace
              ? "Bundled synthetic FHIR sources load automatically when the sample workspace opens."
              : "Import a synthetic or permitted FHIR R4 JSON or NDJSON file to seal and inspect its extracted records inside this encrypted vault."}
          </Empty>
        )}
        <Section title="Document understanding">
          <Notice kind="unsupported" title="Not admitted.">{state.data.document_understanding}</Notice>
        </Section>
      </>}
    </div>
  );

  if (!cur) return pane;

  const imported = cur.fixture.startsWith("imported:");
  const inspector = (
    <div className="ins-pad">
      <div>
        <div className="copy-wrap">
          <span className="mono trunc" style={{ fontSize: 13.5, fontWeight: 500 }}>{cur.source_id}</span>
          <CopyButton text={cur.source_id} ariaLabel="Copy source ID" />
        </div>
        <div className="row-gap" style={{ gap: 14, marginTop: 8 }}>
          <Status kind={imported ? "available" : "synthetic"}>{imported ? "Imported" : "Synthetic"}</Status>
          <Status kind="present">Structured</Status>
        </div>
      </div>
      <Section title="Provenance">
        <KV keyWidth={100} rows={[
          ["resource", cur.resource_type],
          ["claim kind", <span className="mono t-mono-sm">{cur.claim_kind}</span>],
          ["subject", <span className="mono t-mono-sm">{cur.subject_ref}</span>],
          ["origin", <span className="mono t-mono-sm">{cur.fixture}</span>],
          ["pipeline", "Verified ingest → proposal → promotion"],
        ]} />
      </Section>
      <div><button type="button" className="btn btn-s" onClick={() => navigate("Patients", cur.subject_ref)}>Open subject</button></div>
      <Authority>Holding a file doesn&apos;t mean it was understood. Only fields that passed a verified extractor appear in the patient summary.</Authority>
    </div>
  );

  return <ResizableSplit pane={pane} inspector={inspector} inspectorLabel="Selected source" />;
}

/* ───────────────────────────── Evidence ───────────────────────────── */
type Corpus = {
  origin: "bundled_synthetic_fixture"; corpus_id: string; version: string; source_identity: string; content_digest: string; rights: string; rights_uri: string;
  documents: Array<{ doc_id: string; text: string; content_digest: string; status: string; freshness_marker: string | null; conflict_marker: string | null }>;
};
type Hit = { doc_id: string; score: number; snippet: string; retracted: boolean; freshness_marker: string | null; conflict_marker: string | null; relevance_only: boolean };
type Retrieval = { corpus_source_identity: string; query: string; hits: Hit[]; evaluation_id: string; relevance_is_not_authority: boolean };

const short = (hex: string) => (hex.length > 16 ? `${hex.slice(0, 8)}…${hex.slice(-6)}` : hex);

export function Evidence() {
  const { state, reload } = useCommand<Corpus>("evidence_corpus");
  const [sel, setSel] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [retracted, setRetracted] = useState(false);
  const [result, setResult] = useState<Retrieval | null>(null);
  const [err, setErr] = useState<CmdError | null>(null);
  const [busy, setBusy] = useState(false);

  async function search(e: FormEvent) {
    e.preventDefault();
    if (!query.trim()) { setResult(null); return; }
    setBusy(true); setErr(null);
    try { setResult(await call<Retrieval>("evidence_search", { query, includeRetracted: retracted })); }
    catch (x) { setErr(x as CmdError); setResult(null); }
    finally { setBusy(false); }
  }

  if (state.status === "loading") return <div className="pane"><Skeleton /></div>;
  if (state.status === "error") return <div className="pane"><ErrorState error={state.error} onRetry={reload} /></div>;
  const c = state.data;
  const docs = c.documents;
  const cur = docs.find((d) => d.doc_id === sel) ?? docs.find((d) => d.conflict_marker) ?? docs[0];
  const activeCount = docs.filter((d) => d.status !== "retracted" && !d.conflict_marker).length;
  const conflictCount = docs.filter((d) => !!d.conflict_marker).length;
  const retractedCount = docs.filter((d) => d.status === "retracted").length;
  const maxScore = result ? Math.max(0.01, ...result.hits.map((h) => h.score)) : 1;

  const pane = (
    <div className="pane">
      <PageHead title="Evidence" sub="A fixed synthetic sample bundled with MedScale, identical in every workspace. Not live literature, and not imported documents." />
      <Ledger items={[
        { k: "origin", v: <Status kind="synthetic" weight="strong">Bundled sample</Status> },
        { k: "source", v: c.source_identity, mono: true },
        { k: "rights", v: <Status kind="synthetic" weight="strong">{c.rights}</Status> },
        { k: "content_digest", v: short(c.content_digest), mono: true },
        { k: "search", v: "Keyword · on device" },
        { k: "verification", v: <Status kind="present" weight="strong">Digest verified at admission</Status> },
      ]} />

      <div style={{ marginBottom: 14 }}>
        <SegmentedBar
          title="Corpus document posture"
          meta={`${docs.length} bundled documents`}
          items={[
            { key: "active", label: "Active", count: activeCount, tone: "present" },
            { key: "conflict", label: "Conflict (needs review)", count: conflictCount, tone: "attention" },
            { key: "retracted", label: "Retracted", count: retractedCount, tone: "muted" },
          ]}
        />
      </div>

      <form onSubmit={search} className="field-row" style={{ marginBottom: 14 }}>
        <SearchField value={query} onChange={setQuery} placeholder="Search these documents, on this device" ariaLabel="Search corpus text" />
        <button type="button" className={`flt ${retracted ? "on" : ""}`} onClick={() => setRetracted((r) => !r)} aria-pressed={retracted}>Include retracted</button>
        <button type="submit" className="btn btn-s" disabled={busy}>{busy ? <><Glyph kind="running" />Searching</> : "Search"}</button>
        {result && <button type="button" className="btn btn-g" onClick={() => { setResult(null); setQuery(""); }}>Clear</button>}
      </form>
      {err && <ErrorState error={err} />}
      {result ? (
        <Section title="Retrieval matches" count={`${result.hits.length} · ${result.corpus_source_identity}`}>
          {result.hits.length === 0 ? <p className="t-sm i3">No document matched &ldquo;{result.query}&rdquo;.</p> : result.hits.map((h) => (
            <div key={h.doc_id} className={`lr clickable ${sel === h.doc_id ? "sel" : ""}`} style={{ gridTemplateColumns: "150px minmax(0,1fr) 128px 94px", minHeight: "var(--row-2)" }} role="button" tabIndex={0} aria-label={`Open ${h.doc_id}`} aria-pressed={sel === h.doc_id} onClick={() => setSel(h.doc_id)} onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setSel(h.doc_id); } }}>
              <span className={`mono ${h.retracted ? "i3" : ""}`} style={{ fontSize: 12 }}>{h.doc_id}</span>
              <span className={`trunc ${h.retracted ? "i3" : ""}`} style={{ fontSize: 13 }}>{h.snippet}</span>
              <span>{h.conflict_marker ? <Status kind="needs-review" weight="strong">Conflict</Status> : h.retracted ? <Status kind="cancelled" weight="weak">Retracted</Status> : <Status kind="proposal" weight="weak">Relevance only</Status>}</span>
              <span style={{ display: "flex", alignItems: "center", gap: 6, justifyContent: "flex-end" }} title={`Lexical retrieval score: ${h.score.toFixed(2)} (not authority)`}>
                <span style={{ width: 42, height: 5, background: "var(--bg-active)", borderRadius: 2, overflow: "hidden", display: "inline-block" }} aria-hidden="true">
                  <span style={{ display: "block", height: "100%", width: `${Math.round((h.score / maxScore) * 100)}%`, background: h.conflict_marker ? "var(--signal)" : "var(--accent)" }} />
                </span>
                <span className="mono t-mono-sm i3 tnum">{h.score.toFixed(2)}</span>
              </span>
            </div>
          ))}
          <p className="note" style={{ marginTop: 8 }}>Evaluation <span className="mono">{result.evaluation_id}</span>. Scores measure keyword match only — never clinical authority.</p>
        </Section>
      ) : (
        <Section title="Documents" count={docs.length}>
          <div role="table" aria-label="Corpus documents">
            <div role="row" className="lr" style={{ gridTemplateColumns: "152px minmax(0,1fr) 114px 96px", minHeight: "var(--head)", borderBottom: "1px solid var(--line)" }}>
              {["Document", "Text and digest", "Conflicts", "Status"].map((h) => <span key={h} role="columnheader" className="t-label">{h}</span>)}
            </div>
            {docs.map((d) => (
              <div role="row" key={d.doc_id} tabIndex={0} aria-label={`Select document ${d.doc_id}`} aria-selected={cur?.doc_id === d.doc_id} className={`lr clickable ${cur?.doc_id === d.doc_id ? "sel" : ""}`} style={{ gridTemplateColumns: "152px minmax(0,1fr) 114px 96px", minHeight: "var(--row-2)" }} onClick={() => setSel(d.doc_id)} onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setSel(d.doc_id); } }}>
                <span role="cell" className={`mono trunc ${d.status === "retracted" ? "i3" : ""}`} style={{ fontSize: 12 }}>{d.doc_id}</span>
                <div role="cell" style={{ display: "flex", flexDirection: "column", gap: 2, minWidth: 0, padding: "6px 0" }}>
                  <span className={`trunc ${d.status === "retracted" ? "i3" : ""}`} style={{ fontSize: 13 }}>{d.text}</span>
                  <span className="stamp trunc"><i>sha256</i>{short(d.content_digest)}{d.freshness_marker && <><span className="dot" /><i>fresh</i>{d.freshness_marker}</>}</span>
                </div>
                <span role="cell">{d.conflict_marker ? <Status kind="needs-review" weight="strong">Conflict</Status> : <span className="t-sm i3">None</span>}</span>
                <span role="cell"><Status kind={d.status === "retracted" ? "cancelled" : "available"} weight={d.status === "retracted" ? "weak" : "default"}>{d.status === "retracted" ? "Retracted" : "Active"}</Status></span>
              </div>
            ))}
          </div>
        </Section>
      )}
    </div>
  );

  if (!cur) return pane;

  const inspector = (
    <div className="ins-pad">
      <div className="stack" style={{ gap: 8 }}>
        <div className="copy-wrap">
          <span className="mono trunc" style={{ fontSize: 14, fontWeight: 500 }}>{cur.doc_id}</span>
          <CopyButton text={cur.doc_id} ariaLabel="Copy document ID" />
        </div>
        <div className="row-gap" style={{ gap: 14 }}>
          <Status kind={cur.status === "retracted" ? "cancelled" : "available"}>{cur.status === "retracted" ? "Retracted" : "Active"}</Status>
          {cur.conflict_marker && <Status kind="needs-review" weight="strong">Conflict · needs review</Status>}
        </div>
      </div>
      <p style={{ fontSize: 13, lineHeight: "21px", padding: "12px 14px", background: "var(--bg-surface)", border: "1px solid var(--line-subtle)", borderRadius: "var(--r-2)" }}>{cur.text}</p>
      <Section title="Provenance">
        <KV keyWidth={102} rows={[
          ["content_digest", <div className="stack" style={{ gap: 6 }}><span className="mono-break">{cur.content_digest}</span><div><CopyButton text={cur.content_digest} label="Copy SHA-256" /></div></div>],
          ["conflict_marker", <span className="mono t-mono-sm">{cur.conflict_marker ?? "none"}</span>],
          ["freshness", <span className="mono t-mono-sm">{cur.freshness_marker ?? "none"}</span>],
          ["corpus", <span className="mono t-mono-sm">{c.source_identity}</span>],
          ["rights_uri", <div className="stack" style={{ gap: 6 }}><span className="mono-break">{c.rights_uri}</span><div><CopyButton text={c.rights_uri} label="Copy URI" /></div></div>],
        ]} />
      </Section>
      <Authority>{cur.conflict_marker ? "The conflict stays unresolved. MedScale does not pick a side or turn this document into a recommendation." : "Evidence is context. Relevance never becomes clinical authority."}</Authority>
    </div>
  );

  return <ResizableSplit pane={pane} inspector={inspector} inspectorLabel="Selected document" />;
}

/* ───────────────────────────── Insights ───────────────────────────── */
type Row = { label: string; value: string; detail: string; status: string };
type InsightsVm = {
  cohort_size: number; review_attention_count: number; evidence_gap_count: number; present_count: number; synthetic_only: boolean;
  condition_distribution: Row[]; coverage_distribution: Row[];
  cohorts: Array<{ subject_ref: string; display_name: string; condition: string; evidence_state: string; review_attention: string }>;
  evidence: Array<{ source: string; title: string; snippet: string; metadata: string }>;
  risk_state: string; trend_state: string; gap_state: string; recommendation_state: string; assistant_default: string;
};

type GroundedAnswer = {
  answer: { abstained: boolean; unsupported_terms: string[]; note: string; citations: Array<{ sentence: string; doc_id: string; content_digest: string; support: number; conflict_marker: string | null; freshness_marker: string | null }> };
  corpus_id: string; corpus_version: string;
};

function coverageBarColor(label: string): string {
  const v = label.toLowerCase();
  if (v.includes("present")) return "var(--accent)";
  if (v.includes("conflict")) return "var(--signal)";
  if (v.includes("unhealthy")) return "var(--sem-danger)";
  if (v.includes("incomparable")) return "repeating-linear-gradient(135deg, var(--ink-2) 0 2px, transparent 2px 4px)";
  if (v.includes("absent")) return "var(--ink-2)";
  return "var(--ink-4)";
}

export function Insights() {
  const { navigate } = useApp();
  const { state, reload } = useCommand<InsightsVm>("insights_overview");
  const [q, setQ] = useState("");
  const [answer, setAnswer] = useState<GroundedAnswer | null>(null);
  const [err, setErr] = useState<CmdError | null>(null);
  const [asking, setAsking] = useState(false);

  async function ask(e: FormEvent) {
    e.preventDefault();
    if (!q.trim() || asking) return;
    setErr(null);
    setAsking(true);
    try { setAnswer(await call<GroundedAnswer>("insights_ask", { query: q })); }
    catch (x) { setErr(x as CmdError); setAnswer(null); }
    finally { setAsking(false); }
  }

  if (state.status === "loading") return <div className="pane"><Skeleton /></div>;
  if (state.status === "error") return <div className="pane"><ErrorState error={state.error} onRetry={reload} /></div>;
  const v = state.data;
  const max = Math.max(1, ...v.coverage_distribution.map((r) => Number(r.value) || 0));
  const maxCond = Math.max(1, ...v.condition_distribution.map((r) => Number(r.value) || 0));

  const pane = (
    <div className="pane">
      <PageHead title="Insights" sub="What the records support across these subjects. Counts come from verified coverage only, with no invented scores or predictions." />
      <Ledger items={[
        { k: "subjects", v: v.cohort_size, mono: true },
        { k: "present_facts", v: v.present_count, mono: true },
        { k: "evidence_gaps", v: v.evidence_gap_count, mono: true },
        { k: "need_review", v: v.review_attention_count, mono: true },
        { k: "scope", v: <Status kind={v.synthetic_only ? "synthetic" : "available"} weight="strong">{v.synthetic_only ? "Synthetic only" : "Workspace"}</Status> },
      ]} />
      <Section title="Subjects" count={v.cohorts.length}>
        {v.cohorts.length ? (
          <table className="tb">
            <thead><tr><th style={{ width: "22%" }}>Subject</th><th style={{ width: "16%" }}>Condition</th><th style={{ width: "38%" }}>Evidence</th><th>Review</th></tr></thead>
            <tbody>
              {v.cohorts.map((c) => (
                <tr key={c.subject_ref} className="clickable" tabIndex={0} aria-label={`Open patient ${c.display_name}`} onClick={() => navigate("Patients", c.subject_ref)} onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); navigate("Patients", c.subject_ref); } }}>
                  <td><span className="pname trunc">{c.display_name}</span></td>
                  <td className="t-sm i2"><span className="trunc">{c.condition}</span></td>
                  <td title={c.evidence_state}><Status kind={glyphFor(c.evidence_state)}>{c.evidence_state}</Status></td>
                  <td><Status kind={/review/i.test(c.review_attention) ? "needs-review" : "present"} weight="weak">{c.review_attention}</Status></td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <Empty icon="insights" title="No subjects to summarize" action={<button type="button" className="btn btn-s" onClick={() => navigate("Documents")}>Open Documents</button>}>
            Import synthetic FHIR records in Documents to inspect cohort coverage across subjects.
          </Empty>
        )}
      </Section>
      <Section title="Coverage" count={`${v.coverage_distribution.length} statuses`}>
        <div style={{ borderTop: "1px solid var(--line-subtle)" }}>
          {v.coverage_distribution.map((r) => {
            const n = Number(r.value) || 0;
            return (
              <div key={r.label} className="lr" style={{ gridTemplateColumns: "minmax(152px, 190px) 32px minmax(56px, 0.65fr) minmax(0, 1.35fr)", minHeight: 36 }}>
                <Status kind={glyphFor(r.label)} weight={n ? "strong" : "weak"}>{r.label}</Status>
                <span className={`mono t-mono-sm tnum ${n ? "" : "i3"}`} style={{ textAlign: "right" }}>{r.value}</span>
                <span style={{ height: 8, background: "var(--bg-active)", borderRadius: 2, overflow: "hidden", position: "relative" }} aria-hidden="true">
                  <span style={{ position: "absolute", inset: 0, width: `${(n / max) * 100}%`, background: n ? coverageBarColor(r.label) : "transparent" }} />
                </span>
                <span className="t-sm i3">{r.detail}</span>
              </div>
            );
          })}
        </div>
      </Section>
      <Section title="Not computed" count="by design">
        <div className="grid-3">
          {[["Risk", v.risk_state], ["Trend", v.trend_state], ["Recommendation", v.recommendation_state]].map(([k, s]) => (
            <div key={k} className="viz-frame">
              <Status kind="unsupported">{k}</Status>
              <p className="t-sm i3" style={{ marginTop: 6 }}>{s}</p>
            </div>
          ))}
        </div>
      </Section>
    </div>
  );

  const inspector = (
    <div className="ins-pad">
      <Section title="Conditions" count="present facts only">
        {v.condition_distribution.length ? v.condition_distribution.map((r) => {
          const n = Number(r.value) || 0;
          return (
            <div key={r.label} style={{ padding: "8px 0", borderBottom: "1px solid var(--line-subtle)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", gap: 10, marginBottom: 4 }}>
                <span style={{ fontSize: 13 }} className="trunc">{r.label}</span>
                <span className="mono t-mono-sm tnum">{r.value}</span>
              </div>
              <div style={{ height: 4, background: "var(--bg-active)", borderRadius: 2, overflow: "hidden" }} aria-hidden="true">
                <div style={{ height: "100%", width: `${Math.round((n / maxCond) * 100)}%`, background: "var(--accent)" }} />
              </div>
            </div>
          );
        }) : <p className="t-sm i3">No present condition facts.</p>}
      </Section>
      <Section title="Evidence in context" count={v.evidence.length} actions={<button type="button" className="btn btn-g" onClick={() => navigate("Evidence")}>Open Evidence</button>}>
        {v.evidence.map((e, i) => (
          <div key={i} style={{ padding: "10px 0", borderBottom: "1px solid var(--line-subtle)" }} className="stack">
            <span style={{ fontSize: 13, lineHeight: "19px" }}>{e.snippet}</span>
            <span className="row-gap" style={{ gap: 10 }}><Stamp k="src" v={e.source} />{/RETRACTED/.test(e.metadata) && <Status kind="cancelled" weight="weak">Retracted</Status>}</span>
          </div>
        ))}
      </Section>
      <Section title="Ask the evidence">
        <form onSubmit={ask} className="stack" style={{ gap: 8 }}>
          <input type="text" value={q} onChange={(e) => setQ(e.target.value)} placeholder="Ask about these documents" maxLength={256} aria-label="Question for the evidence corpus" />
          <div><button type="submit" className="btn btn-s" disabled={asking}>{asking ? <><Glyph kind="running" />Asking</> : "Ask"}</button></div>
        </form>
        {err && <ErrorState error={err} />}
        {!answer && <p className="t-sm i3" style={{ marginTop: 10 }}>Answers are sentences extracted from retrieved documents, each with its citation. When nothing supports a question, MedScale says so.</p>}
        {answer && (answer.answer.abstained
          ? <div style={{ marginTop: 10 }}><Notice kind="unknown" title="No supporting evidence.">No passage in {answer.corpus_id} supports an answer to this question.</Notice></div>
          : <div className="stack" style={{ gap: 10, marginTop: 10 }}>
              {answer.answer.citations.map((c, i) => (
                <div key={i} className="viz-frame stack" style={{ gap: 6, padding: "10px 12px" }}>
                  <span className="t-sm i1" style={{ lineHeight: "19px" }}>{c.sentence}</span>
                  <div className="row-gap" style={{ gap: 8 }}>
                    <Stamp k={c.doc_id} v={c.content_digest.slice(0, 10)} />
                    <span className="mono t-mono-sm i3 tnum" title="Lexical support score">match {(c.support * 100).toFixed(0)}%</span>
                    {c.conflict_marker && <Status kind="conflict" weight="weak">Conflict</Status>}
                    {c.freshness_marker && <Status kind="limited" weight="weak">{c.freshness_marker}</Status>}
                  </div>
                </div>
              ))}
              {answer.answer.unsupported_terms.length > 0 && <p className="t-sm i3">Not supported by any cited sentence: {answer.answer.unsupported_terms.join(", ")}.</p>}
            </div>)}
        {answer && <p className="t-sm i3" style={{ marginTop: 8 }}>{answer.answer.note}</p>}
      </Section>
      <Authority>Answers quote the documents in this workspace. They are not clinical advice.</Authority>
    </div>
  );

  return <ResizableSplit pane={pane} inspector={inspector} inspectorLabel="Conditions and evidence" keepVisible />;
}
