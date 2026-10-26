import { useEffect, useRef, useState, type FormEvent } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { selectedPathOrPrevious } from "../audio-picker";
import { formatStorage } from "../model-storage";
import { mayStartModelDownload } from "../model-consent";
import { call, useAction, useCommand } from "../lib/ipc";
import { ProjectScope } from "../components/project";
import { Authority, Empty, ErrorState, Glyph, KV, Ledger, Notice, PageHead, Section, Skeleton, Stamp, Status, glyphFor, useOutcome, useToast, type GlyphKind } from "../components/ui";

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

type AsrStatus = { routes: Array<{ route: string; available: boolean; reason: string | null }>; asr_pack_dir: string | null };
type AudioModel = { id: string; name: string; repository: string; revision: string; languages: string; download_budget_bytes: number; required_space_bytes: number; installed: boolean; pack_files_present: boolean; cache_entry_exists: boolean };
type AudioModelPreflight = { model: AudioModel; free_bytes: number; required_space_bytes: number; enough_space: boolean; download_started: false; download_requires_explicit_consent: true };
type ModelDownloadStatus = { state: string; model_id: string | null; bytes_downloaded: number; bytes_total: number; pack_dir: string | null; message: string | null };
type Segment = { seq: number; start_ms: number; end_ms: number; status: string; text: string | null; speaker?: { kind: string; index?: number } };
const speakerLabel = (s: Segment["speaker"]) => (s?.kind === "anonymous" && s.index !== undefined ? `speaker ${s.index}` : null);
type Transcript = { revision_id: string; revision_no: number; state: string; segments: Segment[] };
type AudioRetentionPreview = { project_id: string; source_id: string; source_kind: string; recording_bytes: number; revision_count: number; receipt_count: number; delete_available: false; active_vault_only: boolean; older_backups_may_retain_recording: boolean; linked_evidence_requires_separate_retention_policy: boolean; message: string };
type TranscribeResult = { decision: { outcome: string; reason?: string | null }; engine: { engine_id: string; is_fixture: boolean } | null; limitations: string[]; transcript: Transcript | null };

function AudioBody({ projectId }: { projectId: string }) {
  const o = useCommand<AudioVm>("audio_overview", { projectId });
  const asr = useCommand<AsrStatus>("audio_asr_status");
  const availableModels = useCommand<AudioModel[]>("audio_model_catalog");
  const [chosenModel, setChosenModel] = useState("whisper-tiny-en");
  const [storageCheck, setStorageCheck] = useState<AudioModelPreflight | null>(null);
  const [downloadApproved, setDownloadApproved] = useState(false);
  const [downloadStatus, setDownloadStatus] = useState<ModelDownloadStatus | null>(null);
  const activatedPack = useRef<string | null>(null);
  const [seg, setSeg] = useState<{ summary: string; lines: string[] } | null>(null);
  const [packPath, setPackPath] = useState("");
  const [wavPath, setWavPath] = useState("");
  const [choosing, setChoosing] = useState<"pack" | "wav" | null>(null);
  const [route, setRoute] = useState("local_asr_pack");
  const [language, setLanguage] = useState<"en" | "ar">("en");
  const [diarize, setDiarize] = useState(false);
  const [result, setResult] = useState<TranscribeResult | null>(null);
  const [edit, setEdit] = useState<{ seq: number; text: string; reason: string } | null>(null);
  const [retentionPreview, setRetentionPreview] = useState<AudioRetentionPreview | null>(null);
  const models = useCommand<Array<{ id: string; runnable: boolean; fixture: boolean }>>("medagent_models");
  const [notePack, setNotePack] = useState("");
  const [draft, setDraft] = useState<{ sections: Record<string, Array<{ text: string; segment_seq: number; start_ms: number; end_ms: number }>>; segments_evaluated: number; note: string } | null>(null);
  const runnable = models.state.status === "ready" ? models.state.data.filter((m) => m.runnable) : [];
  useEffect(() => { if (!notePack && runnable[0]) setNotePack(runnable[0].id); }, [runnable, notePack]);
  async function draftNote() {
    if (!result?.transcript || !notePack) return;
    const r = await run<NonNullable<typeof draft>>("draft", "audio_draft_note", { revisionId: result.transcript.revision_id, packId: notePack });
    if (outcome(r, "Draft note created · proposal")) setDraft(r.data);
  }
  const { pending, run } = useAction();
  const outcome = useOutcome();
  const toast = useToast();
  useEffect(() => {
    let mounted = true;
    async function refresh() {
      try {
        const status = await call<ModelDownloadStatus>("audio_model_download_status");
        if (mounted) setDownloadStatus(status);
      } catch {
        // The named Core command is unavailable on older native builds.
      }
    }
    void refresh();
    const timer = window.setInterval(() => { void refresh(); }, 1500);
    return () => { mounted = false; window.clearInterval(timer); };
  }, []);
  useEffect(() => {
    if (downloadStatus?.state !== "ready" || !downloadStatus.pack_dir ||
        activatedPack.current === downloadStatus.pack_dir) return;
    const dir = downloadStatus.pack_dir;
    activatedPack.current = dir;
    // The signed Pack is already on disk. Surface "Activate saved model"
    // even when Core activation fails, so the user can retry after reopening
    // the vault without downloading the same checkpoint a second time.
    void availableModels.reload();
    void run("activate-model", "audio_admit_asr_pack", { path: dir }).then(async (r) => {
      if (outcome(r, "Verified Whisper model activated")) await asr.reload();
    });
  }, [downloadStatus?.state, downloadStatus?.pack_dir, run, outcome, asr.reload]);
  async function startModelDownload() {
    if (!mayStartModelDownload(downloadApproved, chosenModel, storageCheck?.model.id ?? null,
        storageCheck?.enough_space === true, !!pending || downloadStatus?.state === "downloading" ||
        downloadStatus?.state === "verifying", selectedModel?.cache_entry_exists === true)) return;
    const r = await run("download-model", "audio_model_download_start", { modelId: chosenModel, consent: true });
    if (outcome(r, "Download started · pinned Whisper source")) {
      setDownloadApproved(false);
      setDownloadStatus({ state: "downloading", model_id: chosenModel, bytes_downloaded: 0,
        bytes_total: storageCheck?.model.download_budget_bytes ?? 0, pack_dir: null, message: null });
    }
  }
  async function activateExistingModel() {
    const r = await run("activate-saved-model", "audio_model_activate_existing", { modelId: chosenModel });
    if (outcome(r, "Saved Whisper model verified and activated")) {
      await asr.reload();
      await availableModels.reload();
    }
  }
  async function cancelModelDownload() {
    const r = await run("cancel-model", "audio_model_download_cancel");
    if (outcome(r, "Cancellation requested · cleaning temporary files")) {
      setDownloadApproved(false);
    }
  }
  async function checkModelStorage() {
    const r = await run<AudioModelPreflight>("storage", "audio_model_preflight", { modelId: chosenModel });
    if (outcome(r, "Storage checked · no download started")) setStorageCheck(r.data);
  }
  async function browseWav() {
    setChoosing("wav");
    try {
      const selected = await open({ multiple: false, directory: false, title: "Choose a WAV recording", filters: [{ name: "PCM WAV", extensions: ["wav"] }] });
      // Cancel is not an error and never overwrites a previously entered path.
      setWavPath((previous) => selectedPathOrPrevious(previous, selected));
    } catch {
      toast({ kind: "blocked", title: "Cannot open file chooser", detail: "You can still enter an absolute WAV path." });
    } finally { setChoosing(null); }
  }
  async function browsePack() {
    setChoosing("pack");
    try {
      const selected = await open({ multiple: false, directory: true, title: "Choose a verified MedScale Audio Pack folder" });
      setPackPath((previous) => selectedPathOrPrevious(previous, selected));
    } catch {
      toast({ kind: "blocked", title: "Cannot open folder chooser", detail: "You can still enter an absolute Audio Pack path." });
    } finally { setChoosing(null); }
  }
  const local = asr.state.status === "ready" ? asr.state.data.routes.find((r) => r.route === "local_asr_pack") : undefined;
  const selectedModel = availableModels.state.status === "ready"
    ? availableModels.state.data.find((m) => m.id === chosenModel) : undefined;
  async function importSynthetic() { const r = await run("import", "audio_import_synthetic", { projectId }); if (outcome(r, "Synthetic tone imported")) await o.reload(); }
  async function segment(id: string) { const r = await run<{ summary: string; lines: string[] }>(id, "audio_segment", { projectId, sourceId: id }); if (outcome(r, "Segmentation recorded")) setSeg(r.data); }
  async function admitPack(e: FormEvent) { e.preventDefault(); const r = await run("pack", "audio_admit_asr_pack", { path: packPath }); if (outcome(r, "Audio Pack admitted · local recognition on")) { setPackPath(""); await asr.reload(); } }
  async function importWav(e: FormEvent) { e.preventDefault(); const r = await run("wav", "audio_import_wav", { projectId, path: wavPath }); if (outcome(r, "Recording imported")) { setWavPath(""); await o.reload(); } }
  async function inspectStoredData(id: string) {
    const r = await run<AudioRetentionPreview>("stored-" + id, "audio_source_retention_preview", { projectId, sourceId: id });
    if (outcome(r, "Stored recording inspected; no data changed")) setRetentionPreview(r.data);
  }
  async function transcribe(id: string) {
    const r = await run<TranscribeResult>(`t-${id}`, "audio_transcribe", { projectId, sourceId: id, route, language, diarize });
    if (outcome(r, "Transcription recorded · draft")) { setResult(r.data); setEdit(null); }
  }
  async function correct(e: FormEvent) {
    e.preventDefault();
    if (!edit || !result?.transcript) return;
    const r = await run<Transcript>("correct", "audio_correct", { revisionId: result.transcript.revision_id, seq: edit.seq, text: edit.text, reason: edit.reason });
    if (outcome(r, "Correction recorded as a new revision")) { setResult({ ...result, transcript: r.data }); setEdit(null); }
  }
  if (o.state.status === "loading") return <Skeleton />;
  if (o.state.status === "error") return <ErrorState error={o.state.error} onRetry={o.reload} />;
  const v = o.state.data;
  return (
    <>
      <div className="row-gap" style={{ gap: 8, marginBottom: 14 }}><button type="button" className="btn btn-s" onClick={importSynthetic} disabled={!!pending}>{pending === "import" ? <Glyph kind="running" /> : null}Import synthetic tone sample</button><span className="t-sm i3">Generated tone and silence; no microphone, no person.</span></div>
      <Ledger items={[{ k: "routes", v: v.routes }, { k: "native capture", v: v.native_capture }]} />
      <Section title="Audio Model Manager" count={availableModels.state.status === "ready" ? availableModels.state.data.length : "…"}>
        <p className="t-sm i2" style={{ marginBottom: 10 }}>Choose a pinned Whisper model. Check local disk space, approve the exact source, then download, verify and activate it. Previously downloaded Packs can be reactivated without downloading again.</p>
        {availableModels.state.status === "error" ? <ErrorState error={availableModels.state.error} onRetry={availableModels.reload} /> : (
          <div className="stack" style={{ gap: 8 }}>
            <label className="lbl" htmlFor="asr-model-choice">Available speech models</label>
            <select id="asr-model-choice" aria-label="Available speech models" value={chosenModel}
              onChange={(e) => { setChosenModel(e.target.value); setStorageCheck(null); setDownloadApproved(false); }}
              disabled={availableModels.state.status !== "ready" || !!pending}>
              {(availableModels.state.status === "ready" ? availableModels.state.data : []).map((m) => (
                <option key={m.id} value={m.id}>{m.name} · {m.languages}</option>
              ))}
            </select>
            {availableModels.state.status === "ready" && (() => {
              const selected = availableModels.state.data.find((m) => m.id === chosenModel);
              return selected ? <p className="t-sm i3">Download budget: {formatStorage(selected.download_budget_bytes)}; temporary snapshot, signed Pack and free-space reserve: {formatStorage(selected.required_space_bytes)}. Source: {selected.repository} at pinned revision {selected.revision.slice(0, 8)}.</p> : null;
            })()}
            <div><button type="button" className="btn btn-s" onClick={checkModelStorage}
              disabled={availableModels.state.status !== "ready" || !!pending}>
              Check disk budget
            </button></div>
            {storageCheck && <div data-testid="audio-model-preflight" className="t-sm i2">
              {storageCheck.enough_space ? "Enough local storage for the planned download and Pack staging." : "Insufficient free storage; no download should start."}
              {" "}Free: {formatStorage(storageCheck.free_bytes)}; required: {formatStorage(storageCheck.required_space_bytes)}.
              {" "}No download has started.
            </div>}
            {selectedModel?.cache_entry_exists &&
              <p className="t-sm i2" role="status">
                {selectedModel.pack_files_present
                  ? "A saved model Pack is already on this device. Choose Activate saved model to verify it and enable local recognition without downloading again."
                  : "An incomplete local model directory already exists. MedScale will not overwrite it. Inspect the app-owned audio-models folder before attempting another download."}
              </p>}
            <label className="row-gap t-sm" style={{ gap: 8 }}>
              <input type="checkbox" aria-label="Approve Whisper model download"
                checked={downloadApproved} onChange={(e) => setDownloadApproved(e.target.checked)} />
              I approve downloading the selected Apache-2.0 Whisper model from the displayed pinned Hugging Face source to this device, using the stated disk budget.
            </label>
            <div className="row-gap" style={{ gap: 8 }}>
              <button type="button" className="btn btn-s" onClick={startModelDownload}
                disabled={!mayStartModelDownload(downloadApproved, chosenModel, storageCheck?.model.id ?? null,
                  storageCheck?.enough_space === true, !!pending || downloadStatus?.state === "downloading" ||
                  downloadStatus?.state === "verifying", selectedModel?.cache_entry_exists === true)}>Download, verify and install</button>
              <button type="button" className="btn btn-g" onClick={activateExistingModel}
                disabled={!selectedModel?.pack_files_present || !!pending}>Activate saved model</button>
              <button type="button" className="btn btn-g" onClick={cancelModelDownload}
                disabled={!!pending || !["downloading", "verifying"].includes(downloadStatus?.state ?? "")}>Cancel download</button>
            </div>
            {downloadStatus?.state && downloadStatus.state !== "idle" && <div className="t-sm i2" role="status" aria-live="polite">
              Model download: {downloadStatus.state}.
              {" "}{formatStorage(downloadStatus.bytes_downloaded)} received
              {downloadStatus.bytes_total > 0 ? ` of approximately ${formatStorage(downloadStatus.bytes_total)}` : ""}.
              {downloadStatus.message ? ` ${downloadStatus.message}` : ""}
              {downloadStatus.state === "ready" ? " Verified Pack is ready for Core activation." : ""}
            </div>}
            <p className="t-sm i3">Already have a verified Audio Pack? Select its folder below to admit it offline. Downloads remain local, and only the dedicated in-progress cache is deleted on cancellation.</p>
          </div>
        )}
      </Section>
      <Section title="Local speech recognition" count={local?.available ? "on" : "off"}>
        <div className="row-gap" style={{ gap: 10, marginBottom: 8 }}>
          <Status kind={local?.available ? "available" : "unavailable"} weight="strong">{local?.available ? "Local Whisper engine installed" : (local?.reason ?? "Not installed")}</Status>
          {asr.state.status === "ready" && asr.state.data.asr_pack_dir && <span className="mono t-mono-sm i3 trunc">{asr.state.data.asr_pack_dir}</span>}
        </div>
        <div className="grid-2">
          <form onSubmit={admitPack} className="stack" style={{ gap: 6 }}>
            <label className="lbl" htmlFor="ap1">Admit an Audio Pack folder</label>
            <input id="ap1" value={packPath} maxLength={1024} placeholder="Absolute folder path" onChange={(e) => setPackPath(e.target.value)} />
            <div className="row-gap" style={{ gap: 8 }}>
              <button type="button" className="btn btn-g" onClick={browsePack} disabled={!!pending || !!choosing}>Browse folder…</button>
              <button type="submit" className="btn btn-s" disabled={!packPath.trim() || !!pending || !!choosing}>Admit and install</button>
            </div>
          </form>
          <form onSubmit={importWav} className="stack" style={{ gap: 6 }}>
            <label className="lbl" htmlFor="wp1">Import a WAV recording</label>
            <input id="wp1" value={wavPath} maxLength={1024} placeholder="Absolute .wav path (16 kHz, 16-bit PCM, up to 32 MiB)" onChange={(e) => setWavPath(e.target.value)} />
            <div className="row-gap" style={{ gap: 8 }}>
              <button type="button" className="btn btn-g" onClick={browseWav} disabled={!!pending || !!choosing}>Browse WAV…</button>
              <button type="submit" className="btn btn-s" disabled={!wavPath.trim() || !!pending || !!choosing}>Import</button>
            </div>
          </form>
        </div>
        <p className="t-sm i3" style={{ marginTop: 6 }}>Import PCM WAV (16 kHz, 16-bit, up to 32 MiB / 30 minutes). Core verifies the Audio Pack and transcribes on this device. Choose the language before transcription.</p>
      </Section>
      <Section title="Sources" count={v.sources.length} actions={
        <select aria-label="Transcription route" value={route} onChange={(e) => setRoute(e.target.value)} style={{ height: 26 }}>
          <option value="local_asr_pack">Local Whisper (Audio Pack)</option>
          <option value="fixture_asr">Fixture engine (not recognition)</option>
          <option value="segmentation_only">Segmentation only</option>
        </select>}>
        <label className="row-gap t-sm" style={{ gap: 8, marginBottom: 8 }}>
          Transcription language
          <select aria-label="Transcription language" value={language} onChange={(e) => setLanguage(e.target.value as "en" | "ar")}>
            <option value="en">English</option>
            <option value="ar">Arabic (requires multilingual Audio Pack)</option>
          </select>
        </label>
        <label className="row-gap t-sm" style={{ gap: 6, marginBottom: 8 }}>
          <input type="checkbox" checked={diarize} onChange={(e) => setDiarize(e.target.checked)} />
          Group speakers (acoustic clustering, unvalidated; labels are never identity)
        </label>
        {v.sources.length ? v.sources.map((s) => <div key={s.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) minmax(0,1.4fr) 120px auto auto auto" }}><span style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="trunc" style={{ fontWeight: 600 }}>{s.label}</span><span className="mono t-mono-sm i3 trunc">{s.id}</span></span><span className="t-sm i3 trunc">{s.kind} · {s.detail}</span><Status kind={glyphFor(s.health)} weight="weak">{s.health}</Status><button type="button" className="btn btn-g" disabled={!!pending} onClick={() => segment(s.id)}>Segment</button><button type="button" className="btn btn-g" disabled={!!pending} onClick={() => inspectStoredData(s.id)}>Stored data</button><button type="button" className="btn btn-s" disabled={!!pending} onClick={() => transcribe(s.id)}>{pending === `t-${s.id}` ? <Glyph kind="running" /> : null}Transcribe</button></div>) : <p className="t-sm i3">No audio sources in this project.</p>}
        {retentionPreview?.project_id === projectId && v.sources.some((s) => s.id === retentionPreview.source_id) && (
          <p className="t-sm i2" role="status" aria-live="polite">
            Stored audio: {formatStorage(retentionPreview.recording_bytes)} · transcript revisions: {retentionPreview.revision_count} · audit receipts: {retentionPreview.receipt_count}.
            {" "}Read-only inspection; no deletion is available and no data was changed. Older backups may still contain this recording. Transcripts and audit receipts require separate retention handling.
          </p>
        )}
      </Section>
      {seg && <Section title="Segmentation" count="draft"><p className="t-sm" style={{ marginBottom: 8 }}>{seg.summary}</p><pre className="pre">{seg.lines.join("\n") || "No segments."}</pre></Section>}
      {result && <Section title="Transcript" count={result.transcript ? `revision ${result.transcript.revision_no} · ${result.transcript.state}` : result.decision.outcome}>
        <Ledger items={[{ k: "decision", v: `${result.decision.outcome}${result.decision.reason ? ` · ${result.decision.reason}` : ""}` }, { k: "engine", v: result.engine ? `${result.engine.engine_id}${result.engine.is_fixture ? " (fixture)" : ""}` : "none", mono: true }, { k: "limitations", v: result.limitations.join(", ") }]} />
        {result.transcript?.segments.map((x) => (
          <div key={x.seq} className="lr" style={{ gridTemplateColumns: "36px 130px minmax(0,1fr) auto", alignItems: "start", padding: "8px 12px" }}>
            <span className="mono t-mono-sm i3">{x.seq}</span>
            <span className="mono t-mono-sm i3">{x.start_ms}–{x.end_ms} ms{speakerLabel(x.speaker) ? ` · ${speakerLabel(x.speaker)}` : ""}</span>
            <span className="t-sm">{x.text ?? <span className="i3">(untranscribed)</span>}</span>
            <button type="button" className="btn btn-g" onClick={() => setEdit({ seq: x.seq, text: x.text ?? "", reason: "" })}>Correct</button>
          </div>
        ))}
        {result.transcript && <div className="row-gap" style={{ gap: 8, marginTop: 12 }}>
          <select aria-label="Pack for the draft note" value={notePack} onChange={(e) => setNotePack(e.target.value)}>{runnable.map((m) => <option key={m.id} value={m.id}>{m.id}</option>)}</select>
          <button type="button" className="btn btn-s" disabled={!notePack || !!pending} onClick={draftNote}>{pending === "draft" ? <Glyph kind="running" /> : null}Draft note from this revision</button>
        </div>}
        {draft && <div style={{ border: "1px dashed var(--line-strong)", borderRadius: "var(--r-2)", padding: "12px 14px", marginTop: 12 }}>
          <Status kind="proposal" weight="strong">Draft note · proposal</Status>
          <p className="t-sm i3" style={{ margin: "6px 0 10px" }}>{draft.note} Segments evaluated: {draft.segments_evaluated}.</p>
          {Object.keys(draft.sections).length ? Object.entries(draft.sections).map(([label, items]) => (
            <div key={label} style={{ marginBottom: 8 }}>
              <div className="lbl">{label}</div>
              {items.map((i, k) => <div key={k} className="row-gap t-sm" style={{ gap: 8 }}><span>{i.text}</span><span className="mono t-mono-sm i3">segment {i.segment_seq} · {i.start_ms}–{i.end_ms} ms</span></div>)}
            </div>
          )) : <p className="t-sm i3">No entities found in the transcribed segments.</p>}
        </div>}
        {edit && <form onSubmit={correct} className="stack" style={{ gap: 6, marginTop: 10, maxWidth: 640 }}>
          <label className="lbl" htmlFor="ct">Segment {edit.seq} text</label>
          <input id="ct" value={edit.text} maxLength={2000} onChange={(e) => setEdit({ ...edit, text: e.target.value })} />
          <label className="lbl" htmlFor="cr">Reason</label>
          <input id="cr" value={edit.reason} maxLength={200} placeholder="For example: reviewed against the recording" onChange={(e) => setEdit({ ...edit, reason: e.target.value })} />
          <div className="row-gap" style={{ gap: 8 }}><button type="submit" className="btn btn-p" disabled={!edit.text.trim() || !edit.reason.trim() || !!pending}>Record correction</button><button type="button" className="btn btn-g" onClick={() => setEdit(null)}>Cancel</button></div>
        </form>}
      </Section>}
      <Section title="Captures" count={v.captures.length}>
        {v.captures.length ? v.captures.map((c) => <div key={c.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) 140px minmax(0,1.4fr)" }}><span className="trunc">{c.label}</span><Status kind={glyphFor(c.state)}>{c.state}</Status><span className="t-sm i3 trunc">{c.detail}</span></div>) : <p className="t-sm i3">No captures. Native capture requires an admitted backend.</p>}
      </Section>
      <Notice kind="proposal" title="Drafts only.">Segments and transcripts carry a receipt and stay drafts until a person corrects and accepts them. Medical accuracy is unmeasured.</Notice>
      <Authority>Audio never becomes a clinical fact on its own. Lineage is kept for every derived artifact.</Authority>
    </>
  );
}
