import { productStage } from "@/lib/release";
import Link from "next/link";
import type { Metadata } from "next";
import { ArrowRight, ArrowUpRight, Check } from "@/components/Icons";
import { Shot } from "@/components/Shot";
import { docs, pageMeta, type ShotKey } from "@/lib/site";

export const metadata: Metadata = pageMeta("/product/", "Product", "Records, insights, models, agents and governance in one source-aware desktop workspace with a shared Rust Core.");

const rows: { eyebrow: string; title: string; mute: string; body: string; shot: ShotKey; points: string[] }[] = [
  { eyebrow: "Records", mute: "Health information", title: "with a history.", body: "Open a synthetic subject and see coverage, documents and review state together. The list tells you what each record supports before you open it.", shot: "patients", points: ["Longitudinal subjects with coverage per fact", "Documents and FHIR R4 fixtures with their origin", "Review states written out in words"] },
  { eyebrow: "Insights", mute: "Answers that", title: "show their work.", body: "Insights count only present facts and list what is absent, unknown or in conflict. Ask a question and the answer cites the synthetic sentences it used, or says it has none.", shot: "insights", points: ["Cohort coverage without scores or predictions", "Conflicts and gaps kept visible", "Grounded answers with citations"] },
  { eyebrow: "Knowledge", mute: "Search your sources,", title: "with receipts.", body: "Build a local index over a project's sources and search it. Every search leaves a receipt, and an empty result is reported as insufficient evidence instead of being filled in.", shot: "knowledge", points: ["Local index with version and freshness", "Search receipts for every query", "Insufficient evidence is a real answer"] },
  { eyebrow: "Agents", mute: "Agents that", title: "leave receipts.", body: "MedAgent runs named agents on admitted local packs. You bind the sources each run may read, and the output is recorded as a proposal with its receipt.", shot: "medagent", points: ["Context bound to chosen sources", "Runs recorded as proposals", "Local packs only; nothing is downloaded"] },
  { eyebrow: "Models", mute: "A model is", title: "more than a name.", body: "See the local runtime, which packs are admitted in this session, and a browsable catalog of models with source, task, format and size.", shot: "models", points: ["Runtime and admission state", "Catalog search by task and format", "Listing is never clinical approval"] },
  { eyebrow: "Governance", mute: "Every disclosure", title: "on the record.", body: "The Audit Trail shows disclosure history recorded by the Core. Exports show what will leave before it leaves, and network access is denied by default.", shot: "audit", points: ["Read-only audit history", "Exports with disclosure", "Default-deny browsing and egress"] },
];

export default function Product() {
  return (
    <main id="main">
      <section className="shell page-hero">
        <span className="eyebrow">Product</span>
        <h1 className="display display-sub">One workspace.<br /><span className="mute">Every area in context.</span></h1>
        <p className="lede">MedScale is a desktop app and a CLI over one Rust Core. These are real screenshots of the desktop app with the synthetic workspace that ships with the project.</p>
        <div className="actions"><Link className="btn btn-primary" href="/download/">Get MedScale <ArrowRight /></Link><a className="btn btn-secondary" href={docs.desktopSource}>Desktop source <ArrowUpRight /></a></div>
      </section>
      <section className="page-section">
        <div className="shell">
          {rows.map((r, i) => (
            <div key={r.eyebrow} className="feature-row">
              <div className="feature-copy">
                <span className="eyebrow">{r.eyebrow}</span>
                <h2 className="h2 h2-sm"><span className="mute">{r.mute}</span> {r.title}</h2>
                <p className="body">{r.body}</p>
                <ul className="checklist">{r.points.map((p) => <li key={p}><Check />{p}</li>)}</ul>
              </div>
              <Shot name={r.shot} priority={i === 0} />
            </div>
          ))}
        </div>
      </section>
      <section className="page-section">
        <div className="shell feature-row">
          <div className="feature-copy">
            <span className="eyebrow">Also in the workspace</span>
            <p className="body">Projects, data snapshots and analytics queries, a governed browser, collaboration rooms, synthetic audio import, the model fleet and FHIR exports are each covered by the desktop end-to-end journeys that run on the release binary. Workflows, tasks, messages and privacy settings are there too.</p>
          </div>
          <div className="notice"><strong>{productStage}</strong><p>Synthetic workspaces and user-approved licensed nonpatient audio are supported. Real patient data and clinical decisions are not authorized; MedScale is not clinically validated. See the <a className="text-link" href={docs.qualification}>external gates</a>.</p></div>
        </div>
      </section>
    </main>
  );
}
