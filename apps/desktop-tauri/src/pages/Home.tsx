import { useApp } from "../lib/app";
import { useCommand } from "../lib/ipc";
import { MOD } from "../components/shell";
import { ErrorState, Glyph, KV, Mark, Section, Skeleton, Stamp, Status } from "../components/ui";
import { CoverageStrip, COVERAGE } from "./Patients";

type Subjects = { subjects: Array<{ subject_ref: string; display_name: string; condition_summary: string; coverage_slots: Array<{ concept_key: string; status: string }> | null; latest_event: string | null }> };
type Corpus = { source_identity: string; documents: Array<{ doc_id: string; status: string; conflict_marker: string | null; freshness_marker: string | null }> };
type Models = { models: Array<{ model: string; state: string; trust: string; task: string }> };
type Workflow = { tasks: Array<{ action_id: string; title: string; state: string }>; outbox_entries: number };

export function Home() {
  const { ws, navigate, openPalette } = useApp();
  const subjects = useCommand<Subjects>("patients_list");
  const corpus = useCommand<Corpus>("evidence_corpus");
  const models = useCommand<Models>("models_overview");
  const workflow = useCommand<Workflow>("workflow_overview");

  type Item = { key: string; glyph: "needs-review" | "proposal"; title: string; kind: string; why: string; stampK: string; stampV: string; act: string; go: () => void };
  const queue: Item[] = [];
  if (subjects.state.status === "ready") {
    for (const s of subjects.state.data.subjects) {
      const gaps = (s.coverage_slots ?? []).filter((x) => x.status !== "present");
      if (gaps.length) queue.push({
        key: s.subject_ref, glyph: "needs-review", title: s.display_name, kind: "Subject coverage",
        why: summarizeGaps(gaps.map((g) => COVERAGE[g.status]?.label ?? g.status)),
        stampK: "proj", stampV: "SubjectCoverageV1", act: "Open", go: () => navigate("Patients", s.subject_ref),
      });
    }
  }
  if (corpus.state.status === "ready") {
    for (const d of corpus.state.data.documents.filter((d) => d.conflict_marker)) {
      queue.push({ key: d.doc_id, glyph: "needs-review", title: d.doc_id, kind: "Evidence document", why: `Conflict marker ${d.conflict_marker}. Left unresolved; nothing is promoted.`, stampK: "corpus", stampV: corpus.state.data.source_identity, act: "Inspect", go: () => navigate("Evidence") });
    }
  }
  if (models.state.status === "ready") {
    for (const m of models.state.data.models.filter((m) => /candidate|proposal/i.test(m.state))) {
      queue.push({ key: m.model, glyph: "proposal", title: m.model, kind: "Model pack · proposal", why: `${m.state}. Admitted locally, not promoted to a task.`, stampK: "trust", stampV: m.trust, act: "Inspect", go: () => navigate("Models") });
    }
  }
  if (workflow.state.status === "ready") {
    for (const t of workflow.state.data.tasks) {
      queue.push({ key: t.action_id, glyph: "needs-review", title: t.title, kind: "Review task", why: t.state, stampK: "action", stampV: t.action_id, act: "Review", go: () => navigate("Tasks") });
    }
  }
  const loadingQueue = [subjects, corpus, models, workflow].some((c) => c.state.status === "loading");

  return (
    <>
      <div className="pane">
        <div className="page-head">
          <div className="grow">
            <div className="brand-hero" style={{ marginBottom: 6 }}><Mark width={30} className="mark" /><span className="brand-rule" aria-hidden="true" /></div>
            <h1 className="t-h1">Workspace</h1>
            <p className="ph-sub">What needs a person, what changed, and what this workspace can and cannot assert.</p>
          </div>
          <button type="button" className="btn btn-g" onClick={openPalette}>Search or run <span className="kbd">{MOD} K</span></button>
        </div>

        <Section n="01" title="Needs review" count={loadingQueue ? "…" : queue.length}>
          {loadingQueue && !queue.length ? <Skeleton rows={3} /> : queue.length === 0 ? <p className="t-sm i3" style={{ padding: "12px 0" }}>Nothing is waiting for a person.</p> : (
            <div style={{ borderTop: "1px solid var(--line-subtle)" }}>
              {queue.map((r) => (
                <div key={r.key} className="lr" style={{ gridTemplateColumns: "16px minmax(150px, 1fr) minmax(0, 1.6fr) minmax(0, 200px) 72px", minHeight: "var(--row-2)" }}>
                  <Glyph kind={r.glyph} />
                  <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="trunc" style={{ fontSize: 13.5, fontWeight: 600 }}>{r.title}</span><span className="st weak" style={{ fontSize: 11.5 }}>{r.kind}</span></div>
                  <span className="t-sm i2 clamp-2" style={{ minWidth: 0 }} title={r.why}>{r.why}</span>
                  <span className="hide-md" style={{ minWidth: 0, overflow: "hidden" }}><Stamp k={r.stampK} v={r.stampV} /></span>
                  <button type="button" className="btn btn-g" style={{ justifySelf: "end" }} onClick={r.go}>{r.act}</button>
                </div>
              ))}
            </div>
          )}
          {[subjects, corpus, models, workflow].map((c, i) => c.state.status === "error" ? <ErrorState key={i} error={c.state.error} onRetry={c.reload} /> : null)}
        </Section>

        <Section n="02" title="Subjects" count={subjects.state.status === "ready" ? subjects.state.data.subjects.length : "…"} actions={<button type="button" className="btn btn-g" style={{ height: 24 }} onClick={() => navigate("Patients")}>Open Patients</button>}>
          {subjects.state.status === "ready" && (
            <table className="tb"><tbody>
              {subjects.state.data.subjects.map((s) => (
                <tr key={s.subject_ref} className="r2 clickable" tabIndex={0} onClick={() => navigate("Patients", s.subject_ref)} onKeyDown={(e) => e.key === "Enter" && navigate("Patients", s.subject_ref)}>
                  <td style={{ width: "36%" }}><div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="pname trunc">{s.display_name}</span><span className="mono t-mono-sm i3 trunc">{s.subject_ref}</span></div></td>
                  <td style={{ width: "28%" }} className="t-sm i2 col-p2"><span className="trunc">{s.condition_summary}</span></td>
                  <td style={{ width: "20%" }}><CoverageStrip slots={s.coverage_slots} /></td>
                  <td className="num mono t-mono-sm i3">{s.latest_event ?? "—"}</td>
                </tr>
              ))}
            </tbody></table>
          )}
        </Section>
      </div>

      <aside className="inspector" aria-label="Workspace state">
        <div className="ins-pad">
          <Section title="Workspace state">
            <KV keyWidth={104} rows={[
              ["workspace", <Status kind={ws?.workspace?.kind === "synthetic" ? "synthetic" : "available"} weight="strong">{ws?.workspace?.label ?? "None"}</Status>],
              ["vault", <><span className="mono t-mono-sm">{ws?.workspace?.vault_id}</span><div className="t-sm i3">{ws?.workspace?.kind === "encrypted" ? "Encrypted at rest" : "Synthetic fixtures only"}</div></>],
              ["runtime", <><Status kind={ws?.local_only ? "available" : "unknown"} weight="strong">{ws?.local_only ? "Local" : "Unknown"}</Status><div className="t-sm i3">Core in-process</div></>],
              ["network", <Status kind={ws?.network_default_deny ? "blocked" : "unknown"}>{ws?.network_default_deny ? "Default deny" : "Not verified"}</Status>],
              ["real PHI", <Status kind="limited">{ws?.real_phi_authorized ? "Authorized" : "Not authorized"}</Status>],
              ["evidence", corpus.state.status === "ready" ? <><span className="mono t-mono-sm">{corpus.state.data.source_identity}</span><div className="t-sm i3">{corpus.state.data.documents.length} docs · {corpus.state.data.documents.filter((d) => d.status === "retracted").length} retracted</div></> : "…"],
            ]} />
          </Section>
          <Section title="Authority">
            <div className="stack" style={{ gap: 10 }}>
              <div style={{ display: "flex", justifyContent: "space-between" }}><Status kind="proposal">Proposals open</Status><span className="mono t-mono-sm">{queue.filter((q) => q.glyph === "proposal").length}</span></div>
              <div style={{ display: "flex", justifyContent: "space-between" }}><Status kind="needs-review">Needs review</Status><span className="mono t-mono-sm">{queue.filter((q) => q.glyph === "needs-review").length}</span></div>
              <div style={{ display: "flex", justifyContent: "space-between" }}><Status kind="pending">Outbox entries</Status><span className="mono t-mono-sm">{workflow.state.status === "ready" ? workflow.state.data.outbox_entries : "…"}</span></div>
              <p className="t-sm i3" style={{ marginTop: 4 }}>Desktop commits no external action. Consequential work goes through explicit review and the Core Host outbox. UNKNOWN effects fail closed.</p>
            </div>
          </Section>
          <Section title="Keyboard">
            <div style={{ display: "grid", gridTemplateColumns: "100px minmax(0,1fr)", rowGap: 8, alignItems: "center", fontSize: 12.5, color: "var(--ink-2)" }}>
              <span className="kbd" style={{ width: "max-content" }}>{MOD} K</span><span>Search or run a command</span>
              <span className="kbd" style={{ width: "max-content" }}>{MOD} ,</span><span>Settings</span>
              <span className="kbd" style={{ width: "max-content" }}>{MOD} Shift L</span><span>Lock workspace</span>
            </div>
          </Section>
        </div>
      </aside>
    </>
  );
}

/** "3 Conflict · 1 Incomparable units" — counts per coverage status, most frequent first. */
function summarizeGaps(labels: string[]): string {
  const counts = new Map<string, number>();
  for (const l of labels) counts.set(l, (counts.get(l) ?? 0) + 1);
  return [...counts].sort((a, b) => b[1] - a[1]).map(([l, n]) => `${n} ${l.toLowerCase()}`).join(" · ");
}
