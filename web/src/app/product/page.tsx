import { publicRelease } from "@/lib/release";
import { count, PROOF } from "@/lib/proof";
import Link from "next/link";
import type { Metadata } from "next";
import { ArrowRight, ArrowUpRight, Check, Plus } from "@/components/Icons";
import { Shot } from "@/components/Shot";
import { docs, pageMeta, type ShotKey } from "@/lib/site";

export const metadata: Metadata = pageMeta("/product/", "Product", "Records, insights, models, agents and governance in one source-aware desktop workspace with a shared Rust Core.");

const rows: { eyebrow: string; title: string; mute: string; body: string; shot: ShotKey; points: string[] }[] = [
  { eyebrow: "Records", mute: "Health information", title: "with a history.", body: "Open a synthetic subject and see coverage, documents and review state together. The list tells you what each record supports before you open it.", shot: "patients", points: ["Longitudinal subjects with coverage per fact", "Documents and FHIR R4 fixtures with their origin", "Review states written out in words"] },
  { eyebrow: "Insights", mute: "Answers that", title: "show their work.", body: "Insights count only present facts and list what is absent, unknown or in conflict. Ask a question and the answer cites the synthetic sentences it used, or says it has none.", shot: "insights", points: ["Cohort coverage without scores or predictions", "Conflicts and gaps kept visible", "Grounded answers with citations"] },
  { eyebrow: "Knowledge", mute: "Search your sources,", title: "with receipts.", body: "Build a local index over a project's sources and search it. Every search leaves a receipt, and an empty result is reported as insufficient evidence instead of being filled in.", shot: "knowledge", points: ["Local index with version and freshness", "Search receipts for every query", "Insufficient evidence is a real answer"] },
  { eyebrow: "Agents", mute: "Agents that", title: "leave receipts.", body: "MedAgent runs named agents on admitted local packs. You bind the sources each run may read, and the output is recorded as a proposal with its receipt.", shot: "medagent", points: ["Context bound to chosen sources", "Runs recorded as proposals", "Local packs only; nothing is downloaded"] },
  { eyebrow: "Models", mute: "A model is", title: "more than a name.", body: "See the local runtime, which packs are admitted in this session, and a browsable catalog of models with source, task, format and size.", shot: "models", points: ["Runtime and admission state", "Catalog search by task and format", "Listing is never clinical approval"] },
  { eyebrow: "Governance", mute: "Every disclosure", title: "on the record.", body: "The Audit Trail shows disclosure history recorded by the Core. Exports report what the format could not carry, and network access is denied by default.", shot: "audit", points: ["Read-only audit history", "Exports with disclosure", "Default-deny browsing and egress"] },
];


/** Moved from the home page: what can be checked against the repository. */
const numbers = [
  ["$0", "To use", "Free, with Apache-2.0 source code."],
  ["0", "Cloud accounts", "No sign-in. The Core runs on your computer."],
  [count(PROOF.catalogModels), "Models to browse", "From OpenMed's v3.0.0 public catalog, pinned by checksum. A listing isn't a downloaded or runnable model, and a count says nothing about quality."],
  [`${count(PROOF.rustTestsFloor)}+`, "Automated Rust tests", "They run in public CI, and you can run them too."],
  [String(PROOF.catalogLanguages), "Languages", "Tagged across that catalog."],
  [String(PROOF.specs), "Public specifications", "Feature designs are written down in the repository."],
];

const compare = [
  ["Where the AI runs", "On your computer", "Usually the vendor's cloud", "On your hardware"],
  ["Account or contract", "None", "Usually both", "None"],
  ["Cost to start", "Free", "Varies, often a paid plan", "Free"],
  ["Source code", "Open, Apache-2.0", "Usually closed", "Open"],
  ["Records, evidence, agents and audit trail", "One desktop app", "Varies", "Varies, often code you assemble"],
  ["Use with real patients", "Not authorized. Synthetic and licensed nonpatient data only", "Often", "Varies"],
  ["Published customer results", "None yet", "Often", "Varies"],
];

const faq = [
  ["Can I use MedScale with real patient records?", "No. Synthetic workspace fixtures and user-approved licensed nonpatient audio are supported for local testing and transcription. MedScale is not clinically validated; real patient records, clinical decisions and production deployments are not authorized until external qualification gates are met."],
  ["Is there an installer I can download?", publicRelease ? "Yes. The Download page offers each qualified installer with its checksum and signing status. You don't need an account, an app store or a GitHub login." : "Not a public one yet. CI builds and tests packages for Windows, macOS and Linux, but none is published until a release is approved. You can build the desktop app and the CLI from source today."],
  ["Does MedScale need the cloud or an account?", "No. The Rust Core runs locally and network access is denied by default; browsing and egress are explicit, governed actions."],
  ["Which AI models does it run?", "Models run locally through MedScale's own runtime, from admitted model packs. The catalog lists models with their source, task, format and size; being listed does not mean a model is qualified for any clinical task."],
  ["How can I contribute?", "Read the contributing guide, pick an issue or start a discussion. Every report, example and test must use synthetic data."],
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
      <section className="page-section" aria-labelledby="arch-title">
        <div className="shell arch">
          <div className="stack">
            <span className="eyebrow">Built on something real</span>
            <h2 id="arch-title" className="h2"><span className="mute">One trusted Core.</span> Two ways to work.</h2>
            <p className="body">The desktop app is for daily work. The CLI is for scripts and automation. Both are thin surfaces over the same Rust Core, which owns records, provenance, policy and storage.</p>
            <ul className="checklist">
              <li><Check /> Tauri 2 desktop with a React interface</li>
              <li><Check /> Encrypted local storage and FHIR R4 interchange</li>
              <li><Check /> Tests on Windows, macOS and Linux for every pull request</li>
            </ul>
            <div className="built-on" aria-label="Built on">
              <span>Built on</span>
              {["Rust", "Tauri 2", "React", "SQLCipher", "FHIR R4", "tract ONNX", "Candle"].map((t) => <b key={t}>{t}</b>)}
            </div>
            <a className="text-link" href={docs.crates}>Explore the Rust workspace <ArrowUpRight /></a>
          </div>
          <div className="arch-diagram" aria-label="Diagram: the desktop app and the CLI both use the MedScale Rust Core">
            <div className="arch-row">
              <div className="arch-node"><small>Surface</small><strong>Desktop</strong><span>Tauri 2 · React</span></div>
              <div className="arch-node"><small>Surface</small><strong>CLI</strong><span>medscale-cli</span></div>
            </div>
            <div className="arch-links" aria-hidden="true"><i /><i /></div>
            <div className="arch-node arch-core"><small>Authority</small><strong>MedScale Core</strong><span>Records · provenance · policy · audit</span></div>
            <div className="crate-list">{["medscale-core", "medscale-storage", "medscale-fhir", "medscale-pack", "medscale-network", "medscale-keys", "medscale-contracts"].map((c) => <code key={c}>{c}</code>)}</div>
          </div>
        </div>
      </section>

      <section className="page-section" aria-labelledby="numbers-title">
        <div className="shell">
          <h2 id="numbers-title" className="h2 h2-sm">By the numbers.</h2>
          <div className="product-numbers">
            {numbers.map(([n, t, d]) => <div key={t}><strong>{n}</strong><span>{t}</span><small>{d}</small></div>)}
          </div>
          <p className="body body-sm">Each number can be checked against the public repository. They count the catalog and the code, not users or clinical outcomes.</p>
        </div>
      </section>

      <section className="page-section" aria-labelledby="compare-title">
        <div className="shell stack-lg">
          <h2 id="compare-title" className="h2 h2-sm"><span className="mute">Different tools, different trade-offs.</span> MedScale puts a whole workspace on your computer.</h2>
          <div className="table-wrap" role="region" aria-label="How MedScale compares" tabIndex={0}>
            <table className="table-cols-4 compare">
              <thead><tr><th scope="col"><span className="sr-only">Question</span></th><th scope="col">MedScale</th><th scope="col">Cloud clinical AI</th><th scope="col">Open model libraries</th></tr></thead>
              <tbody>{compare.map(([q, a, b, c]) => <tr key={q}><th scope="row">{q}</th><td>{a}</td><td>{b}</td><td>{c}</td></tr>)}</tbody>
            </table>
          </div>
          <p className="body body-sm">Our own general reading of each category, not a survey. Individual products differ.</p>
        </div>
      </section>

      <section className="page-section" aria-labelledby="faq-title">
        <div className="shell faq">
          <h2 id="faq-title" className="h2 h2-sm">Questions.</h2>
          <div className="faq-list">
            {faq.map(([q, a]) => <details key={q}><summary>{q}<Plus /></summary><p>{a}</p></details>)}
          </div>
        </div>
      </section>

      <section className="page-section">
        <div className="shell feature-row">
          <div className="feature-copy">
            <span className="eyebrow">Also in the workspace</span>
            <p className="body">Projects, data snapshots and analytics queries, a governed browser, collaboration rooms, synthetic audio import, the model fleet and FHIR exports are each covered by the desktop end-to-end journeys that run on the release binary. Workflows, tasks, messages and privacy settings are there too.</p>
          </div>
          <div className="notice"><strong>Supported today</strong><p>Synthetic workspaces and user-approved licensed nonpatient audio are supported. Real patient data and clinical decisions are not authorized; MedScale is not clinically validated. See the <a className="text-link" href={docs.qualification}>external gates</a>.</p></div>
        </div>
      </section>
    </main>
  );
}
