import { useEffect, useState, type FormEvent, type KeyboardEvent, type ReactNode } from "react";
import { formatDecimalMegabytes } from "../model-storage";
import { useAction, useCommand, type CmdError } from "../lib/ipc";
import { ProjectScope } from "../components/project";
import { ConfirmPanel } from "../components/confirm";
import { MOD } from "../components/shell";
import {
  Authority,
  CopyButton,
  Disclosure,
  Empty,
  ErrorState,
  Glyph,
  Icon,
  KV,
  Ledger,
  LifecycleStepper,
  LoadError,
  Notice,
  PageHead,
  ResizableSplit,
  Section,
  SegmentedBar,
  Skeleton,
  Status,
  glyphFor,
  useOutcome,
  type SegmentItem,
} from "../components/ui";

/* ───────────────────────────── Models ───────────────────────────── */
type ModelsVm = {
  model_runtime_summary: string;
  model_runtime_boundary: string;
  model_source_summary: string;
  model_inventory_summary: string;
  models: Array<{
    scope: string;
    task: string;
    model: string;
    source: string;
    runtime: string;
    device: string;
    trust: string;
    benchmark: string;
    digest: string;
    state: string;
  }>;
};

type CatalogRow = {
  medscale_id: string;
  repo_id: string;
  task: string | null;
  family: string | null;
  formats: string[];
  tier: string | null;
  status: string;
  license_claim: string | null;
  device_fit: string;
  disk: { kind: string; megabytes?: number } | null;
  runtime_expectation: { kind: string; [k: string]: unknown };
};
type CatalogPage = {
  repository: string;
  commit: string;
  total_rows: number;
  total_matching: number;
  next_offset: number | null;
  status_counts: Record<string, number>;
  rows: CatalogRow[];
};

type EvalResult = {
  proposal_payload: Record<string, unknown>;
  runtime_id: string;
  prepared_cache_hit: boolean;
  audit_id: string;
};

export function Models() {
  const { state, reload } = useCommand<ModelsVm>("models_overview");
  const [sel, setSel] = useState(0);
  const [tab, setTab] = useState<"packs" | "catalog" | "evaluate">("packs");
  const [selectedCatalog, setSelectedCatalog] = useState<CatalogRow | null>(null);
  const [evalSummary, setEvalSummary] = useState<{ packId: string; result: EvalResult; entityCount: number } | null>(null);
  const { pending, run } = useAction();
  const outcome = useOutcome();
  async function admit() {
    const r = await run("admit", "models_admit_fixture_pack");
    if (outcome(r, "Fixture pack submitted to Core")) await reload();
  }

  if (state.status === "loading") return <div className="pane"><Skeleton /></div>;
  if (state.status === "error") return <div className="pane"><ErrorState error={state.error} onRetry={reload} /></div>;
  const v = state.data;
  const m = v.models[sel] ?? v.models[0];

  const inspectorNode: ReactNode = tab === "catalog" ? (
    selectedCatalog ? (
      <div className="ins-pad">
        <div>
          <div style={{ fontSize: 15, fontWeight: 600, letterSpacing: "-0.01em" }}>{catalogDisplayName(selectedCatalog.repo_id)}</div>
          <div className="mono t-mono-sm i3 wrap-any" style={{ marginTop: 4 }}>{selectedCatalog.repo_id}</div>
          <div className="row-gap" style={{ gap: 10, marginTop: 10, flexWrap: "wrap" }}>
            <Status kind={selectedCatalog.status === "DISCOVERABLE" ? "available" : "limited"} weight="strong">{selectedCatalog.status}</Status>
            <CopyButton text={selectedCatalog.repo_id} label="Copy repo ID" />
          </div>
        </div>
        <Section title="Catalog record">
          <KV keyWidth={102} rows={[
            ["Catalog ID", <span className="mono t-mono-sm wrap-any">{selectedCatalog.medscale_id}</span>],
            ["task", selectedCatalog.task ?? "—"],
            ["family", selectedCatalog.family ?? "—"],
            ["tier", selectedCatalog.tier ?? "—"],
            ["license", selectedCatalog.license_claim ?? "See model card"],
            ["formats", <span className="mono t-mono-sm">{selectedCatalog.formats.join(", ") || "—"}</span>],
            ["disk", <span className="mono t-mono-sm">{formatDecimalMegabytes(selectedCatalog.disk?.megabytes)}</span>],
            ["device fit", selectedCatalog.device_fit],
            ["expectation", selectedCatalog.runtime_expectation.kind.replace(/_/g, " ")],
          ]} />
        </Section>
        <Authority>A catalog listing is not a download or endorsement. Rights and clinical suitability follow each model&apos;s own license and evaluation evidence.</Authority>
      </div>
    ) : (
      <div className="ins-pad">
        <Empty icon="models" title="Select a catalog model">Click any row in the MedScale Catalog to inspect its formats, disk budget, license claim, and pre-download expectation.</Empty>
      </div>
    )
  ) : tab === "evaluate" ? (
    <div className="ins-pad">
      {evalSummary ? (
        <>
          <div>
            <div className="t-h2">Evaluation receipt</div>
            <div className="row-gap" style={{ gap: 10, marginTop: 8 }}>
              <Status kind="proposal" weight="strong">Proposal only</Status>
              <Status kind={evalSummary.result.prepared_cache_hit ? "available" : "pending"} weight="weak">
                {evalSummary.result.prepared_cache_hit ? "Warm cache" : "Cold run"}
              </Status>
            </div>
          </div>
          <Section title="Run metadata">
            <KV keyWidth={96} rows={[
              ["pack", <span className="mono t-mono-sm wrap-any">{evalSummary.packId}</span>],
              ["runtime", <span className="mono t-mono-sm wrap-any">{evalSummary.result.runtime_id}</span>],
              ["entities", <span className="mono t-mono-sm">{evalSummary.entityCount}</span>],
              ["Audit ID", <span className="row-gap" style={{ gap: 6, flexWrap: "wrap" }}><span className="mono t-mono-sm wrap-any">{evalSummary.result.audit_id}</span><CopyButton text={evalSummary.result.audit_id} label="Copy" /></span>],
            ]} />
          </Section>
          <Authority>Evaluation output is synthetic evidence only. Nothing is saved to a patient record or promoted.</Authority>
        </>
      ) : (
        <Empty icon="models" title="Ready to evaluate">Choose an admitted pack and run it on synthetic text. Results appear with inline entity highlights and a full audit receipt.</Empty>
      )}
    </div>
  ) : m ? (
    <div className="ins-pad">
      <div>
        <div className="mono" style={{ fontSize: 13.5, fontWeight: 500, wordBreak: "break-word" }}>{m.model}</div>
        <div className="row-gap" style={{ gap: 12, marginTop: 8, flexWrap: "wrap" }}>
          <Status kind={/candidate|proposal/i.test(m.state) ? "proposal" : glyphFor(m.state)} weight="strong">{m.state}</Status>
          <Status kind={/synthetic/i.test(m.source) ? "synthetic" : "available"}>{m.source}</Status>
        </div>
      </div>
      <Section title="Identity">
        <KV keyWidth={96} rows={[
          ["scope", m.scope],
          ["task", m.task],
          ["runtime", <span className="mono t-mono-sm">{m.runtime}</span>],
          ["device", m.device],
          ["trust", m.trust],
          ["digest", <span className="stack" style={{ gap: 6 }}><span className="mono-break">{m.digest}</span>{m.digest && m.digest !== "—" && m.digest.toLowerCase() !== "none" && <CopyButton text={m.digest} label="Copy digest" />}</span>],
        ]} />
      </Section>
      <Section title="Evidence"><p className="t-sm i2">{m.benchmark}</p></Section>
      <Authority>Admitting a pack doesn&apos;t promote it. Promotion to a task needs an explicit gate that this desktop doesn&apos;t grant.</Authority>
    </div>
  ) : null;

  return (
    <ResizableSplit
      inspectorLabel={tab === "catalog" ? "Selected catalog model" : tab === "evaluate" ? "Evaluation receipt" : "Selected model"}
      defaultWidth={376}
      inspector={inspectorNode}
      pane={
        <div className="pane">
          <PageHead title="Models" sub="Model packs admitted in this session, a catalog to browse, and a way to try a model on synthetic text.">
            <button type="button" className="btn btn-s" onClick={admit} disabled={!!pending}>{pending ? <Glyph kind="running" /> : null}Admit synthetic fixture pack</button>
          </PageHead>
          <Disclosure summary="How models are governed">
            <Ledger items={[{ k: "runtime", v: v.model_runtime_summary }, { k: "source", v: v.model_source_summary }, { k: "inventory", v: v.model_inventory_summary }]} />
          </Disclosure>
          <div
            className="tabs"
            role="tablist"
            aria-label="Models views"
            style={{ marginTop: 14, marginBottom: 14 }}
            onKeyDown={(e) => {
              const order: Array<"packs" | "catalog" | "evaluate"> = ["packs", "catalog", "evaluate"];
              const idx = order.indexOf(tab);
              let nextIdx = -1;
              if (e.key === "ArrowRight") nextIdx = (idx + 1) % order.length;
              else if (e.key === "ArrowLeft") nextIdx = (idx - 1 + order.length) % order.length;
              else if (e.key === "Home") nextIdx = 0;
              else if (e.key === "End") nextIdx = order.length - 1;
              if (nextIdx >= 0) {
                e.preventDefault();
                setTab(order[nextIdx]!);
                e.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]')[nextIdx]?.focus();
              }
            }}
          >
            <button type="button" role="tab" aria-selected={tab === "packs"} tabIndex={tab === "packs" ? 0 : -1} className={`tab ${tab === "packs" ? "on" : ""}`} onClick={() => setTab("packs")}>Admitted packs</button>
            <button type="button" role="tab" aria-selected={tab === "catalog"} tabIndex={tab === "catalog" ? 0 : -1} className={`tab ${tab === "catalog" ? "on" : ""}`} onClick={() => setTab("catalog")}>MedScale Catalog</button>
            <button type="button" role="tab" aria-selected={tab === "evaluate"} tabIndex={tab === "evaluate" ? 0 : -1} className={`tab ${tab === "evaluate" ? "on" : ""}`} onClick={() => setTab("evaluate")}>Try a model</button>
          </div>
          {tab === "packs" && (v.models.length ? (
            <table className="tb">
              <thead>
                <tr>
                  <th style={{ width: "34%" }}>Model</th>
                  <th style={{ width: "20%" }}>Task</th>
                  <th style={{ width: "24%" }}>State</th>
                  <th style={{ width: "22%" }}>Trust</th>
                </tr>
              </thead>
              <tbody>{v.models.map((x, i) => (
                <tr key={x.model + i} className={`r2 clickable ${m === x ? "sel" : ""}`} tabIndex={0} aria-label={`Select model ${x.model}`} aria-selected={m === x} onClick={() => setSel(i)} onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setSel(i); } }}>
                  <td>
                    <div style={{ display: "flex", gap: 10, alignItems: "center", minWidth: 0 }}>
                      <Icon name="models" />
                      <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
                        <span className="mono trunc" style={{ fontSize: 12.5, fontWeight: 500 }} title={x.model}>{x.model}</span>
                        <span className="t-sm i3 trunc" title={x.scope}>{x.scope}</span>
                      </div>
                    </div>
                  </td>
                  <td className="t-sm i2"><span className="trunc" title={x.task}>{x.task}</span></td>
                  <td><Status kind={/candidate|proposal/i.test(x.state) ? "proposal" : glyphFor(x.state)}>{x.state}</Status></td>
                  <td className="t-sm i2"><span className="trunc" title={x.trust}>{x.trust}</span></td>
                </tr>
              ))}</tbody>
            </table>
          ) : <Empty icon="models" title="No admitted packs">Admit the bundled synthetic ONNX fixture pack. Core verifies its manifest, digests and signature before admission.</Empty>)}
          {tab === "catalog" && <CatalogTab selected={selectedCatalog} onSelect={setSelectedCatalog} />}
          {tab === "evaluate" && <EvaluateTab onEvaluated={setEvalSummary} />}
          <Section title="How model output is treated">
            <div className="grid-2">
              <Notice kind="proposal" title="Proposal.">Every output stays a proposal, marked with an outlined triangle. MedScale has no step that accepts one into a record.</Notice>
              <Notice kind="unknown" title="Confidence.">Shown only when the task defines it. Raw logits are never presented as probability.</Notice>
            </div>
            <p className="t-sm i3" style={{ marginTop: 10 }}>{v.model_runtime_boundary}</p>
          </Section>
        </div>
      }
    />
  );
}

/** UI-only MedScale catalog label; the source's immutable repo_id remains unchanged. */
function catalogDisplayName(repoId: string): string {
  const [namespace, ...pathParts] = repoId.split("/");
  const slug = pathParts.at(-1) ?? namespace ?? "";
  const segments = slug.split(/[-_]+/).filter(Boolean);
  // Remove duplicated source-namespace prefixes from user-facing labels only.
  if (segments.length > 1 && segments[0]?.toLowerCase() === namespace?.toLowerCase()) segments.shift();
  return segments.join(" ") || "Model";
}

function CatalogTab({ selected, onSelect }: { selected: CatalogRow | null; onSelect: (row: CatalogRow | null) => void }) {
  const [draft, setDraft] = useState({ text: "", task: "", format: "", status: "" });
  const [query, setQuery] = useState({ text: "", task: "", format: "", status: "", offset: 0 });
  const page = useCommand<CatalogPage>("models_catalog", query);
  const p = page.state.status === "ready" ? page.state.data : null;
  const hasFilters = Boolean(draft.text || draft.task || draft.format || draft.status || query.text || query.task || query.format || query.status);

  const submit = (e: FormEvent) => { e.preventDefault(); setQuery({ ...draft, offset: 0 }); };
  const clearFilters = () => {
    const empty = { text: "", task: "", format: "", status: "" };
    setDraft(empty);
    setQuery({ ...empty, offset: 0 });
  };

  useEffect(() => {
    if (p && p.rows.length > 0 && (!selected || !p.rows.some((r) => r.repo_id === selected.repo_id))) {
      onSelect(p.rows[0]!);
    }
  }, [p, selected, onSelect]);

  const statusSegments: SegmentItem[] = p
    ? Object.entries(p.status_counts).map(([k, count]) => ({
        key: k,
        label: k.replace(/_/g, " ").toLowerCase().replace(/^./, (c) => c.toUpperCase()),
        count,
        tone: k === "DISCOVERABLE" ? "present" : /PENDING|RIGHTS/i.test(k) ? "attention" : /BLOCK|DENY/i.test(k) ? "danger" : "neutral",
      }))
    : [];

  return (
    <Section title="Model catalog" count={p ? `${p.total_matching} of ${p.total_rows}` : "…"}>
      {statusSegments.length > 0 && (
        <div style={{ marginBottom: 14 }}>
          <SegmentedBar title="Catalog status distribution" meta={`${p?.total_rows ?? 0} total`} items={statusSegments} />
        </div>
      )}
      <form onSubmit={submit} className="row-gap" style={{ gap: 8, flexWrap: "wrap", marginBottom: 12 }}>
        <input aria-label="Search the catalog" data-page-search="true" placeholder="Search model, family or language" value={draft.text} maxLength={120} onChange={(e) => setDraft({ ...draft, text: e.target.value })} style={{ minWidth: 230, flex: "1 1 230px" }} />
        <select aria-label="Task" value={draft.task} onChange={(e) => setDraft({ ...draft, task: e.target.value })}>
          <option value="">Any task</option>
          {["token-classification", "text-generation", "zero-shot-token-classification", "text-classification", "feature-extraction", "visual-question-answering"].map((t) => <option key={t} value={t}>{t}</option>)}
        </select>
        <select aria-label="Format" value={draft.format} onChange={(e) => setDraft({ ...draft, format: e.target.value })}>
          <option value="">Any format</option>
          {["onnx", "safetensors", "pytorch", "mlx-fp", "mlx-4bit", "gguf"].map((t) => <option key={t} value={t}>{t}</option>)}
        </select>
        <select aria-label="Status" value={draft.status} onChange={(e) => setDraft({ ...draft, status: e.target.value })}>
          <option value="">Any status</option>
          {Object.keys(p?.status_counts ?? { DISCOVERABLE: 0, RIGHTS_PENDING: 0 }).map((t) => <option key={t} value={t}>{t}</option>)}
        </select>
        <button type="submit" className="btn btn-s">Search</button>
        {hasFilters && <button type="button" className="btn btn-g" onClick={clearFilters}>Clear filters</button>}
      </form>
      {page.state.status === "loading" && <Skeleton />}
      {page.state.status === "error" && <ErrorState error={page.state.error} onRetry={page.reload} />}
      {p && (p.rows.length > 0 ? <>
        <table className="tb">
          <thead>
            <tr>
              <th>Model</th>
              <th style={{ width: "20%" }}>Task</th>
              <th className="col-p3" style={{ width: 96 }}>Formats</th>
              <th style={{ width: 84 }}>Disk</th>
              <th style={{ width: 124 }}>Expectation</th>
              <th style={{ width: 128 }}>Status</th>
            </tr>
          </thead>
          <tbody>{p.rows.map((r) => (
            <tr key={r.repo_id} className={`r2 clickable ${selected?.repo_id === r.repo_id ? "sel" : ""}`} tabIndex={0} aria-label={`Inspect catalog model ${catalogDisplayName(r.repo_id)}`} aria-selected={selected?.repo_id === r.repo_id} onClick={() => onSelect(r)} onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onSelect(r); } }}>
              <td>
                <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
                  <span className="mono trunc" style={{ fontSize: 12.5, fontWeight: 500 }} title={r.repo_id}>{catalogDisplayName(r.repo_id)}</span>
                  <span className="t-sm i3 trunc">{[r.family, r.tier, r.license_claim].filter(Boolean).join(" · ")}</span>
                  <span className="t-sm i3 mono trunc" title="MedScale catalog entry ID (not an assertion of model authorship)">{r.medscale_id}</span>
                </div>
              </td>
              <td className="t-sm i2"><span className="trunc" title={r.task ?? undefined}>{r.task ?? "—"}</span></td>
              <td className="col-p3 mono t-mono-sm i2"><span className="trunc">{r.formats.join(", ")}</span></td>
              <td className="mono t-mono-sm i2">{formatDecimalMegabytes(r.disk?.megabytes)}</td>
              <td className="t-sm i2"><span className="trunc" title={r.runtime_expectation.kind.replace(/_/g, " ")}>{r.runtime_expectation.kind.replace(/_/g, " ")}</span></td>
              <td><Status kind={r.status === "DISCOVERABLE" ? "available" : "limited"} weight="weak">{r.status.replace(/_/g, " ").toLowerCase().replace(/^./, (c) => c.toUpperCase())}</Status></td>
            </tr>
          ))}</tbody>
        </table>
        <div className="row-gap" style={{ gap: 8, marginTop: 10, justifyContent: "space-between", flexWrap: "wrap" }}>
          <span className="t-sm i3">MedScale catalog snapshot <span className="mono">{p.commit.slice(0, 12)}</span> · rows {query.offset + 1}–{query.offset + p.rows.length}</span>
          <span className="row-gap" style={{ gap: 8 }}>
            <button type="button" className="btn btn-g" disabled={query.offset === 0} onClick={() => setQuery({ ...query, offset: Math.max(0, query.offset - 50) })}>Previous</button>
            <button type="button" className="btn btn-g" disabled={p.next_offset == null} onClick={() => setQuery({ ...query, offset: p.next_offset ?? query.offset })}>Next</button>
          </span>
        </div>
      </> : (
        <Empty
          icon="models"
          title="No matching catalog models"
          action={hasFilters ? <button type="button" className="btn btn-s" onClick={clearFilters}>Clear filters</button> : undefined}
        >
          No entries matched the active filters in snapshot <span className="mono">{p.commit.slice(0, 12)}</span>.
        </Empty>
      ))}
      <p className="t-sm i3" style={{ marginTop: 8 }}>A listing is not a download and not an executable model. Each row shows MedScale&apos;s pre-download expectation; rights follow each model&apos;s own card.</p>
    </Section>
  );
}

// "recommended" is Core's task-aware decoding: word-first plus the PII merger
// for PII models, word-first otherwise. It scored at or above the pinned baseline on
// every synthetic head-to-head row (evidence/103-local-model-catalog/eval/).
const DECODINGS: Array<[string, string]> = [
  ["recommended", "Recommended (task-aware)"],
  ["word_first", "Word: first sub-token"],
  ["", "Default ONNX token decoder"],
  ["word_average", "Word: average"],
  ["word_max", "Word: most confident"],
  ["pii_merge", "PII merge"],
  ["word_first_pii_merge", "Word first + PII merge"],
];

/** Renders the evaluated synthetic prose with deterministic inline entity highlights when valid offsets exist. */
function HighlightedEntityProse({ text, entities }: { text: string; entities: Array<Record<string, unknown>> }) {
  const spans = entities
    .map((e) => ({
      start: typeof e.start === "number" ? e.start : Number(e.start),
      end: typeof e.end === "number" ? e.end : Number(e.end),
      label: String(e.label ?? e.entity_group ?? e.entity ?? "ENTITY"),
    }))
    .filter((s) => Number.isFinite(s.start) && Number.isFinite(s.end) && s.start >= 0 && s.end > s.start && s.end <= text.length)
    .sort((a, b) => a.start - b.start || b.end - a.end);

  if (!spans.length) return null;
  const parts: ReactNode[] = [];
  let cursor = 0;
  spans.forEach((s, idx) => {
    if (s.start < cursor) return;
    if (s.start > cursor) parts.push(<span key={`t-${idx}`}>{text.slice(cursor, s.start)}</span>);
    parts.push(
      <mark key={`m-${idx}`} className="entity-mark" title={`${s.label} (${s.start}–${s.end})`}>
        {text.slice(s.start, s.end)}
        <span className="entity-tag">{s.label}</span>
      </mark>,
    );
    cursor = s.end;
  });
  if (cursor < text.length) parts.push(<span key="tail">{text.slice(cursor)}</span>);
  return (
    <div style={{ margin: "12px 0" }}>
      <div className="lbl" style={{ marginBottom: 6 }}>Annotated synthetic text</div>
      <div className="entity-prose">{parts}</div>
    </div>
  );
}

function EvaluateTab({ onEvaluated }: { onEvaluated: (s: { packId: string; result: EvalResult; entityCount: number } | null) => void }) {
  const models = useCommand<Model[]>("medagent_models");
  const [packId, setPackId] = useState("");
  const [input, setInput] = useState("Synthetic note: Ada Example, born 12 March 1980, seen at Utrecht clinic for type 2 diabetes.");
  const [evaluatedText, setEvaluatedText] = useState("");
  const [decoding, setDecoding] = useState("recommended");
  const [labels, setLabels] = useState("");
  const [result, setResult] = useState<EvalResult | null>(null);
  const { pending, run } = useAction();
  const outcome = useOutcome();
  const packs = models.state.status === "ready" ? models.state.data.filter((m) => m.runnable) : [];
  useEffect(() => { if (!packId && packs[0]) setPackId(packs[0].id); }, [packs, packId]);

  async function runEvaluate() {
    if (!packId || !input.trim() || !!pending) return;
    const list = labels.split(",").map((l) => l.trim()).filter(Boolean);
    const currentInput = input;
    const r = await run<EvalResult>("evaluate", "models_evaluate", { packId, input: currentInput, decoding: decoding || null, labels: list.length ? list : null });
    if (outcome(r, "Evaluated on the local runtime · evidence only")) {
      setResult(r.data);
      setEvaluatedText(currentInput);
      const ents = (r.data.proposal_payload.entities ?? r.data.proposal_payload.pii_merged_entities) as Array<Record<string, unknown>> | undefined;
      onEvaluated({ packId, result: r.data, entityCount: ents?.length ?? 0 });
    }
  }
  function evaluate(e: FormEvent) {
    e.preventDefault();
    void runEvaluate();
  }
  function onInputKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      void runEvaluate();
    }
  }

  const entities = (result?.proposal_payload.entities ?? result?.proposal_payload.pii_merged_entities) as Array<Record<string, unknown>> | undefined;
  return (
    <Section title="Try a model" count="synthetic text only">
      <form onSubmit={evaluate} className="stack" style={{ gap: 8, maxWidth: 760 }}>
        <label className="lbl" htmlFor="ep">Admitted Pack</label>
        <select id="ep" value={packId} onChange={(e) => setPackId(e.target.value)}>{packs.map((p) => <option key={p.id} value={p.id}>{p.id} · {p.version}</option>)}</select>
        <label className="lbl" htmlFor="ei">Synthetic input</label>
        <textarea id="ei" value={input} maxLength={4000} onChange={(e) => setInput(e.target.value)} onKeyDown={onInputKeyDown} style={{ minHeight: 84 }} />
        <div className="grid-2">
          <div className="stack" style={{ gap: 4 }}>
            <label className="lbl" htmlFor="ed">Entity decoding (token classification)</label>
            <select id="ed" value={decoding} onChange={(e) => setDecoding(e.target.value)}>{DECODINGS.map(([k, l]) => <option key={k} value={k}>{l}</option>)}</select>
          </div>
          <div className="stack" style={{ gap: 4 }}>
            <label className="lbl" htmlFor="el">Labels (GLiNER zero-shot, comma-separated)</label>
            <input id="el" value={labels} maxLength={600} placeholder="person, date, location" onChange={(e) => setLabels(e.target.value)} />
          </div>
        </div>
        <div className="row-gap" style={{ gap: 10 }}>
          <button type="submit" className="btn btn-p btn-inline" disabled={!packId || !input.trim() || !!pending}>{pending === "evaluate" ? <Glyph kind="running" /> : null}Evaluate</button>
          <span className="t-sm i3">{MOD}+Enter to run on local runtime</span>
        </div>
      </form>
      {result && <div style={{ marginTop: 14 }}>
        <Ledger items={[{ k: "runtime", v: result.runtime_id, mono: true }, { k: "prepared cache", v: result.prepared_cache_hit ? "hit" : "cold", mono: true }, { k: "audit", v: result.audit_id, mono: true }]} />
        {entities?.length ? (
          <>
            <HighlightedEntityProse text={evaluatedText} entities={entities} />
            <table className="tb">
              <thead><tr><th style={{ width: "40%" }}>Text</th><th style={{ width: "30%" }}>Label</th><th>Span</th></tr></thead>
              <tbody>{entities.map((x, i) => <tr key={i}><td className="t-sm">{String(x.text ?? x.word ?? "")}</td><td className="t-sm i2">{String(x.label ?? x.entity_group ?? x.entity ?? "")}</td><td className="mono t-mono-sm i3">{String(x.start ?? "")}–{String(x.end ?? "")}</td></tr>)}</tbody>
            </table>
          </>
        ) : null}
        <Disclosure summary="Full proposal payload"><pre className="pre">{JSON.stringify(result.proposal_payload, null, 2)}</pre></Disclosure>
        <Authority>Evaluation evidence, not a record. Nothing is promoted; synthetic input only.</Authority>
      </div>}
    </Section>
  );
}

/* ───────────────────────────── MedAgent ───────────────────────────── */
type Run = { id: string; status: string; revision: number; prompt: string };
type Turn = { seq: number; kind: string; payload: string };

export function MedAgent() {
  return <ProjectScope title="MedAgent" sub="Agents that run on local model packs, reading only the sources you choose. Output is a proposal.">{(pid) => <MedAgentBody projectId={pid} />}</ProjectScope>;
}

type Agent = { id: string; label: string; status: string; revision?: number; pack_id?: string; tools?: string[] };
type Model = { id: string; version: string; runtime: string; fixture: boolean; runnable: boolean };
type Source = { id: string; fixture: string; resource_type: string; subject: string };
type RunDetail = {
  run: Run;
  context_id: string;
  agent: { id: string; label: string; status: string; revision: number; pack_id: string; pack_version: string; tools: string[] };
  evidence: Array<{ id: string; resolution: string | null; fixture: string | null; resource_type: string | null; subject: string | null }>;
};
const TOOLS: Array<[string, string]> = [["read_context_artifact", "Read a bound source"], ["search_context_artifacts", "Search bound sources"]];

function agentLifecycleSteps(status: string, hasTurns: boolean): Array<{ label: string; state: "done" | "active" | "pending" | "failed" }> {
  const s = status.toLowerCase();
  if (s === "cancelled" || s === "failed") {
    return [
      { label: "Created", state: "done" },
      { label: "Started", state: hasTurns ? "done" : "pending" },
      { label: s === "cancelled" ? "Cancelled" : "Failed", state: "failed" },
    ];
  }
  const pending = s === "pending";
  const running = s === "running";
  const completed = s === "completed";
  return [
    { label: "Pending", state: pending ? "active" : "done" },
    { label: "Running", state: running ? (hasTurns ? "done" : "active") : completed ? "done" : "pending" },
    { label: "Executed", state: running && hasTurns ? "active" : completed ? "done" : "pending" },
    { label: "Completed", state: completed ? "done" : "pending" },
  ];
}

function MedAgentBody({ projectId }: { projectId: string }) {
  const [composing, setComposing] = useState(false);
  useEffect(() => { if (composing) document.getElementById("ag")?.focus(); }, [composing]);
  const runs = useCommand<{ runs: Run[]; agents: Agent[] }>("medagent_runs", { projectId });
  const models = useCommand<Model[]>("medagent_models");
  const sources = useCommand<Source[]>("medagent_sources");
  const [sel, setSel] = useState<string | null>(null);
  const [prompt, setPrompt] = useState("");
  const [agentId, setAgentId] = useState("");
  const [picked, setPicked] = useState<string[]>([]);
  const [proposal, setProposal] = useState<unknown>(null);
  const [refusal, setRefusal] = useState<{ runId: string; error: CmdError } | null>(null);
  const [newAgent, setNewAgent] = useState<{ name: string; packId: string; tools: string[] } | null>(null);
  const [confirmRevoke, setConfirmRevoke] = useState<Agent | null>(null);
  const [packPath, setPackPath] = useState("");
  const desktop = useCommand<{ external_packs_allowed: boolean }>("workspace_status");
  const externalPacks = desktop.state.status === "ready" && desktop.state.data.external_packs_allowed;
  const turns = useCommand<Turn[]>(sel ? "medagent_turns" : null, sel ? { runId: sel } : undefined);
  const detail = useCommand<RunDetail>(sel ? "medagent_run_detail" : null, sel ? { runId: sel } : undefined);
  const { pending, run } = useAction();
  const outcome = useOutcome();
  const list = runs.state.status === "ready" ? runs.state.data.runs : [];
  const agents = runs.state.status === "ready" ? runs.state.data.agents : [];
  const active = agents.filter((a) => a.status === "active");
  const packs = models.state.status === "ready" ? models.state.data : [];
  const srcs = sources.state.status === "ready" ? sources.state.data : [];
  const cur = list.find((r) => r.id === sel) ?? null;
  const d = detail.state.status === "ready" ? detail.state.data : null;
  useEffect(() => { if (!sel && list[0]) setSel(list[0].id); }, [list, sel]);
  useEffect(() => { if (!agentId && active[0]) setAgentId(active[0].id); }, [active, agentId]);
  useEffect(() => { if (!picked.length && srcs.length) setPicked(srcs.slice(0, 2).map((s) => s.id)); }, [srcs, picked.length]);

  const after = async () => { await runs.reload(); await turns.reload(); await detail.reload(); };
  async function createRun() {
    if (!prompt.trim() || !picked.length || picked.length > 16 || !!pending) return;
    const args = agentId ? { projectId, prompt, agentId, sourceIds: picked } : { projectId, prompt, sourceIds: picked };
    const r = await run<Run>("create", "medagent_create_run", args);
    if (outcome(r, "Run created (pending)")) { setPrompt(""); setProposal(null); setComposing(false); await runs.reload(); setSel(r.data.id); document.getElementById("new-run")?.focus(); }
  }
  function create(e: FormEvent) {
    e.preventDefault();
    void createRun();
  }
  async function step(cmd: "medagent_start" | "medagent_cancel" | "medagent_complete", label: string) {
    if (!cur) return;
    const r = await run(cmd, cmd, { runId: cur.id, revision: cur.revision });
    if (outcome(r, label)) await after();
  }
  async function execute() {
    if (!cur) return;
    const r = await run<{ turn: unknown; proposal: unknown }>("execute", "medagent_execute", { runId: cur.id });
    if (outcome(r, "Run executed · proposal recorded")) { setRefusal(null); setProposal(r.data.proposal); await after(); }
    else if (!r.ok) { setRefusal({ runId: cur.id, error: r.error }); await after(); }
  }
  async function createAgent(e: FormEvent) {
    e.preventDefault();
    if (!newAgent) return;
    const r = await run<Agent>("agent", "medagent_create_agent", { projectId, ...newAgent });
    if (outcome(r, "Agent registered")) { setNewAgent(null); setAgentId(r.data.id); await runs.reload(); }
  }
  async function revoke(a: Agent) {
    const r = await run("revoke", "medagent_revoke_agent", { agentId: a.id, revision: a.revision ?? 1 });
    setConfirmRevoke(null);
    if (outcome(r, "Agent revoked · history kept")) { if (agentId === a.id) setAgentId(""); await runs.reload(); await detail.reload(); }
  }
  async function admitPack(e: FormEvent) {
    e.preventDefault();
    const r = await run("pack", "medagent_admit_pack", { path: packPath });
    if (outcome(r, "Pack admitted by Core")) { setPackPath(""); await models.reload(); }
  }
  const toggle = (xs: string[], x: string) => (xs.includes(x) ? xs.filter((y) => y !== x) : [...xs, x]);
  const hasExecutedTurns = turns.state.status === "ready" && turns.state.data.length > 0;

  return (
    <div className="agent-layout">
      <div>
        <Section title="Runs" count={list.length} actions={<button type="button" id="new-run" className="btn btn-g" onClick={() => setComposing(true)}>New run</button>}>
          {runs.state.status === "loading" && <Skeleton />}
          {runs.state.status === "error" && <ErrorState error={runs.state.error} onRetry={runs.reload} />}
          {list.length === 0 && runs.state.status === "ready" && <p className="t-sm i3">No runs yet.</p>}
          {list.map((r) => (
            <button type="button" key={r.id} className={`lr clickable ${sel === r.id ? "sel" : ""}`} style={{ gridTemplateColumns: "minmax(0,1fr) auto", width: "100%", textAlign: "left" }} onClick={() => { setSel(r.id); setProposal(null); setRefusal(null); }}>
              <span style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="trunc">{r.prompt}</span><span className="mono t-mono-sm i3 trunc">{r.id}</span></span>
              <Status kind={glyphFor(r.status)} weight="weak">{r.status}</Status>
            </button>
          ))}
        </Section>
      </div>
      <div>
        {(composing || !cur) && <form onSubmit={create} className="stack" style={{ gap: 8, marginBottom: 18 }}>
          <h2 className="sec-t">New run</h2>
          <label className="lbl" htmlFor="ag">Agent</label>
          <select id="ag" value={agentId} onChange={(e) => setAgentId(e.target.value)}>
            {!active.length && <option value="">Synthetic review agent (created on first run)</option>}
            {active.map((a) => <option key={a.id} value={a.id}>{a.label}</option>)}
          </select>
          <div className="row-gap" style={{ justifyContent: "space-between", gap: 8 }}>
            <span className="lbl">Sources this run may read ({picked.length} of {srcs.length} selected)</span>
            {srcs.length > 0 && (
              <span className="row-gap" style={{ gap: 4 }}>
                <button type="button" className="btn btn-g" onClick={() => setPicked(srcs.slice(0, 16).map((s) => s.id))}>Select all</button>
                <button type="button" className="btn btn-g" onClick={() => setPicked([])}>Clear</button>
              </span>
            )}
          </div>
          <div className="stack" style={{ gap: 2, maxHeight: 150, overflow: "auto", padding: "6px 8px", border: "1px solid var(--line-subtle)", borderRadius: "var(--r-1)" }}>
            {srcs.map((s) => (
              <label key={s.id} className="row-gap t-sm" style={{ gap: 6, cursor: "pointer" }}>
                <input type="checkbox" checked={picked.includes(s.id)} onChange={() => setPicked(toggle(picked, s.id))} />
                <span className="trunc">{s.resource_type} · {s.fixture} · {s.subject}</span>
              </label>
            ))}
          </div>
          <label className="lbl" htmlFor="ap">Prompt</label>
          <textarea id="ap" value={prompt} onChange={(e) => setPrompt(e.target.value)} onKeyDown={(e) => { if ((e.ctrlKey || e.metaKey) && e.key === "Enter") { e.preventDefault(); void createRun(); } }} placeholder={`Ask about the bound sources… (${MOD}+Enter to create)`} maxLength={2000} style={{ minHeight: 74 }} />
          <div className="row-gap"><button type="submit" className="btn btn-p btn-inline" disabled={!prompt.trim() || !picked.length || picked.length > 16 || !!pending}>{pending === "create" ? <Glyph kind="running" /> : null}Create run</button>{composing && cur && <button type="button" className="btn btn-g" onClick={() => { setComposing(false); document.getElementById("new-run")?.focus(); }}>Cancel</button>}</div>
        </form>}
        {cur ? <>
          <LifecycleStepper steps={agentLifecycleSteps(cur.status, hasExecutedTurns)} />
          <Ledger items={[
            { k: "run", v: cur.id, mono: true },
            { k: "status", v: <Status kind={glyphFor(cur.status)} weight="strong">{cur.status}</Status> },
            { k: "revision", v: cur.revision, mono: true },
            ...(d ? [{ k: "agent", v: `${d.agent.label} (${d.agent.status})` }, { k: "model", v: `${d.agent.pack_id} · ${d.agent.pack_version}`, mono: true }, { k: "tools", v: d.agent.tools.join(", ") || "none" }] : []),
          ]} />
          <div className="row-gap" style={{ gap: 8, marginBottom: 16, flexWrap: "wrap" }}>
            <button type="button" className="btn btn-s" onClick={() => step("medagent_start", "Run started")} disabled={!!pending || cur.status !== "pending"}>Start</button>
            <button type="button" className="btn btn-p btn-inline" onClick={execute} disabled={!!pending || cur.status !== "running"}>{pending === "execute" ? <Glyph kind="running" /> : null}Execute on local runtime</button>
            <button type="button" className="btn btn-s" onClick={() => step("medagent_complete", "Run completed · receipt recorded")} disabled={!!pending || cur.status !== "running"}>Complete</button>
            <button type="button" className="btn btn-g" onClick={() => step("medagent_cancel", "Run cancelled")} disabled={!!pending || ["completed", "cancelled", "failed"].includes(cur.status)}>Cancel</button>
          </div>
          {refusal?.runId === cur.id && <ErrorState error={refusal.error} />}
          {d && d.evidence.length > 0 && d.evidence.every((e) => e.resolution !== "current") && cur.status !== "completed" && cur.status !== "cancelled" && cur.status !== "failed" && refusal?.runId !== cur.id &&
            <Notice kind="blocked" title="Execute will be refused.">None of this run&apos;s sources is still current, so Core won&apos;t run the model. Create a new run with current sources.</Notice>}
          {proposal !== null && <Section title="Proposal" count="not accepted">
            <div style={{ border: "1px dashed var(--line-strong)", borderRadius: "var(--r-2)", padding: "12px 14px" }}>
              <div className="row-gap" style={{ justifyContent: "space-between", marginBottom: 8 }}>
                <Status kind="proposal" weight="strong">Proposal</Status>
                <CopyButton text={JSON.stringify(proposal, null, 2)} label="Copy JSON" />
              </div>
              <Disclosure summary="Proposal payload" defaultOpen>
                <pre className="pre" style={{ border: 0, background: "transparent", padding: 0 }}>{JSON.stringify(proposal, null, 2)}</pre>
              </Disclosure>
            </div>
          </Section>}
          <Section title="Evidence" count={d ? d.evidence.length : "…"}>
            {detail.state.status === "error" && <ErrorState error={detail.state.error} onRetry={detail.reload} />}
            {d?.evidence.map((e) => (
              <div key={e.id} className="lr" style={{ gridTemplateColumns: "minmax(0,1fr) auto", padding: "6px 12px" }}>
                <span style={{ display: "flex", flexDirection: "column", minWidth: 0 }}><span className="t-sm trunc">{e.resource_type ?? "source"} · {e.fixture ?? e.id}</span><span className="mono t-mono-sm i3 trunc">{e.id}{e.subject ? ` · ${e.subject}` : ""}</span></span>
                <Status kind={e.resolution === "current" ? "available" : "blocked"} weight="weak">{e.resolution ?? "unknown"}</Status>
              </div>
            ))}
          </Section>
          <Section title="Turns" count={turns.state.status === "ready" ? turns.state.data.length : "…"}>
            <LoadError cmd={turns} />
            {turns.state.status === "ready" && (turns.state.data.length ? turns.state.data.map((t) => (
              <div key={t.seq} className="lr" style={{ gridTemplateColumns: "28px 116px minmax(0,1fr)", alignItems: "start", padding: "8px 12px" }}><span className="mono t-mono-sm i3">{t.seq}</span><span className="t-sm i2">{t.kind}</span><TurnPayload payload={t.payload} /></div>
            )) : <p className="t-sm i3">No turns yet. Start, then execute the run.</p>)}
          </Section>
          <Authority>Proposal, not accepted. Nothing is written to a record. Runs, turns and evidence persist in the vault and resume after a restart.</Authority>
        </> : <Empty icon="medagent" title="No run selected">Choose an agent and the sources it may read, then create a run.</Empty>}
      </div>
      <aside className="agent-side" aria-label="Agents and models">
        <Section title="Agents" count={agents.length}>
          {agents.map((a) => (
            <div key={a.id} className="row-gap" style={{ justifyContent: "space-between", minHeight: 30, gap: 8 }}>
              <span className="t-sm trunc">{a.label}</span>
              <span className="row-gap" style={{ gap: 6 }}>
                <Status kind={glyphFor(String(a.status))} weight="weak">{String(a.status)}</Status>
                {a.status === "active" && <button type="button" className="btn btn-g" onClick={() => setConfirmRevoke(a)} disabled={!!pending}>Revoke</button>}
              </span>
            </div>
          ))}
          {confirmRevoke && (
            <ConfirmPanel
              title={`Revoke ${confirmRevoke.label}?`}
              confirmLabel="Revoke agent"
              destructive
              busy={pending === "revoke"}
              onConfirm={() => void revoke(confirmRevoke)}
              onCancel={() => setConfirmRevoke(null)}
            >
              Prevents new runs under this agent. Past runs, turns, and audit history remain preserved in the vault.
            </ConfirmPanel>
          )}
          {!agents.length && <p className="t-sm i3">No agents yet. Register one, or the first run registers a synthetic review agent on the fixture Pack.</p>}
          {newAgent ? (
            <form onSubmit={createAgent} className="stack" style={{ gap: 8, marginTop: 10 }}>
              <label className="lbl" htmlFor="an">Agent name</label>
              <input id="an" value={newAgent.name} maxLength={80} onChange={(e) => setNewAgent({ ...newAgent, name: e.target.value })} />
              <label className="lbl" htmlFor="am">Model (admitted Pack)</label>
              <select id="am" value={newAgent.packId} onChange={(e) => setNewAgent({ ...newAgent, packId: e.target.value })}>
                {packs.map((p) => <option key={p.id} value={p.id} disabled={!p.runnable}>{p.id} · {p.version}{p.fixture ? " · fixture" : ""}{p.runnable ? "" : " · folder unknown"}</option>)}
              </select>
              <span className="lbl">Tools</span>
              {TOOLS.map(([k, label]) => (
                <label key={k} className="row-gap t-sm" style={{ gap: 6 }}>
                  <input type="checkbox" checked={newAgent.tools.includes(k)} onChange={() => setNewAgent({ ...newAgent, tools: toggle(newAgent.tools, k) })} />{label}
                </label>
              ))}
              <div className="row-gap" style={{ gap: 8 }}>
                <button type="submit" className="btn btn-p btn-inline" disabled={!newAgent.name.trim() || !newAgent.packId || !newAgent.tools.length || !!pending}>Register</button>
                <button type="button" className="btn btn-g" onClick={() => setNewAgent(null)}>Cancel</button>
              </div>
            </form>
          ) : (
            <button type="button" className="btn btn-s" style={{ marginTop: 10 }} disabled={!packs.length} onClick={() => setNewAgent({ name: "", packId: packs.find((p) => p.runnable)?.id ?? "", tools: ["read_context_artifact"] })}>New agent</button>
          )}
        </Section>
        <Section title="Models" count={packs.length}>
          {models.state.status === "error" && <ErrorState error={models.state.error} onRetry={models.reload} />}
          {packs.map((p) => <div key={p.id} className="row-gap" style={{ justifyContent: "space-between", minHeight: 28 }}><span className="mono t-mono-sm trunc">{p.id}</span><Status kind={p.runnable ? "available" : "blocked"} weight="weak">{p.fixture ? "fixture" : p.runnable ? "local" : "folder unknown"}</Status></div>)}
          {externalPacks && <form onSubmit={admitPack} className="stack" style={{ gap: 6, marginTop: 8 }}>
            <label className="lbl" htmlFor="pp">Admit a local Pack folder</label>
            <input id="pp" value={packPath} maxLength={1024} placeholder="Absolute folder path" onChange={(e) => setPackPath(e.target.value)} />
            <div><button type="submit" className="btn btn-s" disabled={!packPath.trim() || !!pending}>Admit</button></div>
          </form>}
          <p className="t-sm i3" style={{ marginTop: 6 }}>Core verifies the manifest, digests and signature. Nothing is downloaded.</p>
        </Section>
      </aside>
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

  const agentsQ = useCommand<{ runs: Run[]; agents: Agent[] }>("medagent_runs", { projectId });
  const sourcesQ = useCommand<Source[]>("medagent_sources");
  const fleetAgents = agentsQ.state.status === "ready" ? agentsQ.state.data.agents.filter((a) => a.status === "active") : [];
  const fleetSources = sourcesQ.state.status === "ready" ? sourcesQ.state.data : [];
  const [lane, setLane] = useState<{ role: string; agentId: string; sourceId: string } | null>(null);
  async function addLane(e: FormEvent) {
    e.preventDefault();
    if (!lane) return;
    const r = await run("lane", "fleet_create_lane", { projectId, role: lane.role, agentId: lane.agentId, sourceIds: [lane.sourceId] });
    if (outcome(r, "Lane created")) { setLane(null); await o.reload(); }
  }
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

  const laneRoleById = new Map(v!.lanes.map((l) => [l.id, l.role_label]));
  const reportObs = detail.state.status === "ready" ? detail.state.data.report?.observations ?? [] : [];
  const obsKindCounts = new Map<string, number>();
  for (const obs of reportObs) obsKindCounts.set(obs.kind, (obsKindCounts.get(obs.kind) ?? 0) + 1);
  const obsSegments: SegmentItem[] = Array.from(obsKindCounts.entries()).map(([k, count]) => ({
    key: k,
    label: k,
    count,
    tone: /agree|match/i.test(k) ? "present" : /disagree|conflict|differ/i.test(k) ? "attention" : /fail|error|excl/i.test(k) ? "danger" : "neutral",
  }));

  const activeLaneRuns = detail.state.status === "ready" ? detail.state.data.lanes : [];
  const allLanesDone = activeLaneRuns.length > 0 && activeLaneRuns.every((l) => /completed|done|failed|cancelled/i.test(l.run_status));

  return (
    <div className="wb-side">
      <div>
        <Section title="Lanes" count={v!.lanes.length} actions={<button type="button" className="btn btn-g" onClick={setup} disabled={!!pending}>Add two lanes</button>}>
          {lane ? (
            <form onSubmit={addLane} className="stack" style={{ gap: 6, marginBottom: 10 }}>
              <label className="lbl" htmlFor="lr">Role</label>
              <input id="lr" value={lane.role} maxLength={80} onChange={(e) => setLane({ ...lane, role: e.target.value })} />
              <label className="lbl" htmlFor="la">Agent (its Pack is the lane&apos;s model)</label>
              <select id="la" value={lane.agentId} onChange={(e) => setLane({ ...lane, agentId: e.target.value })}>{fleetAgents.map((a) => <option key={a.id} value={a.id}>{a.label}</option>)}</select>
              <label className="lbl" htmlFor="ls">Source</label>
              <select id="ls" value={lane.sourceId} onChange={(e) => setLane({ ...lane, sourceId: e.target.value })}>{fleetSources.map((s) => <option key={s.id} value={s.id}>{s.resource_type} · {s.fixture}</option>)}</select>
              <div className="row-gap" style={{ gap: 8 }}><button type="submit" className="btn btn-p btn-inline" disabled={!lane.role.trim() || !lane.agentId || !lane.sourceId || !!pending}>Create lane</button><button type="button" className="btn btn-g" onClick={() => setLane(null)}>Cancel</button></div>
            </form>
          ) : (
            <button type="button" className="btn btn-s" style={{ marginBottom: 10 }} disabled={!fleetAgents.length || !fleetSources.length} title={fleetAgents.length ? undefined : "Register an agent in MedAgent first"} onClick={() => setLane({ role: "", agentId: fleetAgents[0]?.id ?? "", sourceId: fleetSources[0]?.id ?? "" })}>New lane</button>
          )}
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
          <button type="submit" className="btn btn-p btn-inline" disabled={!prompt.trim() || !!pending}>Create fleet run</button>
        </form>
        {cur ? <>
          <Ledger items={[{ k: "fleet", v: cur.id, mono: true }, { k: "status", v: <Status kind={glyphFor(cur.status)} weight="strong">{cur.status}</Status> }, { k: "revision", v: cur.revision, mono: true }]} />
          <div className="row-gap" style={{ gap: 8, marginBottom: 16, flexWrap: "wrap" }}>
            <button type="button" className="btn btn-s" onClick={dispatch} disabled={!!pending || cur.status !== "pending" || !v!.lanes.length}>Dispatch to active lanes</button>
            <button type="button" className="btn btn-s" onClick={compare} disabled={!!pending}>Compare lanes</button>
            <button type="button" className="btn btn-g" onClick={cancel} disabled={!!pending || ["completed", "cancelled", "failed"].includes(cur.status)}>Cancel</button>
            {!allLanesDone && cur.status !== "completed" && cur.status !== "cancelled" && (
              <span className="t-sm i3">Dispatch and execute lanes before comparing.</span>
            )}
          </div>
          {detail.state.status === "error" && <ErrorState error={detail.state.error} onRetry={detail.reload} />}
          {detail.state.status === "ready" && <>
            <Section title="Lane runs" count={detail.state.data.lanes.length}>
              {detail.state.data.lanes.length ? detail.state.data.lanes.map((l) => (
                <div key={l.lane_id} className="lr" style={{ gridTemplateColumns: "minmax(0,1.2fr) minmax(0,1fr) 115px auto" }}>
                  <span style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
                    <span className="t-sm trunc" style={{ fontWeight: 500 }}>{laneRoleById.get(l.lane_id) ?? l.lane_id}</span>
                    <span className="mono t-mono-sm i3 trunc">{l.lane_id}</span>
                  </span>
                  <span className="mono t-mono-sm i3 trunc">{l.run_id}</span>
                  <Status kind={glyphFor(l.run_status)}>{l.run_status}</Status>
                  <button type="button" className="btn btn-g" disabled={!!pending || l.run_status !== "running"} onClick={() => execLane(l.lane_id)}>Execute</button>
                </div>
              )) : <p className="t-sm i3">Not dispatched yet.</p>}
            </Section>
            <Section title="Comparison" count={detail.state.data.report ? detail.state.data.report.id : "none"}>
              {detail.state.data.report ? <>
                {obsSegments.length > 0 && <div style={{ marginBottom: 12 }}><SegmentedBar title="Comparison observations" meta={`${reportObs.length} total`} items={obsSegments} /></div>}
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
  // Core records a refused tool call as {status: "refused", reason}.
  if (v.status === "refused") return <span className="row-gap" style={{ gap: 8, alignItems: "baseline" }}><Status kind="blocked" weight="strong">Refused</Status><span className="t-sm i2 wrap-any">{typeof v.reason === "string" ? v.reason : "No reason recorded"}</span></span>;
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
