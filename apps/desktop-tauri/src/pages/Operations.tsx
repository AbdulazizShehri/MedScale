import { useEffect, useState, type FormEvent } from "react";
import { useAction, useCommand } from "../lib/ipc";
import { ProjectScope } from "../components/project";
import { Authority, Empty, ErrorState, Glyph, KV, Ledger, Notice, PageHead, Section, Skeleton, Stamp, Status, glyphFor, useOutcome, type GlyphKind } from "../components/ui";

type WorkflowVm = {
  synthetic_only: boolean; workflow_name: string; workflow_state: string; outbox_summary: string;
  action_boundary: string; task_boundary: string; message_boundary: string; outbox_entries: number;
  steps: Array<{ order: string; title: string; detail: string; state: string }>;
  tasks: Array<{ action_id: string; title: string; state: string; payload_digest: string; next_step: string; reconcile_required: boolean }>;
  messages: Array<{ title: string; body: string; status: string; related_action_id: string }>;
};

const stepGlyph = (s: string): GlyphKind => /read-only/i.test(s) ? "limited" : /required/i.test(s) ? "needs-review" : /bound/i.test(s) ? "present" : /fail closed/i.test(s) ? "blocked" : "available";

function useWorkflow() { return useCommand<WorkflowVm>("workflow_overview"); }

export function Workflows() {
  const { state, reload } = useWorkflow();
  if (state.status === "loading") return <div className="pane"><Skeleton /></div>;
  if (state.status === "error") return <div className="pane"><ErrorState error={state.error} onRetry={reload} /></div>;
  const v = state.data;
  return (
    <>
      <div className="pane">
        <PageHead title={v.workflow_name} sub={v.workflow_state}><Status kind={v.outbox_entries ? "running" : "pending"}>{v.outbox_entries} outbox entr{v.outbox_entries === 1 ? "y" : "ies"}</Status></PageHead>
        <Section n="01" title="Steps" count={v.steps.length}>
          <ol style={{ listStyle: "none", margin: 0, padding: 0 }}>
            {v.steps.map((s, i) => (
              <li key={s.order} style={{ display: "grid", gridTemplateColumns: "36px 28px minmax(0,1fr) 170px", columnGap: 12, minHeight: 76 }}>
                <span className="mono" style={{ fontSize: 11, color: "var(--ink-3)", paddingTop: 16 }}>{s.order}</span>
                <span style={{ position: "relative", display: "flex", justifyContent: "center" }}>
                  <span style={{ position: "absolute", top: i === 0 ? 24 : 0, bottom: i === v.steps.length - 1 ? "calc(100% - 24px)" : 0, left: "50%", width: 1, background: "var(--line-strong)" }} />
                  <span style={{ position: "relative", marginTop: 18, width: 12, height: 12, display: "flex", alignItems: "center", justifyContent: "center", background: "var(--bg-surface)" }}><span className="g g-present" style={{ width: 7, height: 7 }} /></span>
                </span>
                <div style={{ padding: "13px 0 14px", borderBottom: "1px solid var(--line-subtle)" }}><div className="t-h3" style={{ fontSize: 13.5 }}>{s.title}</div><p className="t-sm i2" style={{ marginTop: 3, maxWidth: 640 }}>{s.detail}</p></div>
                <div style={{ paddingTop: 14, borderBottom: "1px solid var(--line-subtle)" }}><Status kind={stepGlyph(s.state)} weight="strong">{s.state}</Status></div>
              </li>
            ))}
          </ol>
        </Section>
        <p className="t-sm i3">{v.outbox_summary}</p>
      </div>
      <aside className="inspector collapsible" aria-label="Boundaries">
        <div className="ins-pad">
          <Section title="Boundaries"><KV keyWidth={78} rows={[["action", v.action_boundary], ["tasks", v.task_boundary], ["messages", v.message_boundary]]} /></Section>
          <Section title="Effect states">
            <div className="stack" style={{ gap: 12 }}>
              <Status kind="needs-review" weight="strong">Action awaiting review</Status>
              <div><Status kind="running">Awaiting effect confirmation</Status><p className="t-sm i3" style={{ margin: "2px 0 0 16px" }}>Do not duplicate-send.</p></div>
              <div><Status kind="completed">Effect confirmed</Status><p className="t-sm i3" style={{ margin: "2px 0 0 16px" }}>Kept as history.</p></div>
              <div><Status kind="failed">Effect failed</Status><p className="t-sm i3" style={{ margin: "2px 0 0 16px" }}>Review before an explicit restart.</p></div>
              <div><Status kind="blocked" weight="strong">Effect state UNKNOWN</Status><p className="t-sm i3" style={{ margin: "2px 0 0 16px" }}>Reconcile before any retry. Blind retry is forbidden.</p></div>
            </div>
          </Section>
        </div>
      </aside>
    </>
  );
}

export function Tasks() {
  const { state, reload } = useWorkflow();
  return (
    <div className="pane">
      <PageHead title="Tasks" sub="Review items created when a workflow prepares an action. Each names the payload digest it would act on." />
      {state.status === "loading" && <Skeleton />}
      {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
      {state.status === "ready" && (state.data.tasks.length ? (
        <table className="tb"><thead><tr><th style={{ width: "28%" }}>Task</th><th style={{ width: "20%" }}>State</th><th className="col-p3" style={{ width: "18%" }}>Payload digest</th><th style={{ width: "22%" }}>Next step</th><th>Reconcile</th></tr></thead>
          <tbody>{state.data.tasks.map((t) => (
            <tr key={t.action_id} className="r2"><td><div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="t-h3 trunc">{t.title}</span><span className="mono t-mono-sm i3 trunc">{t.action_id}</span></div></td>
              <td><Status kind={glyphFor(t.state)} weight="strong">{t.state}</Status></td><td className="col-p3"><Stamp k="sha256" v={t.payload_digest.slice(0, 14)} /></td><td className="t-sm i2"><span className="trunc">{t.next_step}</span></td>
              <td>{t.reconcile_required ? <Status kind="blocked">Required</Status> : <Status kind="present" weight="weak">No</Status>}</td></tr>
          ))}</tbody></table>
      ) : <Empty icon="tasks" title="No review tasks">Nothing is waiting for a person. Tasks appear when a workflow prepares an action in the Core Host outbox; this desktop never creates external actions itself.</Empty>)}
      <Section n="02" title="Safe actions only"><KV keyWidth={88} rows={[["review", "Opens evidence and the payload"], ["approve", "Writes intent to Core Host; Desktop sends nothing itself"], ["retry", "Never automatic. Not offered while the effect is UNKNOWN."]]} /></Section>
    </div>
  );
}

export function Messages() {
  const { state, reload } = useWorkflow();
  return (
    <div className="pane">
      <PageHead title="Messages" sub="Message previews tied to workflow actions. Previews are not transport receipts." />
      {state.status === "loading" && <Skeleton />}
      {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
      {state.status === "ready" && <>
        {state.data.messages.length ? state.data.messages.map((m, i) => (
          <div key={i} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) minmax(0,2fr) 140px auto", minHeight: "var(--row-2)" }}>
            <span className="t-h3 trunc">{m.title}</span><span className="t-sm i2">{m.body}</span><Status kind={glyphFor(m.status)}>{m.status}</Status><Stamp k="action" v={m.related_action_id || "—"} />
          </div>
        )) : <Empty icon="messages" title="No message previews">Previews appear for actions in the outbox.</Empty>}
        <p className="t-sm i3" style={{ marginTop: 12 }}>{state.data.message_boundary}</p>
      </>}
    </div>
  );
}

/* ───────────────────────────── Collaboration ───────────────────────────── */
type Room = { id: string; name: string; revision: number };
type RoomDetail = { threads: Array<{ id: string; status: string; resolution: string; artifact_id: string }>; tasks: Array<{ id: string; title: string; status: string; revision: number }> };
type Sources = { sources: Array<{ source_id: string; resource_type: string; subject_ref: string }> };

export function Collaboration() {
  return <ProjectScope title="Collaboration" sub="Local rooms, source-anchored threads and tasks. Presence and identity are not claimed.">{(pid) => <CollabBody projectId={pid} />}</ProjectScope>;
}

function CollabBody({ projectId }: { projectId: string }) {
  const rooms = useCommand<Room[]>("collab_rooms", { projectId });
  const sources = useCommand<Sources>("documents_list");
  const [room, setRoom] = useState<string | null>(null);
  const [thread, setThread] = useState<string | null>(null);
  const detail = useCommand<RoomDetail>(room ? "collab_room_detail" : null, room ? { roomId: room } : undefined);
  const msgs = useCommand<Array<{ author: string; body: string; seq: number }>>(thread ? "collab_messages" : null, thread ? { threadId: thread } : undefined);
  const [name, setName] = useState(""); const [artifact, setArtifact] = useState(""); const [body, setBody] = useState(""); const [task, setTask] = useState("");
  const { pending, run } = useAction();
  const outcome = useOutcome();
  useEffect(() => { if (rooms.state.status === "ready" && !room && rooms.state.data[0]) setRoom(rooms.state.data[0].id); }, [rooms.state, room]);
  useEffect(() => { if (sources.state.status === "ready" && !artifact && sources.state.data.sources[0]) setArtifact(sources.state.data.sources[0].source_id); }, [sources.state, artifact]);

  async function createRoom(e: FormEvent) { e.preventDefault(); const r = await run<Room>("room", "collab_create_room", { projectId, name }); if (outcome(r, "Room created")) { setName(""); await rooms.reload(); setRoom(r.data.id); } }
  async function openThread() { if (!room || !artifact) return; const r = await run<{ id: string }>("thread", "collab_open_thread", { roomId: room, artifactId: artifact }); if (outcome(r, "Thread opened")) { await detail.reload(); setThread(r.data.id); } }
  async function post(e: FormEvent) { e.preventDefault(); if (!thread) return; const r = await run("post", "collab_post_message", { threadId: thread, body }); if (outcome(r, "Message posted")) { setBody(""); await msgs.reload(); } }
  async function addTask(e: FormEvent) { e.preventDefault(); if (!room) return; const r = await run("task", "collab_create_task", { roomId: room, title: task }); if (outcome(r, "Task created")) { setTask(""); await detail.reload(); } }
  async function complete(id: string, revision: number) { const r = await run("done", "collab_complete_task", { taskId: id, revision }); if (outcome(r, "Task completed")) await detail.reload(); }

  return (
    <div style={{ display: "grid", gridTemplateColumns: "260px minmax(0,1fr) minmax(0,1fr)", gap: 24 }}>
      <div>
        <Section title="Rooms" count={rooms.state.status === "ready" ? rooms.state.data.length : "…"}>
          {rooms.state.status === "error" && <ErrorState error={rooms.state.error} onRetry={rooms.reload} />}
          {rooms.state.status === "ready" && rooms.state.data.map((r) => <button type="button" key={r.id} className={`lr clickable ${room === r.id ? "sel" : ""}`} style={{ gridTemplateColumns: "minmax(0,1fr)", width: "100%", textAlign: "left" }} onClick={() => { setRoom(r.id); setThread(null); }}><span className="trunc" style={{ fontWeight: 600 }}>{r.name}</span></button>)}
          <form onSubmit={createRoom} className="field-row" style={{ marginTop: 10 }}><input type="text" value={name} onChange={(e) => setName(e.target.value)} placeholder="Room name" maxLength={128} aria-label="Room name" style={{ flex: 1 }} /><button type="submit" className="btn btn-s" disabled={!name.trim() || !!pending}>Add</button></form>
        </Section>
      </div>
      <div>
        {!room ? <Empty icon="collaboration" title="No room selected">Create a room to discuss sources in this project.</Empty> : <>
          <Section title="Threads" count={detail.state.status === "ready" ? detail.state.data.threads.length : "…"}>
            <div className="field-row" style={{ marginBottom: 10 }}>
              <select value={artifact} onChange={(e) => setArtifact(e.target.value)} aria-label="Anchor source" style={{ flex: 1 }}>
                {sources.state.status === "ready" && sources.state.data.sources.map((s) => <option key={s.source_id} value={s.source_id}>{s.resource_type} · {s.source_id}</option>)}
              </select>
              <button type="button" className="btn btn-s" onClick={openThread} disabled={!artifact || !!pending}>Open thread</button>
            </div>
            {detail.state.status === "ready" && detail.state.data.threads.map((t) => <button type="button" key={t.id} className={`lr clickable ${thread === t.id ? "sel" : ""}`} style={{ gridTemplateColumns: "minmax(0,1fr) auto", width: "100%", textAlign: "left" }} onClick={() => setThread(t.id)}><Stamp k="on" v={t.artifact_id} /><Status kind={glyphFor(t.status)} weight="weak">{t.status}</Status></button>)}
          </Section>
          <Section title="Tasks" count={detail.state.status === "ready" ? detail.state.data.tasks.length : "…"}>
            <form onSubmit={addTask} className="field-row" style={{ marginBottom: 10 }}><input type="text" value={task} onChange={(e) => setTask(e.target.value)} placeholder="Task title" maxLength={256} aria-label="Task title" style={{ flex: 1 }} /><button type="submit" className="btn btn-s" disabled={!task.trim() || !!pending}>Add</button></form>
            {detail.state.status === "ready" && detail.state.data.tasks.map((t) => <div key={t.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) auto auto" }}><span className="t-sm trunc">{t.title}</span><Status kind={glyphFor(t.status)} weight="weak">{t.status}</Status>{!/done|completed|cancel/i.test(t.status) ? <button type="button" className="btn btn-g" onClick={() => complete(t.id, t.revision)}>Complete</button> : <span />}</div>)}
          </Section>
        </>}
      </div>
      <div>
        {thread ? <Section title="Messages" count={msgs.state.status === "ready" ? msgs.state.data.length : "…"}>
          {msgs.state.status === "ready" && msgs.state.data.map((m) => <div key={m.seq} style={{ padding: "8px 0", borderBottom: "1px solid var(--line-subtle)" }}><span className="mono t-mono-sm i3">{m.author} · #{m.seq}</span><p className="t-sm" style={{ marginTop: 2 }}>{m.body}</p></div>)}
          {msgs.state.status === "ready" && !msgs.state.data.length && <p className="t-sm i3">No messages yet.</p>}
          <form onSubmit={post} className="stack" style={{ gap: 8, marginTop: 10 }}><textarea value={body} onChange={(e) => setBody(e.target.value)} placeholder="Write a message" maxLength={4000} style={{ minHeight: 64 }} aria-label="Message" /><div><button type="submit" className="btn btn-p" disabled={!body.trim() || !!pending}>Post</button></div></form>
        </Section> : <Empty icon="messages" title="No thread selected">Open a thread anchored to an ingested source.</Empty>}
      </div>
    </div>
  );
}

/* ───────────────────────────── Audio ───────────────────────────── */
type AudioVm = { sources: Array<{ id: string; label: string; kind: string; detail: string; health: string }>; captures: Array<{ id: string; label: string; state: string; detail: string }>; routes: string; native_capture: string };

export function Audio() {
  return <ProjectScope title="Audio" sub="Local audio lineage. Segmentation and transcripts are drafts, never clinical truth.">{(pid) => <AudioBody projectId={pid} />}</ProjectScope>;
}

function AudioBody({ projectId }: { projectId: string }) {
  const o = useCommand<AudioVm>("audio_overview", { projectId });
  const [seg, setSeg] = useState<{ summary: string; lines: string[] } | null>(null);
  const { pending, run } = useAction();
  const outcome = useOutcome();
  async function importSynthetic() { const r = await run("import", "audio_import_synthetic", { projectId }); if (outcome(r, "Synthetic tone imported")) await o.reload(); }
  async function segment(id: string) { const r = await run<{ summary: string; lines: string[] }>(id, "audio_segment", { projectId, sourceId: id }); if (outcome(r, "Segmentation recorded")) setSeg(r.data); }
  if (o.state.status === "loading") return <Skeleton />;
  if (o.state.status === "error") return <ErrorState error={o.state.error} onRetry={o.reload} />;
  const v = o.state.data;
  return (
    <>
      <div className="row-gap" style={{ gap: 8, marginBottom: 14 }}><button type="button" className="btn btn-s" onClick={importSynthetic} disabled={!!pending}>{pending === "import" ? <Glyph kind="running" /> : null}Import synthetic tone sample</button><span className="t-sm i3">Generated tone and silence; no microphone, no person.</span></div>
      <Ledger items={[{ k: "routes", v: v.routes }, { k: "native capture", v: v.native_capture }]} />
      <Section title="Sources" count={v.sources.length}>
        {v.sources.length ? v.sources.map((s) => <div key={s.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) minmax(0,1.4fr) 120px auto" }}><span style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="trunc" style={{ fontWeight: 600 }}>{s.label}</span><span className="mono t-mono-sm i3 trunc">{s.id}</span></span><span className="t-sm i3 trunc">{s.kind} · {s.detail}</span><Status kind={glyphFor(s.health)} weight="weak">{s.health}</Status><button type="button" className="btn btn-g" disabled={!!pending} onClick={() => segment(s.id)}>Segment</button></div>) : <p className="t-sm i3">No audio sources in this project.</p>}
      </Section>
      {seg && <Section title="Segmentation" count="draft"><p className="t-sm" style={{ marginBottom: 8 }}>{seg.summary}</p><pre className="pre">{seg.lines.join("\n") || "No segments."}</pre></Section>}
      <Section title="Captures" count={v.captures.length}>
        {v.captures.length ? v.captures.map((c) => <div key={c.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) 140px minmax(0,1.4fr)" }}><span className="trunc">{c.label}</span><Status kind={glyphFor(c.state)}>{c.state}</Status><span className="t-sm i3 trunc">{c.detail}</span></div>) : <p className="t-sm i3">No captures. Native capture requires an admitted backend.</p>}
      </Section>
      <Notice kind="proposal" title="Drafts only.">Segments and transcripts carry a receipt and stay drafts until a person corrects and accepts them.</Notice>
      <Authority>Audio never becomes a clinical fact on its own. Lineage is kept for every derived artifact.</Authority>
    </>
  );
}
