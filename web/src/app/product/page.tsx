import Link from "next/link";
import type { Metadata } from "next";
import { ArrowRight, ArrowUpRight } from "@/components/Icons";
import { Shot } from "@/components/Shot";
import { FeatureAtlas, DESKTOP_FEATURE_COUNT } from "@/components/FeatureAtlas";
import { docs, pageMeta, type ShotKey } from "@/lib/site";

export const metadata: Metadata = pageMeta("/product/", "All MedScale features", "MedScale Beta is free to use and runs locally. Explore its source-aware desktop features, local AI models, agents, research and governance with no automatic upload.");

const rows: { eyebrow: string; title: string; mute: string; body: string; shot: ShotKey; label: string; points: string[] }[] = [
  { eyebrow: "Records", mute: "Health information", title: "with a history.", body: "Open a synthetic subject and see coverage, documents and review state together. The list tells you what each record supports before you open it.", shot: "patients", label: "Patients", points: ["Longitudinal subjects with coverage per fact", "Documents and FHIR R4 fixtures with their origin", "Review states written out in words"] },
  { eyebrow: "Insights", mute: "Answers that", title: "show their work.", body: "Insights count only present facts and list what is absent, unknown or in conflict. Ask a question and the answer cites the synthetic sentences it used, or says it has none.", shot: "insights", label: "Insights", points: ["Cohort coverage without scores or predictions", "Conflicts and gaps kept visible", "Grounded answers with citations"] },
  { eyebrow: "Knowledge", mute: "Search your sources,", title: "with receipts.", body: "Build a local index over a project's sources and search it. Every search leaves a receipt, and an empty result is reported as insufficient evidence instead of being filled in.", shot: "knowledge", label: "Knowledge", points: ["Local index with version and freshness", "Search receipts for every query", "Insufficient evidence is a real answer"] },
  { eyebrow: "Agents", mute: "Agents that", title: "leave receipts.", body: "MedAgent runs named agents on admitted local packs. You bind the sources each run may read, and the output is recorded as a proposal with its receipt.", shot: "medagent", label: "MedAgent", points: ["Context bound to chosen sources", "Runs recorded as proposals", "Local packs only; nothing is downloaded"] },
  { eyebrow: "Models", mute: "A model is", title: "more than a name.", body: "See the local runtime, which packs are admitted in this session, and a browsable catalog of models with source, task, format and size.", shot: "models", label: "Models", points: ["Runtime and admission state", "Catalog search by task and format", "Listing is never clinical approval"] },
  { eyebrow: "Governance", mute: "Every disclosure", title: "on the record.", body: "The Audit Trail shows disclosure history recorded by the Core. Exports show what will leave before it leaves, and network access is denied by default.", shot: "audit", label: "Audit Trail", points: ["Read-only audit history", "Exports with disclosure", "Default-deny browsing and egress"] },
];

export default function Product() {
  return (
    <main id="main">
      <section className="shell page-hero">
        <span className="eyebrow">Product</span>
        <h1 className="display" style={{ fontSize: "clamp(40px, 6vw, 72px)" }}>One workspace.<br /><span className="mute">Every area in context.</span></h1>
        <p className="lede">Free to use and built to run on your computer, MedScale brings records, evidence and AI models into one desktop app and CLI over a shared Rust Core. These screenshots use the synthetic Beta workspace.</p>
        <p className="product-trustline">Local processing · No cloud account · No automatic data upload</p>
        <div className="actions"><Link className="btn btn-primary" href="/download/">Get MedScale <ArrowRight /></Link><a className="btn btn-secondary" href={docs.desktopSource}>Desktop source <ArrowUpRight /></a></div>
      </section>
      <section className="page-section">
        <div className="shell">
          {rows.map((r, i) => (
            <div key={r.eyebrow} className={"feature-row" + (i % 2 ? " flip" : "")}>
              <div className="feature-copy">
                <span className="eyebrow">{r.eyebrow}</span>
                <h2 className="h2" style={{ fontSize: "clamp(28px, 3.4vw, 40px)" }}><span className="mute">{r.mute}</span> {r.title}</h2>
                <p className="body">{r.body}</p>
                <ul className="checklist">{r.points.map((p) => <li key={p}><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="m5 12.5 4.2 4.2L19 7" /></svg>{p}</li>)}</ul>
              </div>
              <Shot name={r.shot} title={r.label} priority={i === 0} />
            </div>
          ))}
        </div>
      </section>
      <section className="page-section" id="all-features">
        <div className="shell">
          <div className="section-head split">
            <div><span className="eyebrow">Feature directory</span><h2 className="h2">All {DESKTOP_FEATURE_COUNT} desktop areas.</h2></div>
            <p className="body">Explore the complete Beta workspace: records, research, models, agents, operations and governance. Synthetic scenarios only.</p>
          </div>
          <FeatureAtlas />
        </div>
      </section>
      <section className="page-section">
        <div className="shell" style={{ display: "grid", gap: 20, maxWidth: 700 }}>
          <span className="eyebrow">Also in the workspace</span>
          <p className="body" style={{ fontSize: 17 }}>Projects, data snapshots and analytics queries, a governed browser, collaboration rooms, synthetic audio import, the model fleet and FHIR exports are each covered by the desktop end-to-end journeys that run on the release binary. Workflows, tasks, messages and privacy settings are there too.</p>
          <div className="notice"><strong>Engineering preview</strong><p>Synthetic data only. MedScale is not clinically validated, is not a medical device and must not be used for patient care. See the <a className="text-link" style={{ fontSize: 14 }} href={docs.qualification}>external gates</a>.</p></div>
        </div>
      </section>
    </main>
  );
}
