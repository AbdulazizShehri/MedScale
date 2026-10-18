import { useMemo, useState, type FormEvent } from "react";
import { useApp } from "../lib/app";
import { call, useCommand, type CmdError } from "../lib/ipc";
import { Authority, Empty, ErrorState, Glyph, KV, Ledger, Notice, PageHead, Section, Skeleton, Stamp, Status, glyphFor } from "../components/ui";

/* ───────────────────────────── Documents ───────────────────────────── */
type Sources = { sources: Array<{ source_id: string; resource_type: string; fixture: string; claim_kind: string; subject_ref: string }>; document_understanding: string };

export function Documents() {
  const { navigate } = useApp();
  const { state, reload } = useCommand<Sources>("documents_list");
  const [sel, setSel] = useState<number>(0);
  const [type, setType] = useState("all");
  const rows = useMemo(() => state.status === "ready" ? state.data.sources.filter((s) => type === "all" || s.resource_type === type) : [], [state, type]);
  const types = state.status === "ready" ? [...new Set(state.data.sources.map((s) => s.resource_type))] : [];
  const cur = rows[sel] ?? rows[0];

  return (
    <>
      <div className="pane">
        <PageHead title="Documents" sub="Source material held in the vault, and exactly what was extracted from it." />
        {state.status === "loading" && <Skeleton />}
        {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
        {state.status === "ready" && <>
          <div className="field-row" style={{ marginBottom: 10 }}>
            <button type="button" className={`flt ${type === "all" ? "on" : ""}`} onClick={() => setType("all")}>All <span className="mono">{state.data.sources.length}</span></button>
            {types.map((t) => <button key={t} type="button" className={`flt ${type === t ? "on" : ""}`} onClick={() => { setType(t); setSel(0); }}>{t} <span className="mono">{state.data.sources.filter((s) => s.resource_type === t).length}</span></button>)}
          </div>
          {rows.length ? (
            <table className="tb" aria-label="Sources">
              <thead><tr><th style={{ width: "30%" }}>Source</th><th style={{ width: "14%" }}>Resource</th><th style={{ width: "15%" }}>Extraction</th><th className="col-p3" style={{ width: "27%" }}>Subject</th><th>Origin</th></tr></thead>
              <tbody>{rows.map((s, i) => (
                <tr key={s.source_id + i} className={`r2 clickable ${cur === s ? "sel" : ""}`} tabIndex={0} onClick={() => setSel(i)} onKeyDown={(e) => e.key === "Enter" && navigate("Patients", s.subject_ref)}>
                  <td><div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="mono trunc" style={{ fontSize: 12 }}>{s.source_id}</span><span className="t-sm i3">FHIR R4 · {s.fixture}</span></div></td>
                  <td className="t-sm">{s.resource_type}</td>
                  <td><Status kind="present">Structured</Status> <span className="mono t-mono-sm i3">{s.claim_kind}</span></td>
                  <td className="col-p3" title={s.subject_ref}><span className="mono t-mono-sm i2 trunc">{s.subject_ref}</span></td>
                  <td title="Bundled synthetic FHIR fixture"><Status kind="synthetic" weight="weak">Synthetic</Status></td>
                </tr>
              ))}</tbody>
            </table>
          ) : <Empty icon="documents" title="No sources in this vault">Sources appear after governed ingest. Open the synthetic workspace to use the bundled synthetic FHIR fixtures; encrypted vaults do not accept clinical ingest yet.</Empty>}
          <Section n="02" title="Document understanding">
            <Notice kind="unsupported" title="Not admitted.">{state.data.document_understanding}</Notice>
          </Section>
        </>}
      </div>
      {cur && (
        <aside className="inspector collapsible" aria-label="Selected source">
          <div className="ins-pad">
            <div><div className="mono" style={{ fontSize: 13.5, fontWeight: 500 }}>{cur.source_id}</div>
              <div className="row-gap" style={{ gap: 14, marginTop: 8 }}><Status kind="synthetic">Synthetic</Status><Status kind="present">Structured</Status></div></div>
            <Section title="Provenance"><KV keyWidth={104} rows={[["resource", cur.resource_type], ["claim kind", <span className="mono t-mono-sm">{cur.claim_kind}</span>], ["subject", <span className="mono t-mono-sm">{cur.subject_ref}</span>], ["fixture", <span className="mono t-mono-sm">{cur.fixture}</span>], ["route", "Governed ingest → proposal → promotion"]]} /></Section>
            <div><button type="button" className="btn btn-s" onClick={() => navigate("Patients", cur.subject_ref)}>Open subject</button></div>
            <Authority>Holding a source is not extraction. Only fields that passed a trusted extractor appear in the patient brief.</Authority>
          </div>
        </aside>
      )}
    </>
  );
}

/* ───────────────────────────── Evidence ───────────────────────────── */
type Corpus = { corpus_id: string; version: string; source_identity: string; content_digest: string; rights: string; rights_uri: string;
  documents: Array<{ doc_id: string; text: string; content_digest: string; status: string; freshness_marker: string | null; conflict_marker: string | null }> };
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
    catch (x) { setErr(x as CmdError); }
    finally { setBusy(false); }
  }

  if (state.status === "loading") return <div className="pane"><Skeleton /></div>;
  if (state.status === "error") return <div className="pane"><ErrorState error={state.error} onRetry={reload} /></div>;
  const c = state.data;
  const docs = c.documents;
  const cur = docs.find((d) => d.doc_id === sel) ?? docs.find((d) => d.conflict_marker) ?? docs[0];

  return (
    <>
      <div className="pane">
        <PageHead title="Evidence" sub="A versioned local corpus with the provenance of every document. Static fixture, not live literature." />
        <Ledger items={[
          { k: "corpus", v: c.source_identity, mono: true },
          { k: "rights", v: <Status kind="synthetic" weight="strong">{c.rights}</Status> },
          { k: "content_digest", v: short(c.content_digest), mono: true },
          { k: "retrieval", v: "Lexical · on device" },
          { k: "verification", v: <Status kind="present" weight="strong">Digest verified at admission</Status> },
        ]} />
        <form onSubmit={search} className="field-row" style={{ marginBottom: 12 }}>
          <label className="field" style={{ flex: "1 1 260px", maxWidth: 420 }}>
            <svg width="16" height="16" viewBox="0 0 20 20" aria-hidden="true"><path d="M8.5 2.5a6 6 0 1 1 0 12 6 6 0 0 1 0-12Zm0 1.7a4.3 4.3 0 1 0 0 8.6 4.3 4.3 0 0 0 0-8.6Z M12.7 13.9 13.9 12.7 18 16.8 16.8 18Z" fill="currentColor" fillRule="evenodd" /></svg>
            <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search corpus text (lexical, local)" aria-label="Search corpus text" maxLength={256} />
          </label>
          <button type="button" className={`flt ${retracted ? "on" : ""}`} onClick={() => setRetracted((r) => !r)} aria-pressed={retracted}>Include retracted</button>
          <button type="submit" className="btn btn-s" disabled={busy}>{busy ? <><Glyph kind="running" />Searching</> : "Search"}</button>
          {result && <button type="button" className="btn btn-g" onClick={() => { setResult(null); setQuery(""); }}>Clear</button>}
        </form>
        {err && <ErrorState error={err} />}
        {result ? (
          <Section title="Retrieval matches" count={`${result.hits.length} · ${result.corpus_source_identity}`}>
            {result.hits.length === 0 ? <p className="t-sm i3">No document matched “{result.query}”.</p> : result.hits.map((h) => (
              <div key={h.doc_id} className="lr clickable" style={{ gridTemplateColumns: "170px minmax(0,1fr) 120px 70px", minHeight: "var(--row-2)" }} onClick={() => setSel(h.doc_id)}>
                <span className={`mono ${h.retracted ? "i3" : ""}`} style={{ fontSize: 12 }}>{h.doc_id}</span>
                <span className={`trunc ${h.retracted ? "i3" : ""}`} style={{ fontSize: 13 }}>{h.snippet}</span>
                <span>{h.conflict_marker ? <Status kind="needs-review" weight="strong">Conflict</Status> : h.retracted ? <Status kind="cancelled" weight="weak">Retracted</Status> : <Status kind="proposal" weight="weak">Relevance only</Status>}</span>
                <span className="mono t-mono-sm i3" style={{ textAlign: "right" }}>{h.score.toFixed(2)}</span>
              </div>
            ))}
            <p className="note" style={{ marginTop: 8 }}>Evaluation <span className="mono">{result.evaluation_id}</span>. Results are retrieval matches, not recommendations.</p>
          </Section>
        ) : (
          <Section title="Corpus" count={`${docs.length} documents`}>
            <div role="table" aria-label="Corpus documents">
              <div role="row" className="lr" style={{ gridTemplateColumns: "190px minmax(0,1fr) 150px 110px", minHeight: "var(--head)", borderBottom: "1px solid var(--line)" }}>
                {["Document", "Text · provenance", "Markers", "Status"].map((h) => <span key={h} role="columnheader" className="t-label" style={{ letterSpacing: "0.08em" }}>{h}</span>)}
              </div>
              {docs.map((d) => (
                <div role="row" key={d.doc_id} tabIndex={0} className={`lr clickable ${cur?.doc_id === d.doc_id ? "sel" : ""}`} style={{ gridTemplateColumns: "190px minmax(0,1fr) 150px 110px", minHeight: "var(--row-2)" }} onClick={() => setSel(d.doc_id)} onKeyDown={(e) => e.key === "Enter" && setSel(d.doc_id)}>
                  <span className={`mono ${d.status === "retracted" ? "i3" : ""}`} style={{ fontSize: 12 }}>{d.doc_id}</span>
                  <div style={{ display: "flex", flexDirection: "column", gap: 2, minWidth: 0, padding: "6px 0" }}>
                    <span className={`trunc ${d.status === "retracted" ? "i3" : ""}`} style={{ fontSize: 13 }}>{d.text}</span>
                    <span className="stamp trunc"><i>sha256</i>{short(d.content_digest)}{d.freshness_marker && <><span className="dot" /><i>fresh</i>{d.freshness_marker}</>}</span>
                  </div>
                  <span>{d.conflict_marker ? <Status kind="needs-review" weight="strong">Conflict</Status> : <span className="t-sm i3">None</span>}</span>
                  <Status kind={d.status === "retracted" ? "cancelled" : "available"} weight={d.status === "retracted" ? "weak" : "default"}>{d.status === "retracted" ? "Retracted" : "Active"}</Status>
                </div>
              ))}
            </div>
          </Section>
        )}
      </div>
      {cur && (
        <aside className="inspector collapsible" aria-label="Selected document">
          <div className="ins-pad">
            <div className="stack" style={{ gap: 8 }}>
              <span className="mono" style={{ fontSize: 14, fontWeight: 500 }}>{cur.doc_id}</span>
              <div className="row-gap" style={{ gap: 14 }}><Status kind={cur.status === "retracted" ? "cancelled" : "available"}>{cur.status === "retracted" ? "Retracted" : "Active"}</Status>{cur.conflict_marker && <Status kind="needs-review" weight="strong">Conflict · needs review</Status>}</div>
            </div>
            <p style={{ fontSize: 13, lineHeight: "21px", padding: "12px 14px", background: "var(--bg-surface)", border: "1px solid var(--line-subtle)", borderRadius: "var(--r-2)" }}>{cur.text}</p>
            <Section title="Provenance">
              <KV keyWidth={112} rows={[["content_digest", <span className="mono-break">{cur.content_digest}</span>], ["conflict_marker", <span className="mono t-mono-sm">{cur.conflict_marker ?? "none"}</span>], ["freshness", <span className="mono t-mono-sm">{cur.freshness_marker ?? "none"}</span>], ["corpus", <span className="mono t-mono-sm">{c.source_identity}</span>], ["rights_uri", <span className="mono-break">{c.rights_uri}</span>]]} />
            </Section>
            <Authority>{cur.conflict_marker ? "The conflict stays unresolved. MedScale does not pick a side or turn this document into a recommendation." : "Evidence is context. Relevance never becomes clinical authority."}</Authority>
          </div>
        </aside>
      )}
    </>
  );
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

export function Insights() {
  const { navigate } = useApp();
  const { state, reload } = useCommand<InsightsVm>("insights_overview");
  const [q, setQ] = useState("");
  const [answer, setAnswer] = useState<string | null>(null);
  const [err, setErr] = useState<CmdError | null>(null);

  async function ask(e: FormEvent) {
    e.preventDefault();
    if (!q.trim()) return;
    setErr(null);
    try { setAnswer(await call<string>("insights_ask", { query: q })); } catch (x) { setErr(x as CmdError); }
  }

  if (state.status === "loading") return <div className="pane"><Skeleton /></div>;
  if (state.status === "error") return <div className="pane"><ErrorState error={state.error} onRetry={reload} /></div>;
  const v = state.data;
  const max = Math.max(1, ...v.coverage_distribution.map((r) => Number(r.value) || 0));

  return (
    <>
      <div className="pane">
        <PageHead title="Insights" sub="What the trusted record supports across the cohort. Counts come from coverage only; there are no scores or predictions." />
        <Ledger items={[
          { k: "cohort_size", v: v.cohort_size, mono: true }, { k: "present_facts", v: v.present_count, mono: true },
          { k: "evidence_gaps", v: v.evidence_gap_count, mono: true }, { k: "review_attention", v: v.review_attention_count, mono: true },
          { k: "scope", v: <Status kind={v.synthetic_only ? "synthetic" : "available"} weight="strong">{v.synthetic_only ? "Synthetic only" : "Workspace"}</Status> },
        ]} />
        <Section n="01" title="Cohort" count={`${v.cohorts.length} subjects`}>
          <table className="tb"><thead><tr><th style={{ width: "22%" }}>Subject</th><th style={{ width: "16%" }}>Condition</th><th style={{ width: "40%" }}>Evidence</th><th>Review</th></tr></thead>
            <tbody>{v.cohorts.map((c) => (
              <tr key={c.subject_ref} className="clickable" tabIndex={0} onClick={() => navigate("Patients", c.subject_ref)} onKeyDown={(e) => e.key === "Enter" && navigate("Patients", c.subject_ref)}>
                <td><span className="pname trunc">{c.display_name}</span></td><td className="t-sm i2"><span className="trunc">{c.condition}</span></td>
                <td title={c.evidence_state}><Status kind={glyphFor(c.evidence_state)}>{c.evidence_state}</Status></td>
                <td><Status kind={/review/i.test(c.review_attention) ? "needs-review" : "present"} weight="weak">{c.review_attention}</Status></td>
              </tr>
            ))}</tbody></table>
        </Section>
        <Section n="02" title="Coverage distribution" count={`${v.coverage_distribution.length} statuses`}>
          <div style={{ borderTop: "1px solid var(--line-subtle)" }}>
            {v.coverage_distribution.map((r) => {
              const n = Number(r.value) || 0;
              return (
                <div key={r.label} className="lr" style={{ gridTemplateColumns: "190px 40px minmax(0,1fr) minmax(0,1fr)", minHeight: 34 }}>
                  <Status kind={glyphFor(r.label)} weight={n ? "strong" : "weak"}>{r.label}</Status>
                  <span className={`mono t-mono-sm ${n ? "" : "i3"}`} style={{ textAlign: "right" }}>{r.value}</span>
                  <span style={{ height: 8, background: "var(--bg-active)", position: "relative" }} aria-hidden="true"><span style={{ position: "absolute", inset: 0, width: `${(n / max) * 100}%`, background: n ? "var(--accent)" : "transparent" }} /></span>
                  <span className="t-sm i3">{r.detail}</span>
                </div>
              );
            })}
          </div>
        </Section>
        <Section n="03" title="Not computed" count="by design">
          <div className="grid-3">
            {[["Risk", v.risk_state], ["Trend", v.trend_state], ["Recommendation", v.recommendation_state]].map(([k, s]) => (
              <div key={k}><Status kind="unsupported">{k}</Status><p className="t-sm i3" style={{ marginTop: 4 }}>{s}</p></div>
            ))}
          </div>
        </Section>
      </div>
      <aside className="inspector collapsible" aria-label="Conditions and evidence">
        <div className="ins-pad">
          <Section title="Conditions" count="present facts only">
            {v.condition_distribution.length ? v.condition_distribution.map((r) => (
              <div key={r.label} style={{ display: "grid", gridTemplateColumns: "minmax(0,1fr) 40px", gap: 10, alignItems: "center", minHeight: 34, borderBottom: "1px solid var(--line-subtle)" }}>
                <span style={{ fontSize: 13 }} className="trunc">{r.label}</span><span className="mono t-mono-sm" style={{ textAlign: "right" }}>{r.value}</span>
              </div>
            )) : <p className="t-sm i3">No present condition facts.</p>}
          </Section>
          <Section title="Evidence in context" count={v.evidence.length}>
            {v.evidence.map((e, i) => (
              <div key={i} style={{ padding: "10px 0", borderBottom: "1px solid var(--line-subtle)" }} className="stack">
                <span style={{ fontSize: 13, lineHeight: "19px" }}>{e.snippet}</span>
                <span className="row-gap" style={{ gap: 10 }}><Stamp k="src" v={e.source} />{/RETRACTED/.test(e.metadata) && <Status kind="cancelled" weight="weak">Retracted</Status>}</span>
              </div>
            ))}
          </Section>
          <Section title="Ask about this cohort">
            <form onSubmit={ask} className="stack" style={{ gap: 8 }}>
              <input type="text" value={q} onChange={(e) => setQ(e.target.value)} placeholder="e.g. coverage gaps" maxLength={256} aria-label="Question about the cohort" />
              <div><button type="submit" className="btn btn-s">Ask</button></div>
            </form>
            {err && <ErrorState error={err} />}
            <p className="t-sm i2" style={{ marginTop: 10 }}>{answer ?? v.assistant_default}</p>
          </Section>
          <Authority>Evidence-context only. Answers come from the trusted projection; they are not clinical advice.</Authority>
        </div>
      </aside>
    </>
  );
}
