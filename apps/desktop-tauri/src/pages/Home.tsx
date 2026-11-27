import { useMemo } from "react";
import { useApp } from "../lib/app";
import { useCommand } from "../lib/ipc";
import { MOD } from "../components/shell";
import { Empty, ErrorState, Glyph, KV, PageHead, ResizableSplit, Section, SegmentedBar, Skeleton, Stamp, Status, type SegmentItem } from "../components/ui";
import { CoverageStrip, COVERAGE } from "./Patients";

type Subjects = { subjects: Array<{ subject_ref: string; display_name: string; condition_summary: string; coverage_slots: Array<{ concept_key: string; status: string }> | null; latest_event: string | null }> };
type Corpus = { source_identity: string; documents: Array<{ doc_id: string; status: string; conflict_marker: string | null; freshness_marker: string | null }> };
type Models = { models: Array<{ model: string; state: string; trust: string; task: string }> };
type Workflow = { tasks: Array<{ action_id: string; title: string; state: string }>; outbox_entries: number };

export function Home() {
  const { ws, navigate } = useApp();
  const subjects = useCommand<Subjects>("patients_list");
  const corpus = useCommand<Corpus>("evidence_corpus");
  const models = useCommand<Models>("models_overview");
  const workflow = useCommand<Workflow>("workflow_overview");

  type Item = { key: string; glyph: "needs-review" | "proposal"; title: string; kind: string; why: string; stampK?: string; stampV?: string; act: string; go: () => void };
  const queue: Item[] = [];
  if (subjects.state.status === "ready") {
    for (const s of subjects.state.data.subjects) {
      const gaps = (s.coverage_slots ?? []).filter((x) => x.status !== "present");
      if (gaps.length) queue.push({
        key: s.subject_ref,
        glyph: "needs-review",
        title: s.display_name,
        kind: "Subject coverage",
        why: summarizeGaps(gaps.map((g) => COVERAGE[g.status]?.label ?? g.status)),
        act: "Open",
        go: () => navigate("Patients", s.subject_ref),
      });
    }
  }
  if (corpus.state.status === "ready") {
    for (const d of corpus.state.data.documents.filter((d) => d.conflict_marker)) {
      queue.push({
        key: d.doc_id,
        glyph: "needs-review",
        title: d.doc_id,
        kind: "Evidence conflict",
        why: `Unresolved conflict (${d.conflict_marker}); left visible and never auto-promoted.`,
        stampK: "corpus",
        stampV: corpus.state.data.source_identity,
        act: "Inspect",
        go: () => navigate("Evidence"),
      });
    }
  }
  if (models.state.status === "ready") {
    for (const m of models.state.data.models.filter((m) => /candidate|proposal/i.test(m.state))) {
      queue.push({
        key: m.model,
        glyph: "proposal",
        title: m.model,
        kind: "Model proposal",
        why: `${m.state}. Admitted locally, awaiting explicit task promotion.`,
        stampK: "trust",
        stampV: m.trust,
        act: "Inspect",
        go: () => navigate("Models"),
      });
    }
  }
  if (workflow.state.status === "ready") {
    for (const t of workflow.state.data.tasks) {
      queue.push({
        key: t.action_id,
        glyph: "needs-review",
        title: t.title,
        kind: "Review task",
        why: t.state,
        stampK: "action",
        stampV: t.action_id,
        act: "Review",
        go: () => navigate("Tasks"),
      });
    }
  }
  const loadingQueue = [subjects, corpus, models, workflow].some((c) => c.state.status === "loading");

  const coverageSegments = useMemo<SegmentItem[]>(() => {
    if (subjects.state.status !== "ready") return [];
    let present = 0, conflict = 0, absent = 0, unknown = 0, other = 0;
    for (const s of subjects.state.data.subjects) {
      for (const slot of s.coverage_slots ?? []) {
        if (slot.status === "present") present++;
        else if (slot.status === "conflict") conflict++;
        else if (slot.status === "absent") absent++;
        else if (slot.status === "unknown") unknown++;
        else other++;
      }
    }
    return [
      { key: "present", label: "Present", count: present, tone: "present" },
      { key: "conflict", label: "Conflict", count: conflict, tone: "attention" },
      { key: "other", label: "Incomparable / unsupported", count: other, tone: "hatched" },
      { key: "absent", label: "Absent", count: absent, tone: "neutral" },
      { key: "unknown", label: "Unknown", count: unknown, tone: "muted" },
    ];
  }, [subjects.state]);

  const totalSlots = coverageSegments.reduce((sum, s) => sum + s.count, 0);
  const reviewCount = queue.filter((q) => q.glyph === "needs-review").length;
  const proposalCount = queue.filter((q) => q.glyph === "proposal").length;
  const subjectCount = subjects.state.status === "ready" ? subjects.state.data.subjects.length : null;
  const docCount = corpus.state.status === "ready" ? corpus.state.data.documents.length : null;

  const pane = (
    <div className="pane">
      <PageHead title="Workspace" sub="What needs human review, your current subjects, and verified workspace posture." />

      <div className="metric-rail" role="group" aria-label="Workspace summary">
        <div className="metric-cell">
          <span className="metric-k">Subjects</span>
          <span className="metric-v">{subjectCount ?? "—"}</span>
        </div>
        <div className="metric-cell">
          <span className="metric-k">Needs review</span>
          <span className={`metric-v ${reviewCount > 0 ? "attention" : ""}`}>{loadingQueue && !queue.length ? "—" : reviewCount}</span>
        </div>
        <div className="metric-cell">
          <span className="metric-k">Evidence corpus</span>
          <span className="metric-v">{docCount !== null ? `${docCount} docs` : "—"}</span>
        </div>
        <div className="metric-cell">
          <span className="metric-k">Sources</span>
          <span className="metric-v">{ws?.workspace?.sources ?? "—"}</span>
        </div>
      </div>

      <Section title="Needs a person" count={loadingQueue ? "…" : queue.length}>
        {loadingQueue && !queue.length ? <Skeleton rows={3} /> : queue.length === 0 ? (
          <p className="t-sm i3" style={{ padding: "12px 0" }}>
            {[subjects, corpus, models, workflow].some((c) => c.state.status === "error")
              ? "Some sources could not be read, so this list may be incomplete."
              : "Nothing is waiting for review right now."}
          </p>
        ) : (
          <div style={{ borderTop: "1px solid var(--line-subtle)" }}>
            {queue.map((r) => (
              <div key={r.key} className="lr home-row" style={{ minHeight: "var(--row-2)" }}>
                <Glyph kind={r.glyph} />
                <div className="home-row-meta">
                  <span className="trunc" style={{ fontSize: 13.5, fontWeight: 600 }}>{r.title}</span>
                  <span className="st weak" style={{ fontSize: 11.5 }}>{r.kind}</span>
                </div>
                <div className="home-row-meta">
                  <span className="t-sm i2 clamp-2" title={r.why}>{r.why}</span>
                  {r.stampK && r.stampV && <Stamp k={r.stampK} v={r.stampV} />}
                </div>
                <button type="button" className="btn btn-g" style={{ justifySelf: "end" }} onClick={r.go}>{r.act}</button>
              </div>
            ))}
          </div>
        )}
        {[subjects, corpus, models, workflow].map((c, i) => c.state.status === "error" ? <ErrorState key={i} error={c.state.error} onRetry={c.reload} /> : null)}
      </Section>

      <Section
        title="Subjects"
        count={subjectCount !== null ? subjectCount : "…"}
        actions={<button type="button" className="btn btn-g" onClick={() => navigate("Patients")}>Open Patients</button>}
      >
        {subjects.state.status === "loading" && <Skeleton rows={2} />}
        {subjects.state.status === "ready" && subjects.state.data.subjects.length === 0 && (
          <Empty
            icon="patients"
            title="No subjects in this workspace"
            action={<button type="button" className="btn btn-s" onClick={() => navigate("Documents")}>Open Documents</button>}
          >
            Import synthetic FHIR records in Documents to populate subjects, timelines, and coverage checks.
          </Empty>
        )}
        {subjects.state.status === "ready" && subjects.state.data.subjects.length > 0 && (
          <>
            {totalSlots > 0 && (
              <div style={{ marginBottom: 14 }}>
                <SegmentedBar
                  title="Record coverage across subjects"
                  meta={`${coverageSegments[0]?.count ?? 0} of ${totalSlots} slots present`}
                  items={coverageSegments}
                />
              </div>
            )}
            <table className="tb">
              <thead>
                <tr>
                  <th>Subject</th>
                  <th style={{ width: "26%" }} className="col-p2">Condition</th>
                  <th style={{ width: 146 }}>Coverage</th>
                  <th style={{ width: 106 }} className="num">Latest</th>
                </tr>
              </thead>
              <tbody>
                {subjects.state.data.subjects.map((s) => (
                  <tr key={s.subject_ref} className="r2 clickable" tabIndex={0} aria-label={`Open patient ${s.display_name}`} onClick={() => navigate("Patients", s.subject_ref)} onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); navigate("Patients", s.subject_ref); } }}>
                    <td>
                      <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
                        <span className="pname trunc">{s.display_name}</span>
                        <span className="mono t-mono-sm i3 trunc">{s.subject_ref}</span>
                      </div>
                    </td>
                    <td className="t-sm i2 col-p2"><span className="trunc">{s.condition_summary}</span></td>
                    <td><CoverageStrip slots={s.coverage_slots} /></td>
                    <td className="num mono t-mono-sm i3">{s.latest_event ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </>
        )}
      </Section>

      <Section title="Quick workspaces">
        <div className="row-gap" style={{ gap: 8 }}>
          <button type="button" className="btn btn-s" onClick={() => navigate("Documents")}>Documents & FHIR</button>
          <button type="button" className="btn btn-s" onClick={() => navigate("Evidence")}>Evidence corpus</button>
          <button type="button" className="btn btn-s" onClick={() => navigate("Models")}>Models</button>
          <button type="button" className="btn btn-s" onClick={() => navigate("Audio")}>Local audio</button>
          <button type="button" className="btn btn-s" onClick={() => navigate("Knowledge")}>Research Canvas</button>
        </div>
      </Section>
    </div>
  );

  const inspector = (
    <div className="ins-pad">
      <Section title="Workspace state">
        <KV keyWidth={78} rows={[
          ["workspace", <Status kind={ws?.workspace?.kind === "synthetic" ? "synthetic" : "available"} weight="strong">{ws?.workspace?.label ?? "None"}</Status>],
          ["vault", <><span className="mono t-mono-sm">{ws?.workspace?.vault_id}</span><div className="t-sm i3">{ws?.workspace?.kind === "encrypted" ? "Encrypted at rest (Argon2id 64 MiB)" : "Sample records only · not encrypted"}</div></>],
          ["runtime", <><Status kind={ws?.local_only ? "available" : "unknown"} weight="strong">{ws?.local_only ? "Local" : "Unknown"}</Status><div className="t-sm i3">Runs on this device</div></>],
          ["network", <><Status kind={ws?.network_default_deny ? "blocked" : "unknown"}>{ws?.network_default_deny ? "Blocked by default" : "Not verified"}</Status>{ws?.egress && <div className="t-sm i3">{ws.egress}</div>}</>],
          ["Real data", <Status kind="limited">{ws?.real_phi_authorized ? "Authorized" : "Not authorized"}</Status>],
          ["evidence", corpus.state.status === "ready" ? <><span className="mono t-mono-sm">{corpus.state.data.source_identity}</span><div className="t-sm i3">{corpus.state.data.documents.length} docs · {corpus.state.data.documents.filter((d) => d.status === "retracted").length} retracted · bundled sample</div></> : "…"],
        ]} />
      </Section>
      <Section title="Authority">
        <div className="stack" style={{ gap: 10 }}>
          <div style={{ display: "flex", justifyContent: "space-between" }}><Status kind="proposal">Proposals open</Status><span className="mono t-mono-sm tnum">{proposalCount}</span></div>
          <div style={{ display: "flex", justifyContent: "space-between" }}><Status kind="needs-review">Needs review</Status><span className="mono t-mono-sm tnum">{reviewCount}</span></div>
          <div style={{ display: "flex", justifyContent: "space-between" }}><Status kind="pending">Pending outbox</Status><span className="mono t-mono-sm tnum">{workflow.state.status === "ready" ? workflow.state.data.outbox_entries : "…"}</span></div>
          <p className="t-sm i3" style={{ marginTop: 4 }}>MedScale takes no external action on its own. Consequential work goes through explicit review, and if an outcome is unknown, it stops rather than retrying.</p>
        </div>
      </Section>
      <Section title="Keyboard">
        <div style={{ display: "grid", gridTemplateColumns: "100px minmax(0,1fr)", rowGap: 8, alignItems: "center", fontSize: 12.5, color: "var(--ink-2)" }}>
          <span className="kbd" style={{ width: "max-content" }}>{MOD} K</span><span>Search or run a command</span>
          <span className="kbd" style={{ width: "max-content" }}>{MOD} B</span><span>Toggle sidebar</span>
          <span className="kbd" style={{ width: "max-content" }}>/</span><span>Focus page search</span>
          <span className="kbd" style={{ width: "max-content" }}>{MOD} ,</span><span>Settings</span>
          <span className="kbd" style={{ width: "max-content" }}>{MOD} Shift L</span><span>Lock workspace</span>
        </div>
      </Section>
    </div>
  );

  return <ResizableSplit pane={pane} inspector={inspector} inspectorLabel="Workspace state" />;
}

/** "3 Conflict · 1 Incomparable units" — counts per coverage status, most frequent first. */
function summarizeGaps(labels: string[]): string {
  const counts = new Map<string, number>();
  for (const l of labels) counts.set(l, (counts.get(l) ?? 0) + 1);
  return [...counts].sort((a, b) => b[1] - a[1]).map(([l, n]) => `${n} ${l.toLowerCase()}`).join(" · ");
}
