import { useEffect, useState, type FormEvent } from "react";
import { useAction, useCommand } from "../lib/ipc";
import { ProjectScope } from "../components/project";
import { Authority, Empty, ErrorState, Glyph, Icon, KV, Ledger, Notice, PageHead, Section, Skeleton, Status, glyphFor, useOutcome } from "../components/ui";

/* ───────────────────────────── Models ───────────────────────────── */
type ModelsVm = {
  model_runtime_summary: string; model_runtime_boundary: string; model_source_summary: string; model_inventory_summary: string;
  openmed_baseline: string; competitive_summary: string;
  models: Array<{ scope: string; task: string; model: string; source: string; runtime: string; device: string; trust: string; benchmark: string; digest: string; state: string }>;
  evidence: Array<{ capability: string; medscale: string; openmed: string; verdict: string; evidence: string; limitations: string }>;
};

export function Models() {
  const { state, reload } = useCommand<ModelsVm>("models_overview");
  const [sel, setSel] = useState(0);
  const [tab, setTab] = useState<"packs" | "comparative">("packs");
  const { pending, run } = useAction();
  const outcome = useOutcome();
  async function admit() { const r = await run("admit", "models_admit_fixture_pack"); if (outcome(r, "Fixture pack submitted to Core")) await reload(); }

  if (state.status === "loading") return <div className="pane"><Skeleton /></div>;
  if (state.status === "error") return <div className="pane"><ErrorState error={state.error} onRetry={reload} /></div>;
  const v = state.data;
  const m = v.models[sel] ?? v.models[0];
  return (
    <>
      <div className="pane">
        <PageHead title="Models" sub="Admitted model packs, the local runtime that executes them, and how far each is trusted.">
          <button type="button" className="btn btn-s" onClick={admit} disabled={!!pending}>{pending ? <Glyph kind="running" /> : null}Admit synthetic fixture pack</button>
        </PageHead>
        <Ledger items={[{ k: "runtime", v: v.model_runtime_summary }, { k: "source", v: v.model_source_summary }, { k: "inventory", v: v.model_inventory_summary }]} />
        <div className="tabs" role="tablist" style={{ marginBottom: 14 }}>
          <button type="button" role="tab" aria-selected={tab === "packs"} className={`tab ${tab === "packs" ? "on" : ""}`} onClick={() => setTab("packs")}>Packs <span className="mono">{v.models.length}</span></button>
          <button type="button" role="tab" aria-selected={tab === "comparative"} className={`tab ${tab === "comparative" ? "on" : ""}`} onClick={() => setTab("comparative")}>Comparative evidence <span className="mono">static</span></button>
        </div>
        {tab === "packs" && (v.models.length ? (
          <table className="tb"><thead><tr><th style={{ width: "30%" }}>Model</th><th style={{ width: "18%" }}>Task</th><th className="col-p3" style={{ width: "18%" }}>Runtime</th><th style={{ width: "17%" }}>State</th><th>Trust</th></tr></thead>
            <tbody>{v.models.map((x, i) => (
              <tr key={x.model + i} className={`r2 clickable ${m === x ? "sel" : ""}`} tabIndex={0} onClick={() => setSel(i)} onKeyDown={(e) => e.key === "Enter" && setSel(i)}>
                <td><div style={{ display: "flex", gap: 10, alignItems: "center", minWidth: 0 }}><Icon name="models" /><div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="mono trunc" style={{ fontSize: 12.5, fontWeight: 500 }}>{x.model}</span><span className="t-sm i3 trunc">{x.scope}</span></div></div></td>
                <td className="t-sm i2"><span className="trunc">{x.task}</span></td><td className="col-p3 mono t-mono-sm i2"><span className="trunc">{x.runtime}</span></td>
                <td><Status kind={/candidate|proposal/i.test(x.state) ? "proposal" : glyphFor(x.state)}>{x.state}</Status></td>
                <td className="t-sm i2"><span className="trunc">{x.trust}</span></td>
              </tr>
            ))}</tbody></table>
        ) : <Empty icon="models" title="No admitted packs">Admit the bundled synthetic ONNX fixture pack. Core verifies its manifest, digests and signature before admission.</Empty>)}
        {tab === "comparative" && <Section title="Comparative evidence" count="MedScale vs OpenMed · static ledger">
          <p className="t-sm i2" style={{ marginBottom: 10 }}>{v.competitive_summary} {v.openmed_baseline}</p>
          <table className="tb"><thead><tr><th style={{ width: "22%" }}>Capability</th><th style={{ width: "20%" }}>MedScale</th><th style={{ width: "20%" }}>OpenMed</th><th style={{ width: "14%" }}>Verdict</th><th>Limitations</th></tr></thead>
            <tbody>{v.evidence.map((e, i) => <tr key={i}><td className="t-sm">{e.capability}</td><td className="t-sm i2"><span className="trunc">{e.medscale}</span></td><td className="t-sm i2"><span className="trunc">{e.openmed}</span></td><td><Status kind={glyphFor(e.verdict)}>{e.verdict}</Status></td><td className="t-sm i3"><span className="trunc">{e.limitations}</span></td></tr>)}</tbody></table>
          <p className="note" style={{ marginTop: 8 }}>“Ahead” is shown only where a bound comparative result exists. This is a pinned ledger, not live benchmarking.</p>
        </Section>}
        <Section n="02" title="How model output is treated">
          <div className="grid-2">
            <Notice kind="proposal" title="Proposal.">Every output stays an outline-triangle proposal until a person accepts it.</Notice>
            <Notice kind="unknown" title="Confidence.">Shown only when the task defines it. Raw logits are never presented as probability.</Notice>
          </div>
          <p className="t-sm i3" style={{ marginTop: 10 }}>{v.model_runtime_boundary}</p>
        </Section>
      </div>
      {m && (
        <aside className="inspector collapsible" aria-label="Selected model">
          <div className="ins-pad">
            <div><div className="mono" style={{ fontSize: 13.5, fontWeight: 500 }}>{m.model}</div><div className="row-gap" style={{ gap: 14, marginTop: 8 }}><Status kind={/candidate/i.test(m.state) ? "proposal" : glyphFor(m.state)}>{m.state}</Status><Status kind={/synthetic/i.test(m.source) ? "synthetic" : "available"}>{m.source}</Status></div></div>
            <Section title="Identity"><KV keyWidth={96} rows={[["task", m.task], ["runtime", <span className="mono t-mono-sm">{m.runtime}</span>], ["device", m.device], ["trust", m.trust], ["digest", <span className="mono-break">{m.digest}</span>]]} /></Section>
            <Section title="Evidence"><p className="t-sm i2">{m.benchmark}</p></Section>
            <Authority>Admission is not promotion. Promoting a pack to a task requires an explicit gate that this desktop does not grant.</Authority>
          </div>
        </aside>
      )}
    </>
  );
}

/* ───────────────────────────── MedAgent ───────────────────────────── */
type Run = { id: string; status: string; revision: number; prompt: string };
type Turn = { seq: number; kind: string; payload: string };

export function MedAgent() {
  return <ProjectScope title="MedAgent" sub="Agent runs bound to ingested sources, executed on the local fixture runtime. Output is a proposal.">{(pid) => <MedAgentBody projectId={pid} />}</ProjectScope>;
}

function MedAgentBody({ projectId }: { projectId: string }) {
  const runs = useCommand<{ runs: Run[]; agents: Array<{ id: string; label: string; status: string }> }>("medagent_runs", { projectId });
  const [sel, setSel] = useState<string | null>(null);
  const [prompt, setPrompt] = useState("");
  const [proposal, setProposal] = useState<unknown>(null);
  const turns = useCommand<Turn[]>(sel ? "medagent_turns" : null, sel ? { runId: sel } : undefined);
  const { pending, run } = useAction();
  const outcome = useOutcome();
  const list = runs.state.status === "ready" ? runs.state.data.runs : [];
  const cur = list.find((r) => r.id === sel) ?? null;
  useEffect(() => { if (!sel && list[0]) setSel(list[0].id); }, [list, sel]);

  async function create(e: FormEvent) {
    e.preventDefault();
    const r = await run<Run>("create", "medagent_create_run", { projectId, prompt });
    if (outcome(r, "Run created (pending)")) { setPrompt(""); await runs.reload(); setSel(r.data.id); }
  }
  async function step(cmd: "medagent_start" | "medagent_cancel", label: string) {
    if (!cur) return;
    const r = await run(cmd, cmd, { runId: cur.id, revision: cur.revision });
    if (outcome(r, label)) { await runs.reload(); await turns.reload(); }
  }
  async function execute() {
    if (!cur) return;
    const r = await run<{ turn: unknown; proposal: unknown }>("execute", "medagent_execute", { runId: cur.id });
    if (outcome(r, "Run executed · proposal recorded")) { setProposal(r.data.proposal); await runs.reload(); await turns.reload(); }
  }

  return (
    <div style={{ display: "grid", gridTemplateColumns: "300px minmax(0,1fr)", gap: 24 }}>
      <div>
        <Section title="Runs" count={list.length}>
          {runs.state.status === "error" && <ErrorState error={runs.state.error} onRetry={runs.reload} />}
          {list.length === 0 && runs.state.status === "ready" && <p className="t-sm i3">No runs yet.</p>}
          {list.map((r) => (
            <button type="button" key={r.id} className={`lr clickable ${sel === r.id ? "sel" : ""}`} style={{ gridTemplateColumns: "minmax(0,1fr) auto", width: "100%", textAlign: "left" }} onClick={() => { setSel(r.id); setProposal(null); }}>
              <span style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="trunc">{r.prompt}</span><span className="mono t-mono-sm i3 trunc">{r.id}</span></span>
              <Status kind={glyphFor(r.status)} weight="weak">{r.status}</Status>
            </button>
          ))}
        </Section>
        {runs.state.status === "ready" && <Section title="Agents" count={runs.state.data.agents.length}>
          {runs.state.data.agents.map((a) => <div key={a.id} className="row-gap" style={{ justifyContent: "space-between", minHeight: 30 }}><span className="t-sm trunc">{a.label}</span><Status kind={glyphFor(String(a.status))} weight="weak">{String(a.status)}</Status></div>)}
          {!runs.state.data.agents.length && <p className="t-sm i3">Created with the first run, on the admitted fixture pack.</p>}
        </Section>}
      </div>
      <div>
        <form onSubmit={create} className="stack" style={{ gap: 8, marginBottom: 18 }}>
          <label className="lbl" htmlFor="ap">New run · scope: ingested synthetic sources</label>
          <textarea id="ap" value={prompt} onChange={(e) => setPrompt(e.target.value)} placeholder="Ask about the bound sources…" maxLength={2000} style={{ minHeight: 70 }} />
          <div><button type="submit" className="btn btn-p" disabled={!prompt.trim() || !!pending}>{pending === "create" ? <Glyph kind="running" /> : null}Create run</button></div>
        </form>
        {cur ? <>
          <Ledger items={[{ k: "run", v: cur.id, mono: true }, { k: "status", v: <Status kind={glyphFor(cur.status)} weight="strong">{cur.status}</Status> }, { k: "revision", v: cur.revision, mono: true }]} />
          <div className="row-gap" style={{ gap: 8, marginBottom: 16 }}>
            <button type="button" className="btn btn-s" onClick={() => step("medagent_start", "Run started")} disabled={!!pending || cur.status !== "pending"}>Start</button>
            <button type="button" className="btn btn-p" onClick={execute} disabled={!!pending || cur.status !== "running"}>{pending === "execute" ? <Glyph kind="running" /> : null}Execute on local runtime</button>
            <button type="button" className="btn btn-g" onClick={() => step("medagent_cancel", "Run cancelled")} disabled={!!pending || ["completed", "cancelled", "failed"].includes(cur.status)}>Cancel</button>
          </div>
          <Section title="Turns" count={turns.state.status === "ready" ? turns.state.data.length : "…"}>
            {turns.state.status === "ready" && (turns.state.data.length ? turns.state.data.map((t) => (
              <div key={t.seq} className="lr" style={{ gridTemplateColumns: "36px 140px minmax(0,1fr)", alignItems: "start", padding: "8px 12px" }}><span className="mono t-mono-sm i3">{t.seq}</span><span className="t-sm i2">{t.kind}</span><TurnPayload payload={t.payload} /></div>
            )) : <p className="t-sm i3">No turns yet. Start, then execute the run.</p>)}
          </Section>
          {proposal !== null && <Section title="Proposal" count="not accepted">
            <div style={{ border: "1px dashed var(--line-strong)", borderRadius: "var(--r-2)", padding: "12px 14px" }}>
              <Status kind="proposal" weight="strong">Proposal</Status>
              <pre className="pre" style={{ marginTop: 10, border: 0, background: "transparent", padding: 0 }}>{JSON.stringify(proposal, null, 2)}</pre>
            </div>
          </Section>}
          <Authority>Proposal, not accepted. Nothing is written to a record; a person decides through review.</Authority>
        </> : <Empty icon="medagent" title="No run selected">Create a run. MedScale registers a synthetic agent on the admitted fixture pack and binds its context to ingested sources.</Empty>}
      </div>
    </div>
  );
}

/* ───────────────────────────── Model Fleet ───────────────────────────── */
type FleetVm = { lanes: Array<{ id: string; role_label: string; status: string; revision: number }>; fleets: Array<{ id: string; status: string; revision: number }> };
type FleetDetail = { fleet: { id: string; status: string; revision: number }; lanes: Array<{ lane_id: string; run_id: string; run_status: string }>; report: Report | null };
type Report = { id: string; participating: string; excluded: string; observations: Array<{ kind: string; lanes: string; detail: string }> };

export function ModelFleet() {
  return <ProjectScope title="Model Fleet" sub="Independent lanes run the same task on the local runtime; comparisons record agreement without picking a winner.">{(pid) => <FleetBody projectId={pid} />}</ProjectScope>;
}

function FleetBody({ projectId }: { projectId: string }) {
  const o = useCommand<FleetVm>("fleet_overview", { projectId });
  const [sel, setSel] = useState<string | null>(null);
  const [prompt, setPrompt] = useState("");
  const detail = useCommand<FleetDetail>(sel ? "fleet_detail" : null, sel ? { fleetId: sel } : undefined);
  const { pending, run } = useAction();
  const outcome = useOutcome();
  const v = o.state.status === "ready" ? o.state.data : null;
  useEffect(() => { if (v && !sel && v.fleets[0]) setSel(v.fleets[0].id); }, [v, sel]);

  const after = async () => { await o.reload(); await detail.reload(); };
  async function setup() { const r = await run("setup", "fleet_setup_lanes", { projectId }); if (outcome(r, "Two reviewer lanes created")) await o.reload(); }
  async function create(e: FormEvent) { e.preventDefault(); const r = await run<{ id: string }>("create", "fleet_create", { projectId, prompt }); if (outcome(r, "Fleet run created")) { setPrompt(""); await o.reload(); setSel(r.data.id); } }
  const cur = v?.fleets.find((f) => f.id === sel) ?? null;
  async function dispatch() { if (!cur || !v) return; const r = await run("dispatch", "fleet_dispatch", { fleetId: cur.id, revision: cur.revision, laneIds: v.lanes.filter((l) => l.status === "active").map((l) => l.id) }); if (outcome(r, "Dispatched to lanes")) await after(); }
  async function execLane(laneId: string) { if (!cur) return; const r = await run("exec", "fleet_execute_lane", { fleetId: cur.id, laneId }); if (outcome(r, "Lane executed")) await after(); }
  async function compare() { if (!cur) return; const r = await run("compare", "fleet_compare", { fleetId: cur.id }); if (outcome(r, "Comparison recorded")) await after(); }
  async function cancel() { if (!cur) return; const r = await run("cancel", "fleet_cancel", { fleetId: cur.id, revision: cur.revision }); if (outcome(r, "Fleet cancelled")) await after(); }

  if (o.state.status === "loading") return <Skeleton />;
  if (o.state.status === "error") return <ErrorState error={o.state.error} onRetry={o.reload} />;
  return (
    <div style={{ display: "grid", gridTemplateColumns: "300px minmax(0,1fr)", gap: 24 }}>
      <div>
        <Section title="Lanes" count={v!.lanes.length} actions={<button type="button" className="btn btn-g" style={{ height: 24 }} onClick={setup} disabled={!!pending}>Add two lanes</button>}>
          {v!.lanes.length ? v!.lanes.map((l) => <div key={l.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) auto" }}><span style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="t-sm">{l.role_label}</span><span className="mono t-mono-sm i3 trunc">{l.id}</span></span><Status kind={glyphFor(l.status)} weight="weak">{l.status}</Status></div>) : <p className="t-sm i3">No lanes. Each lane is an agent on the fixture pack bound to a different ingested source.</p>}
        </Section>
        <Section title="Fleet runs" count={v!.fleets.length}>
          {v!.fleets.map((f) => <button type="button" key={f.id} className={`lr clickable ${sel === f.id ? "sel" : ""}`} style={{ gridTemplateColumns: "minmax(0,1fr) auto", width: "100%", textAlign: "left" }} onClick={() => setSel(f.id)}><span className="mono t-mono-sm trunc">{f.id}</span><Status kind={glyphFor(f.status)} weight="weak">{f.status}</Status></button>)}
          {!v!.fleets.length && <p className="t-sm i3">No fleet runs yet.</p>}
        </Section>
      </div>
      <div>
        <form onSubmit={create} className="field-row" style={{ marginBottom: 18 }}>
          <input type="text" value={prompt} onChange={(e) => setPrompt(e.target.value)} placeholder="Task for every lane" maxLength={2000} aria-label="Fleet task" style={{ flex: 1 }} />
          <button type="submit" className="btn btn-p" disabled={!prompt.trim() || !!pending}>Create fleet run</button>
        </form>
        {cur ? <>
          <Ledger items={[{ k: "fleet", v: cur.id, mono: true }, { k: "status", v: <Status kind={glyphFor(cur.status)} weight="strong">{cur.status}</Status> }, { k: "revision", v: cur.revision, mono: true }]} />
          <div className="row-gap" style={{ gap: 8, marginBottom: 16 }}>
            <button type="button" className="btn btn-s" onClick={dispatch} disabled={!!pending || cur.status !== "pending" || !v!.lanes.length}>Dispatch to active lanes</button>
            <button type="button" className="btn btn-s" onClick={compare} disabled={!!pending}>Compare lanes</button>
            <button type="button" className="btn btn-g" onClick={cancel} disabled={!!pending || ["completed", "cancelled", "failed"].includes(cur.status)}>Cancel</button>
          </div>
          {detail.state.status === "error" && <ErrorState error={detail.state.error} onRetry={detail.reload} />}
          {detail.state.status === "ready" && <>
            <Section title="Lane runs" count={detail.state.data.lanes.length}>
              {detail.state.data.lanes.length ? detail.state.data.lanes.map((l) => (
                <div key={l.lane_id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) minmax(0,1fr) 120px auto" }}><span className="mono t-mono-sm trunc">{l.lane_id}</span><span className="mono t-mono-sm i3 trunc">{l.run_id}</span><Status kind={glyphFor(l.run_status)}>{l.run_status}</Status>
                  <button type="button" className="btn btn-g" disabled={!!pending || l.run_status !== "running"} onClick={() => execLane(l.lane_id)}>Execute</button></div>
              )) : <p className="t-sm i3">Not dispatched yet.</p>}
            </Section>
            <Section title="Comparison" count={detail.state.data.report ? detail.state.data.report.id : "none"}>
              {detail.state.data.report ? <>
                <KV keyWidth={110} rows={[["participating", detail.state.data.report.participating], ["excluded", detail.state.data.report.excluded || "none"]]} />
                {detail.state.data.report.observations.map((x, i) => <div key={i} className="lr" style={{ gridTemplateColumns: "minmax(0, 170px) minmax(0, 110px) minmax(0,1fr)", padding: "8px 12px", alignItems: "start" }}><span className="t-sm wrap-any">{x.kind}</span><span className="mono t-mono-sm i3">{x.lanes}</span><span className="t-sm i2 wrap-any">{x.detail}</span></div>)}
              </> : <p className="t-sm i3">Compare after lanes complete. A comparison over a pending fleet is refused, never faked.</p>}
            </Section>
          </>}
          <Authority>Lanes are independent proposals. A comparison records agreement and disagreement; it does not choose a winner or promote output.</Authority>
        </> : <Empty icon="fleet" title="No fleet run selected">Add lanes, then create a fleet run with a task for every lane.</Empty>}
      </div>
    </div>
  );
}

/** Turn payloads are Core JSON. Prompts read as text, model output as a labelled prediction table; anything else is pretty-printed, never reinterpreted. */
function TurnPayload({ payload }: { payload: string }) {
  let v: Record<string, unknown> | null = null;
  try { const parsed: unknown = JSON.parse(payload); if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) v = parsed as Record<string, unknown>; } catch { /* raw text */ }
  if (!v) return <span className="mono t-mono-sm i2" style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>{payload}</span>;
  if (typeof v.prompt === "string" && Object.keys(v).length === 1) return <span className="t-sm">“{v.prompt}”</span>;
  const preds = Array.isArray(v.predictions) ? (v.predictions as Array<Record<string, unknown>>) : null;
  if (preds) {
    const { predictions: _p, note, ...meta } = v;
    return (
      <div className="stack" style={{ gap: 8, minWidth: 0 }}>
        {typeof note === "string" && <Status kind="proposal" weight="strong">{note}</Status>}
        <KV keyWidth={128} rows={Object.entries(meta).map(([k, x]) => [k, <span className="mono t-mono-sm wrap-any">{String(x)}</span>])} />
        <table className="tb"><thead><tr><th style={{ width: "18%" }}>#</th><th style={{ width: "40%" }}>Token</th><th>Label</th></tr></thead>
          <tbody>{preds.map((x, i) => <tr key={i}><td className="mono t-mono-sm i3">{i + 1}</td><td className="mono t-mono-sm">{String(x.token ?? "—")}</td><td className="t-sm">{String(x.label ?? "—")} <span className="mono t-mono-sm i3">{x.label_index !== undefined ? `idx ${String(x.label_index)}` : ""}</span></td></tr>)}</tbody></table>
      </div>
    );
  }
  return <pre className="pre" style={{ maxHeight: 220 }}>{JSON.stringify(v, null, 2)}</pre>;
}
