import { useEffect, useMemo, useState, type FormEvent } from "react";
import { useApp } from "../lib/app";
import { useAction, useCommand } from "../lib/ipc";
import { ConfirmPanel } from "../components/confirm";
import { MOD } from "../components/shell";
import { ProjectScope, type ProjectRow } from "../components/project";
import {
  Authority, CopyButton, Empty, ErrorState, Glyph, KV, Ledger, LoadError, Notice, PageHead,
  ResizableSplit, Section, SegmentedBar, Skeleton, Stamp, Status, glyphFor, useOutcome, useToast,
} from "../components/ui";

/* ───────────────────────────── Projects ───────────────────────────── */
type ProjectDetail = {
  project_id: string; name: string; status: string; revision: number; experiment_count: number; active_ref_count: number; active_edge_count: number;
  experiments: Array<{ id: string; name: string; status: string; revision: number }>;
  refs: Array<{ ref_id: string; kind: string; object_id: string; resolution: string }>;
  edges: Array<{ edge_id: string; subject: string; predicate: string; object: string; revision: number }>;
};

export function Projects() {
  const { project, setProject } = useApp();
  const list = useCommand<ProjectRow[]>("projects_list");
  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");
  const [exp, setExp] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const [confirmArchive, setConfirmArchive] = useState(false);
  const { pending, run } = useAction();
  const outcome = useOutcome();
  const rows = list.state.status === "ready" ? list.state.data.filter((p) => showArchived || p.status !== "archived") : [];
  const current = rows.find((p) => p.id === project) ?? rows[0] ?? null;
  const detail = useCommand<ProjectDetail>(current ? "project_detail" : null, current ? { projectId: current.id } : undefined);

  async function create(e: FormEvent) {
    e.preventDefault();
    const r = await run<ProjectRow>("create", "project_create", { name, description: desc || null });
    if (outcome(r, `Project “${name.trim()}” created`)) { setName(""); setDesc(""); setProject(r.data.id); await list.reload(); }
  }
  async function archive() {
    if (!current) return;
    const r = await run("archive", "project_archive", { projectId: current.id, revision: current.revision });
    setConfirmArchive(false);
    if (outcome(r, "Project archived")) await list.reload();
  }
  async function addExperiment(e: FormEvent) {
    e.preventDefault();
    if (!current) return;
    const r = await run("exp", "experiment_create", { projectId: current.id, name: exp });
    if (outcome(r, "Experiment created")) { setExp(""); await detail.reload(); await list.reload(); }
  }

  const pane = (
    <div className="pane">
      <PageHead title="Projects" sub="Versioned research containers: experiments, pinned vault references, and provenance relationships.">
        <button type="button" className={`flt ${showArchived ? "on" : ""}`} onClick={() => setShowArchived((s) => !s)} aria-pressed={showArchived}>Show archived</button>
      </PageHead>
      {list.state.status === "loading" && <Skeleton />}
      {list.state.status === "error" && <ErrorState error={list.state.error} onRetry={list.reload} />}
      {list.state.status === "ready" && (rows.length ? (
        <table className="tb" aria-label="Projects">
          <thead>
            <tr>
              <th style={{ width: "34%" }}>Project</th>
              <th style={{ width: "16%" }}>Status</th>
              <th className="num" style={{ width: "11%" }} title="Revision">Rev</th>
              <th className="num col-p3" style={{ width: "13%" }} title="Experiments">Exp</th>
              <th className="num col-p3" style={{ width: "13%" }} title="References">Refs</th>
              <th className="num" style={{ width: "13%" }}>Edges</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((p) => (
              <tr key={p.id} className={`r2 clickable ${current?.id === p.id ? "sel" : ""}`} tabIndex={0} aria-label={`Select project ${p.name}`} aria-selected={current?.id === p.id} onClick={() => { setProject(p.id); setConfirmArchive(false); }} onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setProject(p.id); setConfirmArchive(false); } }}>
                <td><div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="pname trunc">{p.name}</span><span className="mono t-mono-sm i3 trunc">{p.id}</span></div></td>
                <td><Status kind={glyphFor(p.status)}>{p.status}</Status></td>
                <td className="num mono t-mono-sm tnum">{p.revision}</td>
                <td className="num mono t-mono-sm col-p3 tnum">{p.experiments}</td>
                <td className="num mono t-mono-sm col-p3 tnum">{p.refs}</td>
                <td className="num mono t-mono-sm tnum">{p.edges}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <Empty icon="projects" title="No projects in this workspace">
          A project pins vault objects at an exact revision so experiments, queries, and audio transcripts stay reproducible. Create one in the panel on the right.
        </Empty>
      ))}

      {current && detail.state.status === "loading" && <Skeleton rows={3} />}
      {current && detail.state.status === "error" && <ErrorState error={detail.state.error} onRetry={detail.reload} />}
      {current && detail.state.status === "ready" && (
        <Section title="Project details" count={detail.state.data.name}>
          <Ledger items={[
            { k: "project_id", v: <span className="copy-wrap">{detail.state.data.project_id} <CopyButton text={detail.state.data.project_id} ariaLabel="Copy project ID" /></span>, mono: true },
            { k: "status", v: detail.state.data.status },
            { k: "revision", v: detail.state.data.revision, mono: true },
            { k: "experiments", v: detail.state.data.experiment_count, mono: true },
            { k: "refs", v: detail.state.data.active_ref_count, mono: true },
            { k: "edges", v: detail.state.data.active_edge_count, mono: true },
          ]} />
          <div className="stack" style={{ gap: 18 }}>
            <div>
              <div className="t-h3" style={{ marginBottom: 8 }}>Experiments</div>
              {detail.state.data.experiments.length
                ? detail.state.data.experiments.map((x) => (
                    <div key={x.id} style={{ display: "flex", justifyContent: "space-between", minHeight: 32, borderBottom: "1px solid var(--line-subtle)", alignItems: "center", gap: 8 }}>
                      <span className="t-sm trunc">{x.name}</span>
                      <Status kind={glyphFor(x.status)} weight="weak">{x.status}</Status>
                    </div>
                  ))
                : <p className="t-sm i3">None yet.</p>}
              <form onSubmit={addExperiment} className="field-row" style={{ marginTop: 10, maxWidth: 420 }}>
                <input type="text" value={exp} onChange={(e) => setExp(e.target.value)} placeholder="Experiment name" maxLength={128} aria-label="Experiment name" style={{ flex: 1 }} />
                <button type="submit" className="btn btn-s" disabled={!exp.trim() || !!pending}>Add</button>
              </form>
            </div>
            <div className="grid-2">
              <div>
                <div className="t-h3" style={{ marginBottom: 8 }}>References</div>
                {detail.state.data.refs.length
                  ? detail.state.data.refs.map((r) => (
                      <div key={r.ref_id} style={{ minHeight: 32, borderBottom: "1px solid var(--line-subtle)", display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8 }}>
                        <Stamp k={r.kind} v={r.object_id} />
                        <Status kind={glyphFor(r.resolution)} weight="weak">{r.resolution}</Status>
                      </div>
                    ))
                  : <p className="t-sm i3">No vault objects pinned.</p>}
              </div>
              <div>
                <div className="t-h3" style={{ marginBottom: 8 }}>Graph</div>
                {detail.state.data.edges.length
                  ? detail.state.data.edges.map((g) => (
                      <div key={g.edge_id} className="t-sm i2" style={{ minHeight: 32, borderBottom: "1px solid var(--line-subtle)", display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
                        <span>{g.subject}</span>
                        <span className="mono t-mono-sm i3">{g.predicate}</span>
                        <span>{g.object}</span>
                      </div>
                    ))
                  : <p className="t-sm i3">No provenance edges yet.</p>}
              </div>
            </div>
          </div>
        </Section>
      )}
    </div>
  );

  const inspector = (
    <form onSubmit={create} className="ins-pad" style={{ gap: 16 }}>
      <div className="sec" style={{ margin: 0 }}><span className="sec-t">New project</span><span className="sec-rule" /></div>
      <div>
        <label className="lbl" htmlFor="pn">Name</label>
        <input id="pn" type="text" value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Coverage audit" maxLength={128} style={{ width: "100%" }} />
        <span className="t-sm i3" style={{ display: "block", marginTop: 4 }}>Up to 128 characters.</span>
      </div>
      <div>
        <label className="lbl" htmlFor="pd">Description</label>
        <textarea id="pd" value={desc} onChange={(e) => setDesc(e.target.value)} maxLength={512} placeholder="Optional scope or hypothesis" style={{ width: "100%", minHeight: 68 }} />
      </div>
      <div className="row-gap" style={{ gap: 8 }}>
        <button type="submit" className="btn btn-p" disabled={!name.trim() || !!pending}>{pending === "create" ? <><Glyph kind="running" />Creating</> : "Create project"}</button>
      </div>
      {current && current.status !== "archived" && (
        <>
          <div className="sec" style={{ margin: "8px 0 0" }}><span className="sec-t">Selected project</span><span className="sec-rule" /></div>
          {!confirmArchive ? (
            <button type="button" className="btn btn-s" onClick={() => setConfirmArchive(true)} disabled={!!pending}>Archive &ldquo;{current.name}&rdquo;…</button>
          ) : (
            <ConfirmPanel
              title={`Archive “${current.name}”?`}
              confirmLabel="Archive project"
              busy={pending === "archive"}
              destructive
              onConfirm={archive}
              onCancel={() => setConfirmArchive(false)}
            >
              Archiving keeps every saved revision in history, and hides the project from active pickers unless &ldquo;Show archived&rdquo; is toggled.
            </ConfirmPanel>
          )}
          <p className="t-sm i3">Archiving preserves all prior revisions and refuses stale updates if the project changed on disk.</p>
        </>
      )}
      <Notice kind="limited" title="Versioned writes.">Every change checks the current vault revision before saving; if anything conflicts, nothing is written.</Notice>
    </form>
  );

  return <ResizableSplit pane={pane} inspector={inspector} inspectorLabel="New project" />;
}

/* ───────────────────────────── Data ───────────────────────────── */
type Source = { id: string; name: string; kind: string; health: string; revision: number };
type Snapshot = { id: string; rows: number; digest: string };
type Page = { snapshot_id: string; schema_line: string; rows: string[]; next_cursor: string | null; detail: string };

/** Parses delimited or pipe/comma snapshot lines into a clean table when uniform. */
function parseStructuredRows(schemaLine: string, rowLines: string[]): { headers: string[]; cells: string[][] } | null {
  if (!rowLines.length) return null;
  const sep = rowLines[0]!.includes("|") ? "|" : rowLines[0]!.includes("\t") ? "\t" : rowLines[0]!.includes(",") ? "," : null;
  if (!sep) return null;
  const splitLine = (l: string) => l.split(sep).map((x) => x.trim()).filter((_, idx, arr) => !(sep === "|" && (idx === 0 || idx === arr.length - 1) && arr[idx] === ""));
  const parsed = rowLines.map(splitLine);
  const colCount = parsed[0]?.length ?? 0;
  if (colCount < 2 || !parsed.every((r) => r.length === colCount)) return null;
  const schemaCols = schemaLine.split(/[,|]/).map((s) => s.trim().split(/[:\s]/)[0] ?? "").filter(Boolean);
  const headers = schemaCols.length === colCount ? schemaCols : Array.from({ length: colCount }, (_, i) => `col_${i + 1}`);
  return { headers, cells: parsed };
}

export function Data() {
  return <ProjectScope title="Data" sub="Local datasets and pinned snapshots. Rows are read from an immutable snapshot, never from a live file.">{(pid) => <DataBody projectId={pid} />}</ProjectScope>;
}

function DataBody({ projectId }: { projectId: string }) {
  const { navigate } = useApp();
  const sources = useCommand<Source[]>("data_sources", { projectId });
  const [source, setSource] = useState<string | null>(null);
  const [snap, setSnap] = useState<string | null>(null);
  const [rawView, setRawView] = useState(false);
  const snaps = useCommand<Snapshot[]>(source ? "data_snapshots" : null, source ? { sourceId: source } : undefined);
  const page = useCommand<Page>(snap ? "data_page" : null, snap ? { snapshotId: snap } : undefined);
  const { pending, run } = useAction();
  const outcome = useOutcome();
  const [cols, setCols] = useState(""); const [fc, setFc] = useState(""); const [fo, setFo] = useState("equals"); const [fv, setFv] = useState(""); const [sort, setSort] = useState("");

  useEffect(() => { if (sources.state.status === "ready" && !source && sources.state.data[0]) setSource(sources.state.data[0].id); }, [sources.state, source]);
  useEffect(() => { if (snaps.state.status === "ready") setSnap(snaps.state.data.at(-1)?.id ?? null); }, [snaps.state]);

  async function addSample() {
    const r = await run<{ source_id: string }>("sample", "data_add_sample_source", { projectId });
    if (outcome(r, "Synthetic sample source created")) { await sources.reload(); setSource(r.data.source_id); }
  }
  async function importSnap() {
    if (!source) return;
    const r = await run("import", "data_import_snapshot", { sourceId: source });
    if (outcome(r, "Snapshot imported")) await snaps.reload();
  }
  async function transform(e: FormEvent) {
    e.preventDefault();
    if (!snap) return;
    const r = await run<string>("transform", "data_transform", { snapshotId: snap, columns: cols.trim() ? cols.split(",").map((c) => c.trim()).filter(Boolean) : null, filterColumn: fc || null, filterOp: fc ? fo : null, filterValue: fc ? fv : null, sort: sort || null });
    if (outcome(r, "Transform applied")) await snaps.reload();
  }

  const structured = useMemo(
    () => (page.state.status === "ready" ? parseStructuredRows(page.state.data.schema_line, page.state.data.rows) : null),
    [page.state],
  );
  const maxSnapRows = snaps.state.status === "ready" ? Math.max(1, ...snaps.state.data.map((s) => s.rows)) : 1;

  return (
    <>
      <div className="row-gap" style={{ gap: 8, marginBottom: 14 }}>
        <button type="button" className="btn btn-s" onClick={addSample} disabled={!!pending}>{pending === "sample" ? <Glyph kind="running" /> : null}Add synthetic sample source</button>
        <button type="button" className="btn btn-s" onClick={importSnap} disabled={!source || !!pending}>Import snapshot</button>
        {snap && <button type="button" className="btn btn-g" onClick={() => navigate("Analytics")}>Query in Analytics</button>}
        <span className="t-sm i3" style={{ marginLeft: "auto" }}>Read-only snapshots</span>
      </div>
      <div className="wb-side">
        <div>
          <Section title="Sources" count={sources.state.status === "ready" ? sources.state.data.length : "…"}>
            {sources.state.status === "loading" && <Skeleton rows={2} />}
            {sources.state.status === "error" && <ErrorState error={sources.state.error} onRetry={sources.reload} />}
            {sources.state.status === "ready" && !sources.state.data.length && <p className="t-sm i3">No data sources yet. Add the synthetic sample to explore snapshots.</p>}
            {sources.state.status === "ready" && sources.state.data.map((s) => (
              <button type="button" key={s.id} className={`lr clickable ${source === s.id ? "sel" : ""}`} style={{ gridTemplateColumns: "minmax(0,1fr) auto", width: "100%", textAlign: "left" }} onClick={() => { setSource(s.id); setSnap(null); }}>
                <span style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="trunc" style={{ fontWeight: 600 }}>{s.name}</span><span className="mono t-mono-sm i3 trunc">{s.kind} · r{s.revision}</span></span>
                <Status kind={glyphFor(s.health)} weight="weak">{s.health}</Status>
              </button>
            ))}
          </Section>
          <LoadError cmd={snaps} />
          {snaps.state.status === "loading" && <Skeleton rows={2} />}
          {snaps.state.status === "ready" && (
            <Section title="Snapshots" count={snaps.state.data.length}>
              {snaps.state.data.length ? snaps.state.data.map((s) => (
                <button type="button" key={s.id} className={`lr clickable ${snap === s.id ? "sel" : ""}`} style={{ gridTemplateColumns: "minmax(0,1fr)", width: "100%", textAlign: "left", paddingBlock: 7 }} onClick={() => setSnap(s.id)}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", gap: 8 }}>
                    <span className="mono t-mono-sm trunc">{s.id}</span>
                    <span className="mono t-mono-sm i3 tnum">{s.rows} rows</span>
                  </div>
                  <div style={{ height: 3, background: "var(--bg-active)", borderRadius: 2, overflow: "hidden", marginTop: 4 }} aria-hidden="true">
                    <div style={{ height: "100%", width: `${Math.round((s.rows / maxSnapRows) * 100)}%`, background: snap === s.id ? "var(--accent)" : "var(--ink-3)" }} />
                  </div>
                </button>
              )) : <p className="t-sm i3">No snapshots yet. Click Import snapshot.</p>}
            </Section>
          )}
        </div>
        <div>
          {!snap && <Empty icon="data" title="No snapshot selected">Rows are read from a pinned snapshot so every analysis and chart can be reproduced from its snapshot ID.</Empty>}
          {snap && page.state.status === "loading" && <Skeleton />}
          {snap && page.state.status === "error" && <ErrorState error={page.state.error} onRetry={page.reload} />}
          {snap && page.state.status === "ready" && <>
            <Ledger items={[
              { k: "snapshot_id", v: <span className="copy-wrap">{page.state.data.snapshot_id} <CopyButton text={page.state.data.snapshot_id} ariaLabel="Copy snapshot ID" /></span>, mono: true },
              { k: "schema", v: page.state.data.schema_line, mono: true },
              { k: "rows on page", v: page.state.data.rows.length, mono: true },
            ]} />
            {structured && !rawView ? (
              <div>
                <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 6 }}>
                  <button type="button" className="btn btn-g" onClick={() => setRawView(true)}>Raw text</button>
                </div>
                <div className="data-table-wrap">
                  <table className="tb" aria-label="Snapshot rows">
                    <thead><tr>{structured.headers.map((h, i) => <th key={i}>{h}</th>)}</tr></thead>
                    <tbody>
                      {structured.cells.map((row, ri) => (
                        <tr key={ri}>{row.map((cell, ci) => <td key={ci} title={cell}><span className="trunc">{cell}</span></td>)}</tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            ) : (
              <div>
                {structured && (
                  <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 6 }}>
                    <button type="button" className="btn btn-g" onClick={() => setRawView(false)}>Table view</button>
                  </div>
                )}
                <pre className="pre" aria-label="Snapshot rows">{page.state.data.rows.join("\n") || "No rows."}</pre>
              </div>
            )}
            <p className="note" style={{ marginTop: 6 }}>{page.state.data.detail}{page.state.data.next_cursor ? ` · next cursor ${page.state.data.next_cursor}` : ""}</p>
            <Section title="Transform" count="creates a new snapshot">
              <form onSubmit={transform} className="stack" style={{ gap: 10 }}>
                <div className="field-row"><label htmlFor="tf-cols" className="t-sm i3" style={{ width: 70 }}>Columns</label><input id="tf-cols" type="text" value={cols} onChange={(e) => setCols(e.target.value)} placeholder="comma-separated, empty = all" style={{ flex: 1 }} /></div>
                <div className="field-row">
                  <label htmlFor="tf-filter" className="t-sm i3" style={{ width: 70 }}>Filter</label>
                  <input id="tf-filter" type="text" value={fc} onChange={(e) => setFc(e.target.value)} placeholder="column" style={{ width: 140 }} />
                  <select value={fo} onChange={(e) => setFo(e.target.value)} aria-label="Filter operator"><option value="equals">equals</option><option value="contains">contains</option><option value="regex">regex</option></select>
                  <input type="text" value={fv} onChange={(e) => setFv(e.target.value)} placeholder="value" aria-label="Filter value" style={{ flex: 1 }} />
                </div>
                <div className="field-row"><label htmlFor="tf-sort" className="t-sm i3" style={{ width: 70 }}>Sort</label><input id="tf-sort" type="text" value={sort} onChange={(e) => setSort(e.target.value)} placeholder="column:asc or column:desc" style={{ flex: 1 }} /></div>
                <div><button type="submit" className="btn btn-s" disabled={!!pending}>Apply transform</button></div>
              </form>
            </Section>
          </>}
        </div>
      </div>
    </>
  );
}

/* ───────────────────────────── Analytics ───────────────────────────── */
export function Analytics() {
  return <ProjectScope title="Analytics" sub="Read-only SQL queries over pinned snapshots, run locally. Every execution leaves an audit receipt.">{(pid) => <AnalyticsBody projectId={pid} />}</ProjectScope>;
}

function AnalyticsBody({ projectId }: { projectId: string }) {
  const receipts = useCommand<Array<{ id: string; outcome: string; detail: string }>>("analytics_receipts", { projectId });
  const [sql, setSql] = useState("SELECT * FROM t");
  const [bindings, setBindings] = useState("");
  const [result, setResult] = useState<{ summary: string; preview: string } | null>(null);
  const { pending, run } = useAction();
  const outcome = useOutcome();
  const toast = useToast();

  async function exec(e?: FormEvent) {
    e?.preventDefault();
    if (!sql.trim() || pending) return;
    const r = await run<{ summary: string; preview: string }>("run", "analytics_query", { projectId, sql, bindings });
    if (r.ok && /^(denied|failed)/i.test(r.data.summary)) { toast({ kind: "blocked", title: "Denied by read-only guard", detail: r.data.summary }); setResult(r.data); }
    else if (outcome(r, "Query ran")) setResult(r.data); else setResult(null);
    await receipts.reload();
  }

  const allowedReceipts = receipts.state.status === "ready" ? receipts.state.data.filter((r) => !/denied|fail/i.test(r.outcome)).length : 0;
  const deniedReceipts = receipts.state.status === "ready" ? receipts.state.data.length - allowedReceipts : 0;

  return (
    <>
      <form onSubmit={exec} style={{ border: "1px solid var(--line)", borderRadius: "var(--r-2)", overflow: "hidden" }}>
        <div className="field-row" style={{ minHeight: 48, padding: "6px 10px 6px 12px", borderBottom: "1px solid var(--line-subtle)", background: "var(--bg-raised)", gap: 10 }}>
          <span className="t-label">Bindings</span>
          <input type="text" value={bindings} onChange={(e) => setBindings(e.target.value)} placeholder="alias=snapshot_id (e.g. t=snap-…)" aria-label="Bindings" className="mono" style={{ flex: "1 1 220px" }} />
          <Status kind="limited" weight="weak">Read-only · writes denied</Status>
          <button type="submit" className="btn btn-p btn-inline" disabled={!!pending || !sql.trim()}>{pending ? <Glyph kind="running" /> : null}Run</button>
        </div>
        <textarea
          className="mono"
          value={sql}
          onChange={(e) => setSql(e.target.value)}
          onKeyDown={(e) => { if ((e.ctrlKey || e.metaKey) && e.key === "Enter") { e.preventDefault(); void exec(); } }}
          aria-label="SQL"
          style={{ width: "100%", border: 0, borderRadius: 0, minHeight: 150, background: "var(--bg-surface)" }}
        />
      </form>
      <p className="note" style={{ marginTop: 6 }}>Bind each table alias to a snapshot ID from Data (press {MOD}+Enter to run). Write statements are rejected as read-only (<span className="mono">not_read_only</span>).</p>
      <Section title="Result">
        {result ? (
          <>
            <p className="t-sm" style={{ marginBottom: 8 }}>{result.summary}</p>
            <pre className="pre">{result.preview || "No rows."}</pre>
          </>
        ) : <p className="t-sm i3">No result yet. Results show a bounded preview and the exact snapshot they came from.</p>}
      </Section>
      <Section title="Receipts" count={receipts.state.status === "ready" ? receipts.state.data.length : "…"}>
        {receipts.state.status === "loading" && <Skeleton rows={2} />}
        {receipts.state.status === "error" && <ErrorState error={receipts.state.error} onRetry={receipts.reload} />}
        {receipts.state.status === "ready" && receipts.state.data.length > 0 && (
          <div style={{ marginBottom: 12 }}>
            <SegmentedBar
              title="Query receipt outcomes"
              meta={`${receipts.state.data.length} runs`}
              items={[
                { key: "ok", label: "Completed", count: allowedReceipts, tone: "present" },
                { key: "denied", label: "Denied / failed", count: deniedReceipts, tone: "danger" },
              ]}
            />
          </div>
        )}
        {receipts.state.status === "ready" && (receipts.state.data.length ? (
          <table className="tb">
            <thead><tr><th style={{ width: "32%" }}>Receipt</th><th style={{ width: "18%" }}>Outcome</th><th>Detail</th></tr></thead>
            <tbody>
              {receipts.state.data.map((r) => (
                <tr key={r.id}>
                  <td><span className="mono t-mono-sm trunc">{r.id}</span></td>
                  <td><Status kind={glyphFor(r.outcome)}>{r.outcome}</Status></td>
                  <td className="t-sm i2" title={r.detail}><span className="trunc">{r.detail}</span></td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : <p className="t-sm i3">No runs recorded in this project.</p>)}
      </Section>
      <Authority>Query results describe a snapshot. They are not clinical findings and are never written back to any record.</Authority>
    </>
  );
}

/* ───────────────────────────── Knowledge ───────────────────────────── */
export function Knowledge() {
  return <ProjectScope title="Knowledge" sub="Search this project's sources with a local index and organize notes with live citations on a Research Canvas.">{(pid) => <KnowledgeBody projectId={pid} />}</ProjectScope>;
}

type Citable = { receipt_id: string; hits: Array<{ rank: number; freshness: string }> };
type SearchResult = { summary: string; hits: string; citable?: Citable };
type CanvasNodeVm = { key: string; content: { note?: { text: string }; evidence?: { span: unknown } } | Record<string, unknown> };
type CanvasViewVm = {
  canvas: { header: { id: string }; revision: number; title: string; nodes: CanvasNodeVm[]; edges: Array<{ from: string; to: string; relation: string }> };
  resolutions: Array<{ key: string; freshness: string; text: string | null }>;
  contradictions: Array<{ from: string; to: string }>;
  unsupported_notes: string[];
  superseded_evidence: string[];
  missing_evidence: string[];
  overlapping_evidence: Array<{ kind: string; first: string; second: string }>;
  evidence_coverage: Array<{ note: string; evidence_class: string; state: string; source_count: number }>;
  coverage_hash: string;
};
const RELATIONS = ["supports", "contradicts", "relates", "questions"] as const;

/** A node's text: the note, or the cited span read live from its source. */
function nodeText(v: CanvasViewVm, n: CanvasNodeVm): string {
  const c = n.content as Record<string, { text?: string }>;
  if (c.note?.text) return c.note.text;
  const r = v.resolutions.find((x) => x.key === n.key);
  return r?.text ?? "Source no longer readable";
}
const isNote = (n: CanvasNodeVm) => "note" in (n.content as Record<string, unknown>);

/** Deterministic bipartite SVG argument map of real Research Canvas notes, cited evidence, and edges. */
function CanvasArgumentMap({ view }: { view: CanvasViewVm }) {
  const nodes = view.canvas.nodes;
  const edges = view.canvas.edges;
  if (!nodes.length) return null;
  const notes = nodes.filter(isNote);
  const evs = nodes.filter((n) => !isNote(n));
  const colRows = Math.max(notes.length, evs.length, 1);
  const height = Math.max(96, colRows * 46 + 24);
  const pos = new Map<string, { x: number; y: number }>();
  notes.forEach((n, i) => pos.set(n.key, { x: 110, y: 28 + i * 46 }));
  evs.forEach((n, i) => pos.set(n.key, { x: 470, y: 28 + i * 46 }));
  const unsupported = new Set(view.unsupported_notes);

  return (
    <div className="viz-frame" style={{ marginBottom: 14 }}>
      <div className="viz-head">
        <span className="viz-title">Canvas argument structure</span>
        <span className="viz-meta">{notes.length} note{notes.length === 1 ? "" : "s"} · {evs.length} citation{evs.length === 1 ? "" : "s"} · {edges.length} link{edges.length === 1 ? "" : "s"}</span>
      </div>
      <svg viewBox={`0 0 580 ${height}`} style={{ width: "100%", height: "auto", display: "block" }} role="img" aria-label={`Research canvas map with ${notes.length} notes, ${evs.length} evidence citations, and ${edges.length} links`}>
        {edges.map((e, i) => {
          const a = pos.get(e.from);
          const b = pos.get(e.to);
          if (!a || !b) return null;
          const color = e.relation === "contradicts" ? "var(--signal)" : e.relation === "supports" ? "var(--accent)" : "var(--ink-3)";
          const sameCol = a.x === b.x;
          let pathD: string;
          let labelX: number;
          const labelY = (a.y + b.y) / 2 - 4;
          if (sameCol) {
            const edgeX = a.x < 290 ? a.x + 95 : a.x - 95;
            const bowX = a.x < 290 ? edgeX + 42 : edgeX - 42;
            pathD = `M${edgeX} ${a.y} C${bowX} ${a.y}, ${bowX} ${b.y}, ${edgeX} ${b.y}`;
            labelX = bowX;
          } else {
            const x1 = a.x < b.x ? a.x + 95 : a.x - 95;
            const x2 = a.x < b.x ? b.x - 95 : b.x + 95;
            const mx = (x1 + x2) / 2;
            pathD = `M${x1} ${a.y} C${mx} ${a.y}, ${mx} ${b.y}, ${x2} ${b.y}`;
            labelX = mx;
          }
          return (
            <g key={i}>
              <path d={pathD} fill="none" stroke={color} strokeWidth="1.5" strokeDasharray={e.relation === "questions" ? "4 3" : undefined} />
              <text x={labelX} y={labelY} textAnchor="middle" fill="var(--ink-3)" fontSize="9.5" fontFamily="var(--font-mono)">{e.relation}</text>
            </g>
          );
        })}
        {nodes.map((n) => {
          const p = pos.get(n.key);
          if (!p) return null;
          const noteNode = isNote(n);
          const isUnsup = unsupported.has(n.key);
          const stroke = isUnsup ? "var(--signal)" : noteNode ? "var(--ink-2)" : "var(--accent)";
          const rawText = nodeText(view, n);
          const keyChars = n.key.length;
          const maxPreview = Math.max(10, 24 - keyChars);
          const preview = rawText.slice(0, maxPreview) + (rawText.length > maxPreview ? "…" : "");
          return (
            <g key={n.key} transform={`translate(${p.x - 95}, ${p.y - 14})`}>
              <rect width="190" height="28" rx="4" fill="var(--bg-surface)" stroke={stroke} strokeWidth="1.2" />
              <text x="8" y="17" fontSize="10.5" fontFamily="var(--font-ui)">
                <tspan fill="var(--ink-3)" fontFamily="var(--font-mono)" fontSize="10">{n.key}</tspan>
                <tspan fill="var(--ink-1)" dx="6">{preview}</tspan>
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

function KnowledgeBody({ projectId }: { projectId: string }) {
  const overview = useCommand<{ index_status: string; canvases: Array<{ id: string; title: string; detail: string }> }>("knowledge_overview", { projectId });
  const [q, setQ] = useState("");
  const [res, setRes] = useState<SearchResult | null>(null);
  const [canvas, setCanvas] = useState<CanvasViewVm | null>(null);
  const [newTitle, setNewTitle] = useState("");
  const [note, setNote] = useState("");
  const [link, setLink] = useState<{ from: string; to: string; relation: string }>({ from: "", to: "", relation: "supports" });
  const { pending, run } = useAction();
  const outcome = useOutcome();

  async function build() { const r = await run<string>("build", "knowledge_build", { projectId }); if (outcome(r, "Index built")) await overview.reload(); }
  async function search(e: FormEvent) { e.preventDefault(); const r = await run<SearchResult>("search", "knowledge_search", { projectId, query: q }); if (outcome(r, "Search complete")) setRes(r.data); }
  async function apply(label: string, cmd: string, args: Record<string, unknown>, done: string) {
    const r = await run<CanvasViewVm>(label, cmd, args);
    if (outcome(r, done)) { setCanvas(r.data); await overview.reload(); return true; }
    if (canvas) { const fresh = await run<CanvasViewVm>("canvas", "canvas_view", { canvasId: canvas.canvas.header.id }); if (fresh.ok) setCanvas(fresh.data); }
    return false;
  }
  async function openCanvas(id: string) { const r = await run<CanvasViewVm>("canvas", "canvas_view", { canvasId: id }); if (r.ok) setCanvas(r.data); else outcome(r, ""); }
  async function create(e: FormEvent) { e.preventDefault(); if (await apply("create", "canvas_create", { projectId, title: newTitle }, "Canvas created")) setNewTitle(""); }
  const at = () => ({ canvasId: canvas!.canvas.header.id, revision: canvas!.canvas.revision });
  async function addNote(e: FormEvent) { e.preventDefault(); if (canvas && await apply("note", "canvas_add_note", { ...at(), text: note }, "Note added")) setNote(""); }
  async function cite(rank: number) { if (canvas && res?.citable) await apply("cite", "canvas_cite", { ...at(), receiptId: res.citable.receipt_id, rank }, `Result ${rank} cited`); }
  async function addLink(e: FormEvent) { e.preventDefault(); if (canvas) await apply("link", "canvas_link", { ...at(), ...link }, "Linked"); }
  async function remove(key: string) { if (canvas) await apply("remove", "canvas_remove", { ...at(), key }, "Removed"); }

  const nodes = canvas?.canvas.nodes ?? [];
  const optionLabel = (n: CanvasNodeVm) => {
    const txt = canvas ? nodeText(canvas, n) : "";
    return `${n.key} — ${txt.slice(0, 32)}${txt.length > 32 ? "…" : ""}`;
  };

  return (
    <>
      {overview.state.status === "error" && <ErrorState error={overview.state.error} onRetry={overview.reload} />}
      <div className="knowledge-layout">
        <div>
          <form onSubmit={search} className="field-row knowledge-search">
            <input type="text" value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search indexed sources" maxLength={256} aria-label="Search indexed sources" data-page-search="true" style={{ flex: "1 1 240px" }} />
            <button type="submit" className="btn btn-p" disabled={!q.trim() || !!pending}>Search</button>
          </form>
          <Section title="Results">
            {res ? <>
              <p className="t-sm" style={{ marginBottom: 8 }}>{res.summary}</p>
              <pre className="pre">{res.hits || "No hits."}</pre>
              {res.citable && res.citable.hits.length > 0 && (
                <div className="row-gap" style={{ gap: 6, marginTop: 8, flexWrap: "wrap" }}>
                  <span className="t-sm i3">{canvas ? `Cite on “${canvas.canvas.title}”:` : "Open or create a canvas to cite a result."}</span>
                  {canvas && res.citable.hits.map((h) => (
                    <button key={h.rank} type="button" className="btn btn-s" onClick={() => void cite(h.rank)} disabled={!!pending} aria-label={`Cite result ${h.rank} on the canvas`}>Cite {h.rank}</button>
                  ))}
                </div>
              )}
            </> : <p className="t-sm i3">Build the local index on the right, then search this project&apos;s sources.</p>}
          </Section>
          {canvas && (
            <Section title={canvas.canvas.title} count={`revision ${canvas.canvas.revision}`}>
              <CanvasArgumentMap view={canvas} />
              {nodes.length === 0 && <p className="t-sm i3">Add a note below, then cite search results that support or contradict it.</p>}
              {nodes.map((n) => {
                const r = canvas.resolutions.find((x) => x.key === n.key);
                return (
                  <div key={n.key} className="lr" style={{ gridTemplateColumns: "40px minmax(0,1fr) auto auto", alignItems: "start", padding: "8px 0" }}>
                    <span className="mono t-mono-sm i3">{n.key}</span>
                    <span className="t-sm wrap-any">{nodeText(canvas, n)}</span>
                    {isNote(n) ? <Status kind="pending" weight="weak">Note</Status> : <Status kind={r?.freshness === "current" ? "available" : "blocked"} weight="weak">{r?.freshness ?? "Evidence"}</Status>}
                    <button type="button" className="btn btn-g" onClick={() => void remove(n.key)} disabled={!!pending} aria-label={`Remove ${n.key}`}>Remove</button>
                  </div>
                );
              })}
              {canvas.canvas.edges.length > 0 && <KV keyWidth={120} rows={canvas.canvas.edges.map((e) => [`${e.from} → ${e.to}`, e.relation])} />}
              <form onSubmit={addNote} className="field-row" style={{ marginTop: 12 }}>
                <input id="canvas-note" type="text" value={note} onChange={(e) => setNote(e.target.value)} placeholder="Write a research note or claim…" maxLength={4000} aria-label="New note" style={{ flex: "1 1 240px" }} />
                <button type="submit" className="btn btn-s" disabled={!note.trim() || !!pending}>Add note</button>
              </form>
              {nodes.length >= 2 && (
                <form onSubmit={addLink} className="field-row" style={{ marginTop: 8 }}>
                  <select aria-label="Link from" value={link.from} onChange={(e) => setLink({ ...link, from: e.target.value })}><option value="">From…</option>{nodes.map((n) => <option key={n.key} value={n.key}>{optionLabel(n)}</option>)}</select>
                  <select aria-label="Relation" value={link.relation} onChange={(e) => setLink({ ...link, relation: e.target.value })}>{RELATIONS.map((x) => <option key={x} value={x}>{x}</option>)}</select>
                  <select aria-label="Link to" value={link.to} onChange={(e) => setLink({ ...link, to: e.target.value })}><option value="">To…</option>{nodes.map((n) => <option key={n.key} value={n.key}>{optionLabel(n)}</option>)}</select>
                  <button type="submit" className="btn btn-s" disabled={!link.from || !link.to || link.from === link.to || !!pending}>Link</button>
                </form>
              )}
              <div className="stack" style={{ gap: 6, marginTop: 12 }}>
                {canvas.unsupported_notes.length > 0 && <Notice kind="needs-review" title="Unsupported:">{canvas.unsupported_notes.join(", ")} have no current supporting evidence.</Notice>}
                {canvas.contradictions.length > 0 && <Notice kind="needs-review" title="Contradicted:">{canvas.contradictions.map((c) => `${c.from} contradicts ${c.to}`).join("; ")}.</Notice>}
                {canvas.overlapping_evidence.length > 0 && <Notice kind="needs-review" title="Overlapping evidence:">{canvas.overlapping_evidence.map((o) => `${o.first} and ${o.second} (${o.kind})`).join("; ")} cite the same text, so it may be counted twice.</Notice>}
                {(canvas.superseded_evidence.length > 0 || canvas.missing_evidence.length > 0) && <Notice kind="blocked" title="Out of date:">{[...canvas.superseded_evidence.map((k) => `${k} superseded`), ...canvas.missing_evidence.map((k) => `${k} missing`)].join("; ")}.</Notice>}
              </div>
              {canvas.evidence_coverage.length > 0 && (
                <table className="tb" style={{ marginTop: 10 }}>
                  <thead><tr><th style={{ width: "20%" }}>Note</th><th style={{ width: "35%" }}>Evidence kind</th><th style={{ width: "25%" }}>Coverage</th><th>Sources</th></tr></thead>
                  <tbody>
                    {canvas.evidence_coverage.map((c) => (
                      <tr key={c.note + c.evidence_class}>
                        <td className="mono t-mono-sm">{c.note}</td>
                        <td className="t-sm">{c.evidence_class.replace(/_/g, " ")}</td>
                        <td><Status kind={c.state === "present" ? "available" : c.state === "conflicting" ? "needs-review" : "blocked"} weight="weak">{c.state}</Status></td>
                        <td className="mono t-mono-sm tnum">{c.source_count}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
              <p className="t-sm i3" style={{ marginTop: 8 }}>Evidence is read live from its source. A canvas holds research notes, not clinical findings.</p>
            </Section>
          )}
        </div>
        <aside className="agent-side" aria-label="Index and canvases">
          <Section title="Index">
            {overview.state.status === "loading" && <Skeleton rows={2} />}
            {overview.state.status === "ready" && <p className="t-sm i2" style={{ marginBottom: 12 }}>{overview.state.data.index_status}</p>}
            <button type="button" className="btn btn-s" onClick={build} disabled={!!pending || overview.state.status === "loading"}>{pending === "build" ? <Glyph kind="running" /> : null}Build index</button>
            <p className="t-sm i3" style={{ marginTop: 12 }}>The index covers this project&apos;s sources and stays on this device.</p>
          </Section>
          {overview.state.status === "ready" && (
            <Section title="Canvases" count={overview.state.data.canvases.length}>
              {overview.state.data.canvases.length ? overview.state.data.canvases.map((c) => (
                <button type="button" key={c.id} className={`lr clickable ${canvas?.canvas.header.id === c.id ? "sel" : ""}`} style={{ gridTemplateColumns: "minmax(0,1fr)", width: "100%", textAlign: "left" }} onClick={() => void openCanvas(c.id)}>
                  <span className="trunc" style={{ fontWeight: 600 }}>{c.title}</span>
                  <span className="t-sm i3 trunc">{c.detail}</span>
                </button>
              )) : <p className="t-sm i3">No canvases in this project yet.</p>}
              <form onSubmit={create} className="field-row" style={{ marginTop: 10 }}>
                <input id="canvas-title" type="text" value={newTitle} onChange={(e) => setNewTitle(e.target.value)} placeholder="New canvas title" maxLength={200} aria-label="New canvas title" style={{ flex: "1 1 140px" }} />
                <button type="submit" className="btn btn-s" disabled={!newTitle.trim() || !!pending}>Create</button>
              </form>
            </Section>
          )}
        </aside>
      </div>
    </>
  );
}

/* ───────────────────────────── Browse ───────────────────────────── */
type BrowseVm = { allowlist: Array<{ id: string; target: string; enabled: boolean; revision: number }>; sessions: Array<{ id: string; url: string; state: string; reason: string }>; routes: string; fixture_url: string };

export function Browse() {
  return <ProjectScope title="Browse" sub="Governed fetches through the network broker. Blocked by default; only allowlisted hosts are reachable.">{(pid) => <BrowseBody projectId={pid} />}</ProjectScope>;
}

function BrowseBody({ projectId }: { projectId: string }) {
  const { ws } = useApp();
  const o = useCommand<BrowseVm>("browse_overview", { projectId });
  const [host, setHost] = useState("fixture.medscale.test");
  const [prefix, setPrefix] = useState("");
  const [url, setUrl] = useState("");
  const [fetched, setFetched] = useState<{ session: { url: string; state: string; reason: string }; excerpt: string; flagged: boolean; downloads: number } | null>(null);
  const { pending, run } = useAction();
  const outcome = useOutcome();
  const toast = useToast();
  useEffect(() => { if (o.state.status === "ready" && !url) setUrl(o.state.data.fixture_url); }, [o.state, url]);

  async function allow(e: FormEvent) { e.preventDefault(); const r = await run("allow", "browse_allow", { projectId, hostName: host, pathPrefix: prefix }); if (outcome(r, "Host allowlisted")) await o.reload(); }
  async function allowTargetHost(hostname: string) {
    const r = await run("allow", "browse_allow", { projectId, hostName: hostname, pathPrefix: "" });
    if (outcome(r, `Allowlisted ${hostname}`)) await o.reload();
  }
  async function fetchUrl(e: FormEvent) {
    e.preventDefault();
    const r = await run<typeof fetched>("fetch", "browse_fetch", { projectId, url });
    if (r.ok && r.data) {
      setFetched(r.data);
      if (/denied|blocked/i.test(r.data.session.state)) {
        toast({ kind: "blocked", title: "Fetch blocked", detail: r.data.session.reason || r.data.session.state });
      } else {
        toast({ kind: "completed", title: "Fetch recorded" });
      }
    } else {
      outcome(r, "Fetch recorded");
    }
    await o.reload();
  }
  async function disable(id: string, revision: number) { const r = await run("disable", "browse_disable", { entryId: id, revision }); if (outcome(r, "Host disabled")) await o.reload(); }

  if (o.state.status === "loading") return <Skeleton />;
  if (o.state.status === "error") return <ErrorState error={o.state.error} onRetry={o.reload} />;
  const v = o.state.data;
  const deniedHost = (() => {
    if (!fetched || !/denied|blocked/i.test(fetched.session.state)) return null;
    try { return new URL(fetched.session.url).hostname; } catch { return null; }
  })();
  const allowedSessions = v.sessions.filter((s) => !/denied|blocked|fail/i.test(s.state)).length;
  const deniedSessions = v.sessions.length - allowedSessions;

  return (
    <>
      {ws?.workspace?.kind === "synthetic" && (
        <Notice kind="synthetic" title="Offline fixture network.">In the synthetic workspace the broker serves <span className="mono">{v.fixture_url}</span> from a local fixture. No request leaves this device.</Notice>
      )}
      <div className="grid-2" style={{ marginTop: 18 }}>
        <Section title="Fetch" count="through the broker">
          <form onSubmit={fetchUrl} className="field-row">
            <input type="text" value={url} onChange={(e) => setUrl(e.target.value)} aria-label="URL" className="mono" style={{ flex: 1 }} />
            <button type="submit" className="btn btn-p btn-inline" disabled={!url.trim() || !!pending}>Fetch</button>
          </form>
          {fetched && (
            <div className="stack" style={{ marginTop: 12 }}>
              <div className="row-gap" style={{ gap: 10 }}>
                <Status kind={glyphFor(fetched.session.state)} weight="strong">{fetched.session.state}{fetched.session.reason ? ` · ${fetched.session.reason}` : ""}</Status>
                {deniedHost && (
                  <button type="button" className="btn btn-s" disabled={!!pending} onClick={() => void allowTargetHost(deniedHost)}>Allow {deniedHost}</button>
                )}
              </div>
              {fetched.excerpt && <pre className="pre">{fetched.excerpt}</pre>}
              {fetched.flagged && <Notice kind="needs-review" title="Flagged.">Content was flagged by the privacy recognizers.</Notice>}
              <span className="t-sm i3">{fetched.downloads} download candidate{fetched.downloads === 1 ? "" : "s"}</span>
            </div>
          )}
        </Section>
        <Section title="Allowlist" count={v.allowlist.length}>
          <form onSubmit={allow} className="field-row" style={{ marginBottom: 10 }}>
            <input type="text" value={host} onChange={(e) => setHost(e.target.value)} placeholder="host" aria-label="Host" style={{ flex: 1 }} />
            <input type="text" value={prefix} onChange={(e) => setPrefix(e.target.value)} placeholder="/path prefix" aria-label="Path prefix" style={{ width: 120 }} />
            <button type="submit" className="btn btn-s" disabled={!host.trim() || !!pending}>Allow</button>
          </form>
          {v.allowlist.map((a) => (
            <div key={a.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) auto auto" }}>
              <span className="mono t-mono-sm trunc">{a.target}</span>
              <Status kind={a.enabled ? "available" : "cancelled"} weight="weak">{a.enabled ? "Enabled" : "Disabled"}</Status>
              {a.enabled ? <button type="button" className="btn btn-g" disabled={!!pending} onClick={() => disable(a.id, a.revision)}>Disable</button> : <span />}
            </div>
          ))}
          {!v.allowlist.length && <p className="t-sm i3">Allowlist is empty — every fetch is blocked by default until you add a host above.</p>}
        </Section>
      </div>
      <Section title="Sessions" count={v.sessions.length}>
        {v.sessions.length > 0 && (
          <div style={{ marginBottom: 12 }}>
            <SegmentedBar
              title="Broker session outcomes"
              meta={`${v.sessions.length} total`}
              items={[
                { key: "allowed", label: "Allowed", count: allowedSessions, tone: "present" },
                { key: "denied", label: "Denied / blocked", count: deniedSessions, tone: "danger" },
              ]}
            />
          </div>
        )}
        {v.sessions.length ? (
          <table className="tb">
            <thead><tr><th style={{ width: "46%" }}>URL</th><th style={{ width: "18%" }}>State</th><th>Reason</th></tr></thead>
            <tbody>
              {v.sessions.map((s) => (
                <tr key={s.id}>
                  <td><span className="mono t-mono-sm trunc">{s.url}</span></td>
                  <td><Status kind={glyphFor(s.state)}>{s.state}</Status></td>
                  <td className="t-sm i2">{s.reason || "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : <p className="t-sm i3">No fetches recorded.</p>}
        <p className="note" style={{ marginTop: 6 }}>{v.routes}</p>
      </Section>
    </>
  );
}

/* ───────────────────────────── Research OS ───────────────────────────── */
const PLANE_DESC: Record<string, string> = {
  Hub: "Project artifacts and shared references",
  Compute: "Local job execution and resource budgets",
  "R Workspace": "Isolated statistical scripting sessions",
  Packs: "Verified model and capability bundles",
  "Research Packs": "Reproducible study bundles and manifests",
  Extensions: "Sandboxed local workflow plugins",
  Huddles: "Local working sessions and review notes",
  Adapters: "Read-only analytics and data connectors",
  Federation: "Cross-site coordination (offline by default)",
  Institutional: "Organization-scoped governance records",
};

export function ResearchOS() {
  return <ProjectScope title="Research OS" sub="Inspect local research infrastructure planes—artifacts, compute, packs, extensions, federation, and governance.">{(pid) => <ResearchOSBody projectId={pid} />}</ProjectScope>;
}

function ResearchOSBody({ projectId }: { projectId: string }) {
  const rows = useCommand<Array<{ plane: string; id: string; state: string; detail: string; action: string }>>("research_os_rows", { projectId });
  const { pending, run } = useAction();
  const outcome = useOutcome();
  async function act(r: { plane: string; id: string; action: string }) { const res = await run<string>(r.id, "research_os_act", { projectId, plane: r.plane, id: r.id, action: r.action }); if (outcome(res, `${r.action} · ${r.id}`)) await rows.reload(); }
  if (rows.state.status === "loading") return <Skeleton />;
  if (rows.state.status === "error") return <ErrorState error={rows.state.error} onRetry={rows.reload} />;
  const all = rows.state.data;
  const isPlaceholder = (r: (typeof all)[number]) => r.id === "-" || /^empty$/i.test(r.state);
  const planes = [...new Set(all.map((r) => r.plane))];
  const active = planes.filter((p) => all.some((r) => r.plane === p && !isPlaceholder(r)));

  return (
    <>
      {planes.length === 0 ? (
        <Empty icon="research" title="No plane activity">Nothing is registered in the research planes for this project yet.</Empty>
      ) : (
        <div className="plane-grid" role="group" aria-label="Research planes overview">
          {planes.map((plane) => {
            const items = all.filter((r) => r.plane === plane && !isPlaceholder(r));
            return (
              <div key={plane} className="plane-tile">
                <div className="plane-tile-head">
                  <span className="t-h3">{plane}</span>
                  <Status kind={items.length ? "available" : "unknown"} weight={items.length ? "strong" : "weak"}>
                    {items.length ? `${items.length} active` : "Idle"}
                  </Status>
                </div>
                <span className="t-sm i3">{PLANE_DESC[plane] ?? "Project infrastructure plane"}</span>
              </div>
            );
          })}
        </div>
      )}
      {active.map((plane) => {
        const items = all.filter((r) => r.plane === plane && !isPlaceholder(r));
        return (
          <Section key={plane} title={plane} count={items.length}>
            {items.map((r) => (
              <div key={r.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) 140px minmax(0,1.4fr) auto" }}>
                <span className="mono t-mono-sm trunc">{r.id}</span>
                <Status kind={glyphFor(r.state)}>{r.state}</Status>
                <span className="t-sm i3 trunc" title={r.detail}>{r.detail}</span>
                {r.action ? <button type="button" className="btn btn-g" disabled={!!pending} onClick={() => act(r)}>{r.action}</button> : <span />}
              </div>
            ))}
          </Section>
        );
      })}
      <Authority>Every plane action executes through the local runtime with its own receipt; Desktop only reflects verified state.</Authority>
    </>
  );
}
