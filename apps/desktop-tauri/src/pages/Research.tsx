import { useEffect, useState, type FormEvent } from "react";
import { useApp } from "../lib/app";
import { useAction, useCommand } from "../lib/ipc";
import { ProjectScope, type ProjectRow } from "../components/project";
import { Authority, Empty, ErrorState, Glyph, KV, Ledger, Notice, PageHead, Section, Skeleton, Stamp, Status, glyphFor, useOutcome, useToast } from "../components/ui";

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
    if (outcome(r, "Project archived")) await list.reload();
  }
  async function addExperiment(e: FormEvent) {
    e.preventDefault();
    if (!current) return;
    const r = await run("exp", "experiment_create", { projectId: current.id, name: exp });
    if (outcome(r, "Experiment created")) { setExp(""); await detail.reload(); await list.reload(); }
  }

  return (
    <>
      <div className="pane">
        <PageHead title="Projects" sub="Versioned research containers: experiments, references to vault objects, and the graph between them.">
          <button type="button" className={`flt ${showArchived ? "on" : ""}`} onClick={() => setShowArchived((s) => !s)} aria-pressed={showArchived}>Show archived</button>
        </PageHead>
        {list.state.status === "loading" && <Skeleton />}
        {list.state.status === "error" && <ErrorState error={list.state.error} onRetry={list.reload} />}
        {list.state.status === "ready" && (rows.length ? (
          <table className="tb" aria-label="Projects">
            <thead><tr><th style={{ width: "34%" }}>Project</th><th style={{ width: "14%" }}>Status</th><th className="num" style={{ width: "12%" }} title="Revision">Rev</th><th className="num col-p3" style={{ width: "13%" }} title="Experiments">Exp</th><th className="num col-p3" style={{ width: "13%" }} title="References">Refs</th><th className="num">Edges</th></tr></thead>
            <tbody>{rows.map((p) => (
              <tr key={p.id} className={`r2 clickable ${current?.id === p.id ? "sel" : ""}`} tabIndex={0} onClick={() => setProject(p.id)} onKeyDown={(e) => e.key === "Enter" && setProject(p.id)}>
                <td><div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="pname trunc">{p.name}</span><span className="mono t-mono-sm i3 trunc">{p.id}</span></div></td>
                <td><Status kind={glyphFor(p.status)}>{p.status}</Status></td>
                <td className="num mono t-mono-sm">{p.revision}</td><td className="num mono t-mono-sm col-p3">{p.experiments}</td><td className="num mono t-mono-sm col-p3">{p.refs}</td><td className="num mono t-mono-sm">{p.edges}</td>
              </tr>
            ))}</tbody>
          </table>
        ) : <Empty icon="projects" title="No projects in this workspace">A project pins vault objects at a revision, so an experiment can be re-read exactly as it was. Create one on the right.</Empty>)}

        {current && detail.state.status === "ready" && (
          <Section n="02" title="Project" count={detail.state.data.name}>
            <Ledger items={[{ k: "project_id", v: detail.state.data.project_id, mono: true }, { k: "status", v: detail.state.data.status }, { k: "revision", v: detail.state.data.revision, mono: true }, { k: "experiments", v: detail.state.data.experiment_count, mono: true }, { k: "refs", v: detail.state.data.active_ref_count, mono: true }, { k: "edges", v: detail.state.data.active_edge_count, mono: true }]} />
            <div className="grid-3">
              <div><div className="t-h3" style={{ marginBottom: 8 }}>Experiments</div>
                {detail.state.data.experiments.length ? detail.state.data.experiments.map((x) => <div key={x.id} style={{ display: "flex", justifyContent: "space-between", minHeight: 30, borderBottom: "1px solid var(--line-subtle)", alignItems: "center", gap: 8 }}><span className="t-sm trunc">{x.name}</span><Status kind={glyphFor(x.status)} weight="weak">{x.status}</Status></div>) : <p className="t-sm i3">None yet.</p>}
                <form onSubmit={addExperiment} className="field-row" style={{ marginTop: 10 }}><input type="text" value={exp} onChange={(e) => setExp(e.target.value)} placeholder="Experiment name" maxLength={128} aria-label="Experiment name" style={{ flex: 1 }} /><button type="submit" className="btn btn-s" disabled={!exp.trim() || !!pending}>Add</button></form>
              </div>
              <div><div className="t-h3" style={{ marginBottom: 8 }}>References</div>
                {detail.state.data.refs.length ? detail.state.data.refs.map((r) => <div key={r.ref_id} style={{ minHeight: 30, borderBottom: "1px solid var(--line-subtle)" }}><Stamp k={r.kind} v={r.object_id} /> <Status kind={glyphFor(r.resolution)} weight="weak">{r.resolution}</Status></div>) : <p className="t-sm i3">No vault objects pinned.</p>}
              </div>
              <div><div className="t-h3" style={{ marginBottom: 8 }}>Graph</div>
                {detail.state.data.edges.length ? detail.state.data.edges.map((g) => <div key={g.edge_id} className="t-sm i2" style={{ minHeight: 30, borderBottom: "1px solid var(--line-subtle)" }}>{g.subject} <span className="mono t-mono-sm">{g.predicate}</span> {g.object}</div>) : <p className="t-sm i3">No edges.</p>}
              </div>
            </div>
          </Section>
        )}
      </div>
      <aside className="inspector collapsible" aria-label="New project">
        <form onSubmit={create} className="ins-pad" style={{ gap: 16 }}>
          <div className="sec" style={{ margin: 0 }}><span className="sec-t">New project</span><span className="sec-rule" /></div>
          <div><label className="lbl" htmlFor="pn">Name</label><input id="pn" type="text" value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Coverage audit" maxLength={128} style={{ width: "100%" }} /><span className="t-sm i3" style={{ display: "block", marginTop: 4 }}>Up to 128 characters.</span></div>
          <div><label className="lbl" htmlFor="pd">Description</label><textarea id="pd" value={desc} onChange={(e) => setDesc(e.target.value)} maxLength={512} style={{ width: "100%", minHeight: 64 }} /></div>
          <div className="row-gap" style={{ gap: 8 }}><button type="submit" className="btn btn-p" disabled={!name.trim() || !!pending}>{pending === "create" ? <><Glyph kind="running" />Creating</> : "Create project"}</button></div>
          {current && current.status !== "archived" && <><div className="sec" style={{ margin: "8px 0 0" }}><span className="sec-t">Selected</span><span className="sec-rule" /></div>
            <button type="button" className="btn btn-s" onClick={archive} disabled={!!pending}>Archive “{current.name}”</button>
            <p className="t-sm i3">Archiving keeps every revision. Core rejects it if the project changed since you loaded it.</p></>}
          <Notice kind="unavailable" title="Writes go through Core.">If the vault lease is not ready you will see the Core message, and nothing is written.</Notice>
        </form>
      </aside>
    </>
  );
}

/* ───────────────────────────── Data ───────────────────────────── */
type Source = { id: string; name: string; kind: string; health: string; revision: number };
type Snapshot = { id: string; rows: number; digest: string };
type Page = { snapshot_id: string; schema_line: string; rows: string[]; next_cursor: string | null; detail: string };

export function Data() {
  return <ProjectScope title="Data" sub="Local datasets and pinned snapshots. Rows are read from a snapshot, never from a live file.">{(pid) => <DataBody projectId={pid} />}</ProjectScope>;
}

function DataBody({ projectId }: { projectId: string }) {
  const sources = useCommand<Source[]>("data_sources", { projectId });
  const [source, setSource] = useState<string | null>(null);
  const [snap, setSnap] = useState<string | null>(null);
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

  return (
    <>
      <div className="row-gap" style={{ gap: 8, marginBottom: 12 }}>
        <button type="button" className="btn btn-s" onClick={addSample} disabled={!!pending}>{pending === "sample" ? <Glyph kind="running" /> : null}Add synthetic sample source</button>
        <button type="button" className="btn btn-g" onClick={importSnap} disabled={!source || !!pending}>Import snapshot</button>
        <span className="t-sm i3">Paged by cursor · read-only</span>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "280px minmax(0,1fr)", gap: 24 }}>
        <div>
          <Section title="Sources" count={sources.state.status === "ready" ? sources.state.data.length : "…"}>
            {sources.state.status === "error" && <ErrorState error={sources.state.error} onRetry={sources.reload} />}
            {sources.state.status === "ready" && !sources.state.data.length && <p className="t-sm i3">No data sources. Add the synthetic sample to try the workbench.</p>}
            {sources.state.status === "ready" && sources.state.data.map((s) => (
              <button type="button" key={s.id} className={`lr clickable ${source === s.id ? "sel" : ""}`} style={{ gridTemplateColumns: "minmax(0,1fr) auto", width: "100%", textAlign: "left" }} onClick={() => { setSource(s.id); setSnap(null); }}>
                <span style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="trunc" style={{ fontWeight: 600 }}>{s.name}</span><span className="mono t-mono-sm i3 trunc">{s.kind} · r{s.revision}</span></span>
                <Status kind={glyphFor(s.health)} weight="weak">{s.health}</Status>
              </button>
            ))}
          </Section>
          {snaps.state.status === "ready" && <Section title="Snapshots" count={snaps.state.data.length}>
            {snaps.state.data.length ? snaps.state.data.map((s) => (
              <button type="button" key={s.id} className={`lr clickable ${snap === s.id ? "sel" : ""}`} style={{ gridTemplateColumns: "minmax(0,1fr) auto", width: "100%", textAlign: "left" }} onClick={() => setSnap(s.id)}>
                <span className="mono t-mono-sm trunc">{s.id}</span><span className="mono t-mono-sm i3">{s.rows} rows</span>
              </button>
            )) : <p className="t-sm i3">No snapshots yet. Import one.</p>}
          </Section>}
        </div>
        <div>
          {!snap && <Empty icon="data" title="No snapshot selected">Rows are paged from a pinned snapshot, so every view can be reproduced from its snapshot ID.</Empty>}
          {snap && page.state.status === "loading" && <Skeleton />}
          {snap && page.state.status === "error" && <ErrorState error={page.state.error} onRetry={page.reload} />}
          {snap && page.state.status === "ready" && <>
            <Ledger items={[{ k: "snapshot_id", v: page.state.data.snapshot_id, mono: true }, { k: "schema", v: page.state.data.schema_line, mono: true }, { k: "rows on page", v: page.state.data.rows.length, mono: true }]} />
            <pre className="pre" aria-label="Snapshot rows">{page.state.data.rows.join("\n") || "No rows."}</pre>
            <p className="note" style={{ marginTop: 6 }}>{page.state.data.detail}{page.state.data.next_cursor ? ` · next cursor ${page.state.data.next_cursor}` : ""}</p>
            <Section n="02" title="Transform" count="creates a new snapshot">
              <form onSubmit={transform} className="stack" style={{ gap: 10 }}>
                <div className="field-row"><label className="t-sm i3" style={{ width: 70 }}>Columns</label><input type="text" value={cols} onChange={(e) => setCols(e.target.value)} placeholder="comma-separated, empty = all" style={{ flex: 1 }} /></div>
                <div className="field-row"><label className="t-sm i3" style={{ width: 70 }}>Filter</label><input type="text" value={fc} onChange={(e) => setFc(e.target.value)} placeholder="column" style={{ width: 140 }} /><select value={fo} onChange={(e) => setFo(e.target.value)} aria-label="Filter operator"><option value="equals">equals</option><option value="contains">contains</option><option value="regex">regex</option></select><input type="text" value={fv} onChange={(e) => setFv(e.target.value)} placeholder="value" style={{ flex: 1 }} /></div>
                <div className="field-row"><label className="t-sm i3" style={{ width: 70 }}>Sort</label><input type="text" value={sort} onChange={(e) => setSort(e.target.value)} placeholder="column:asc or column:desc" style={{ flex: 1 }} /></div>
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
  return <ProjectScope title="Analytics" sub="Read-only queries over pinned snapshots, run on this device. Every run leaves a receipt.">{(pid) => <AnalyticsBody projectId={pid} />}</ProjectScope>;
}

function AnalyticsBody({ projectId }: { projectId: string }) {
  const receipts = useCommand<Array<{ id: string; outcome: string; detail: string }>>("analytics_receipts", { projectId });
  const [sql, setSql] = useState("SELECT * FROM t");
  const [bindings, setBindings] = useState("");
  const [result, setResult] = useState<{ summary: string; preview: string } | null>(null);
  const { pending, run } = useAction();
  const outcome = useOutcome();
  const toast = useToast();

  async function exec(e: FormEvent) {
    e.preventDefault();
    const r = await run<{ summary: string; preview: string }>("run", "analytics_query", { projectId, sql, bindings });
    if (r.ok && /^(denied|failed)/i.test(r.data.summary)) { toast({ kind: "blocked", title: "Denied by Core", detail: r.data.summary }); setResult(r.data); }
    else if (outcome(r, "Query ran")) setResult(r.data); else setResult(null);
    await receipts.reload();
  }

  return (
    <>
      <form onSubmit={exec} style={{ border: "1px solid var(--line)", borderRadius: "var(--r-2)", overflow: "hidden" }}>
        <div className="field-row" style={{ height: 44, padding: "0 8px 0 12px", borderBottom: "1px solid var(--line-subtle)", background: "var(--bg-raised)", flexWrap: "nowrap" }}>
          <span className="t-label">Bindings</span>
          <input type="text" value={bindings} onChange={(e) => setBindings(e.target.value)} placeholder="alias=snapshot_id (e.g. t=snap-…)" aria-label="Bindings" className="mono" style={{ flex: 1, height: 28 }} />
          <Status kind="limited" weight="weak">Read-only · writes denied</Status>
          <button type="submit" className="btn btn-p" disabled={!!pending || !sql.trim()}>{pending ? <Glyph kind="running" /> : null}Run</button>
        </div>
        <textarea className="mono" value={sql} onChange={(e) => setSql(e.target.value)} aria-label="SQL" style={{ width: "100%", border: 0, borderRadius: 0, minHeight: 150, background: "var(--bg-surface)" }} />
      </form>
      <p className="note" style={{ marginTop: 6 }}>Bind each table alias to a snapshot ID from Data. Core rejects writes with <span className="mono">not_read_only</span>.</p>
      <Section n="02" title="Result">
        {result ? <><p className="t-sm" style={{ marginBottom: 8 }}>{result.summary}</p><pre className="pre">{result.preview || "No rows."}</pre></> : <p className="t-sm i3">No result yet. Results show a bounded preview and the snapshot they came from.</p>}
      </Section>
      <Section n="03" title="Receipts" count={receipts.state.status === "ready" ? receipts.state.data.length : "…"}>
        {receipts.state.status === "error" && <ErrorState error={receipts.state.error} onRetry={receipts.reload} />}
        {receipts.state.status === "ready" && (receipts.state.data.length ? (
          <table className="tb"><thead><tr><th style={{ width: "34%" }}>Receipt</th><th style={{ width: "18%" }}>Outcome</th><th>Detail</th></tr></thead>
            <tbody>{receipts.state.data.map((r) => <tr key={r.id}><td><span className="mono t-mono-sm trunc">{r.id}</span></td><td><Status kind={glyphFor(r.outcome)}>{r.outcome}</Status></td><td className="t-sm i2"><span className="trunc">{r.detail}</span></td></tr>)}</tbody></table>
        ) : <p className="t-sm i3">No runs recorded in this project.</p>)}
      </Section>
      <Authority>Query results describe a snapshot. They are not clinical findings and are not written back to any record.</Authority>
    </>
  );
}

/* ───────────────────────────── Knowledge ───────────────────────────── */
export function Knowledge() {
  return <ProjectScope title="Knowledge" sub="A local index over this project's sources, with search receipts and canvases.">{(pid) => <KnowledgeBody projectId={pid} />}</ProjectScope>;
}

function KnowledgeBody({ projectId }: { projectId: string }) {
  const overview = useCommand<{ index_status: string; canvases: Array<{ id: string; title: string; detail: string }> }>("knowledge_overview", { projectId });
  const [q, setQ] = useState("");
  const [res, setRes] = useState<{ summary: string; hits: string } | null>(null);
  const { pending, run } = useAction();
  const outcome = useOutcome();

  async function build() { const r = await run<string>("build", "knowledge_build", { projectId }); if (outcome(r, "Index built")) await overview.reload(); }
  async function search(e: FormEvent) { e.preventDefault(); const r = await run<{ summary: string; hits: string }>("search", "knowledge_search", { projectId, query: q }); if (outcome(r, "Search complete")) setRes(r.data); }

  return (
    <>
      {overview.state.status === "error" && <ErrorState error={overview.state.error} onRetry={overview.reload} />}
      {overview.state.status === "ready" && <Ledger items={[{ k: "index", v: overview.state.data.index_status }, { k: "canvases", v: overview.state.data.canvases.length, mono: true }]} />}
      <div className="row-gap" style={{ gap: 8, marginBottom: 12 }}>
        <button type="button" className="btn btn-s" onClick={build} disabled={!!pending}>{pending === "build" ? <Glyph kind="running" /> : null}Build index</button>
        <form onSubmit={search} className="field-row" style={{ flex: 1 }}>
          <input type="text" value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search indexed sources" maxLength={256} aria-label="Search indexed sources" style={{ flex: "1 1 240px", maxWidth: 420 }} />
          <button type="submit" className="btn btn-g" disabled={!q.trim() || !!pending}>Search</button>
        </form>
      </div>
      <Section title="Results">{res ? <><p className="t-sm" style={{ marginBottom: 8 }}>{res.summary}</p><pre className="pre">{res.hits || "No hits."}</pre></> : <p className="t-sm i3">Build the index, then search. The index covers this project's data sources only.</p>}</Section>
      {overview.state.status === "ready" && <Section title="Canvases" count={overview.state.data.canvases.length}>
        {overview.state.data.canvases.length ? overview.state.data.canvases.map((c) => <div key={c.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) minmax(0,1.5fr)" }}><span className="trunc" style={{ fontWeight: 600 }}>{c.title}</span><span className="t-sm i3 trunc">{c.detail}</span></div>) : <p className="t-sm i3">No canvases in this project.</p>}
      </Section>}
    </>
  );
}

/* ───────────────────────────── Browse ───────────────────────────── */
type BrowseVm = { allowlist: Array<{ id: string; target: string; enabled: boolean; revision: number }>; sessions: Array<{ id: string; url: string; state: string; reason: string }>; routes: string; fixture_url: string };

export function Browse() {
  return <ProjectScope title="Browse" sub="Governed fetches through the network broker. Default deny; only allowlisted hosts are reachable.">{(pid) => <BrowseBody projectId={pid} />}</ProjectScope>;
}

function BrowseBody({ projectId }: { projectId: string }) {
  const o = useCommand<BrowseVm>("browse_overview", { projectId });
  const [host, setHost] = useState("fixture.medscale.test");
  const [prefix, setPrefix] = useState("");
  const [url, setUrl] = useState("");
  const [fetched, setFetched] = useState<{ session: { url: string; state: string; reason: string }; excerpt: string; flagged: boolean; downloads: number } | null>(null);
  const { pending, run } = useAction();
  const outcome = useOutcome();
  useEffect(() => { if (o.state.status === "ready" && !url) setUrl(o.state.data.fixture_url); }, [o.state, url]);

  async function allow(e: FormEvent) { e.preventDefault(); const r = await run("allow", "browse_allow", { projectId, hostName: host, pathPrefix: prefix }); if (outcome(r, "Host allowlisted")) await o.reload(); }
  async function fetchUrl(e: FormEvent) { e.preventDefault(); const r = await run<typeof fetched>("fetch", "browse_fetch", { projectId, url }); if (r.ok) setFetched(r.data); outcome(r, "Fetch recorded"); await o.reload(); }
  async function disable(id: string, revision: number) { const r = await run("disable", "browse_disable", { entryId: id, revision }); if (outcome(r, "Host disabled")) await o.reload(); }

  if (o.state.status === "loading") return <Skeleton />;
  if (o.state.status === "error") return <ErrorState error={o.state.error} onRetry={o.reload} />;
  const v = o.state.data;
  return (
    <>
      <Notice kind="synthetic" title="Offline fixture network.">In the synthetic workspace the broker serves <span className="mono">{v.fixture_url}</span> from a local fixture. No request leaves this device.</Notice>
      <div className="grid-2" style={{ marginTop: 18 }}>
        <Section title="Fetch" count="through the broker">
          <form onSubmit={fetchUrl} className="field-row"><input type="text" value={url} onChange={(e) => setUrl(e.target.value)} aria-label="URL" className="mono" style={{ flex: 1 }} /><button type="submit" className="btn btn-p" disabled={!url.trim() || !!pending}>Fetch</button></form>
          {fetched && <div className="stack" style={{ marginTop: 12 }}><Status kind={glyphFor(fetched.session.state)} weight="strong">{fetched.session.state}{fetched.session.reason ? ` · ${fetched.session.reason}` : ""}</Status>{fetched.excerpt && <pre className="pre">{fetched.excerpt}</pre>}{fetched.flagged && <Notice kind="needs-review" title="Flagged.">Content was flagged by the privacy recognizers.</Notice>}<span className="t-sm i3">{fetched.downloads} download candidate{fetched.downloads === 1 ? "" : "s"}</span></div>}
        </Section>
        <Section title="Allowlist" count={v.allowlist.length}>
          <form onSubmit={allow} className="field-row" style={{ marginBottom: 10 }}><input type="text" value={host} onChange={(e) => setHost(e.target.value)} placeholder="host" aria-label="Host" style={{ flex: 1 }} /><input type="text" value={prefix} onChange={(e) => setPrefix(e.target.value)} placeholder="/path prefix" aria-label="Path prefix" style={{ width: 120 }} /><button type="submit" className="btn btn-s" disabled={!host.trim() || !!pending}>Allow</button></form>
          {v.allowlist.map((a) => <div key={a.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) auto auto" }}><span className="mono t-mono-sm trunc">{a.target}</span><Status kind={a.enabled ? "available" : "cancelled"} weight="weak">{a.enabled ? "Enabled" : "Disabled"}</Status>{a.enabled ? <button type="button" className="btn btn-g" onClick={() => disable(a.id, a.revision)}>Disable</button> : <span />}</div>)}
          {!v.allowlist.length && <p className="t-sm i3">Empty allowlist: every fetch is denied.</p>}
        </Section>
      </div>
      <Section title="Sessions" count={v.sessions.length}>
        {v.sessions.length ? <table className="tb"><thead><tr><th style={{ width: "46%" }}>URL</th><th style={{ width: "18%" }}>State</th><th>Reason</th></tr></thead><tbody>{v.sessions.map((s) => <tr key={s.id}><td><span className="mono t-mono-sm trunc">{s.url}</span></td><td><Status kind={glyphFor(s.state)}>{s.state}</Status></td><td className="t-sm i2">{s.reason || "—"}</td></tr>)}</tbody></table> : <p className="t-sm i3">No fetches recorded.</p>}
        <p className="note" style={{ marginTop: 6 }}>{v.routes}</p>
      </Section>
    </>
  );
}

/* ───────────────────────────── Research OS ───────────────────────────── */
export function ResearchOS() {
  return <ProjectScope title="Research OS" sub="Hub, compute, packs, extensions, federation and institutional planes for this project.">{(pid) => <ResearchOSBody projectId={pid} />}</ProjectScope>;
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
  const idle = planes.filter((p) => !active.includes(p));
  return (
    <>
      {planes.length === 0 && <Empty icon="research" title="No plane activity">Nothing is registered in the hub, compute, pack, extension, federation or institutional planes for this project.</Empty>}
      {active.map((plane, i) => {
        const items = all.filter((r) => r.plane === plane && !isPlaceholder(r));
        return (
          <Section key={plane} n={String(i + 1).padStart(2, "0")} title={plane} count={items.length}>
            {items.map((r) => (
              <div key={r.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) 140px minmax(0,1.4fr) auto" }}>
                <span className="mono t-mono-sm trunc">{r.id}</span><Status kind={glyphFor(r.state)}>{r.state}</Status><span className="t-sm i3 trunc" title={r.detail}>{r.detail}</span>
                {r.action ? <button type="button" className="btn btn-g" disabled={!!pending} onClick={() => act(r)}>{r.action}</button> : <span />}
              </div>
            ))}
          </Section>
        );
      })}
      {idle.length > 0 && (
        <Section n={String(active.length + 1).padStart(2, "0")} title="Planes with nothing recorded" count={idle.length}>
          {idle.map((plane) => (
            <div key={plane} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) 140px minmax(0,1.4fr)" }}>
              <span className="t-sm">{plane}</span><Status kind="unknown" weight="weak">Empty</Status><span className="t-sm i3">Nothing registered for this project</span>
            </div>
          ))}
        </Section>
      )}
      <KV rows={[["authority", "Every action is a Core request with its own receipt; Desktop only displays the plane state."]]} />
    </>
  );
}
