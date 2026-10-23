import Link from "next/link";
import type { Metadata } from "next";
import { DownloadCards } from "@/components/DownloadCards";
import { DownloadCount } from "@/components/DownloadCount";
import { ModelCatalogScale } from "@/components/ModelCatalogScale";
import { EvidenceExample } from "@/components/EvidenceExample";
import { ArrowRight, ArrowUpRight, Check, GitHub, Monitor, Plus, Terminal } from "@/components/Icons";
import { ProductTour } from "@/components/ProductTour";
import { Quickstart } from "@/components/Quickstart";
import { Shot } from "@/components/Shot";
import { asset, docs, github, pageMeta } from "@/lib/site";

const homeMeta = pageMeta("/", "Your health AI. One open workspace.", "Open-source, local-first workspace for health records, evidence and AI models. Beta software in active development; synthetic-only evaluation.");
const homeTitle = "MedScale — Your health AI. One open workspace.";
export const metadata: Metadata = { ...homeMeta, title: { absolute: homeTitle }, openGraph: { ...homeMeta.openGraph, title: homeTitle }, twitter: { ...homeMeta.twitter, title: homeTitle } };

const strip = [
  ["Open source", "Apache-2.0 code, built in public."],
  ["Local first", "The Core runs on your machine."],
  ["Source-aware", "Every insight keeps its evidence."],
  ["People decide", "AI proposes. You approve."],
];

const faq = [
  ["Can I use MedScale with real patient records?", "No. MedScale Beta is still under active development. It is authorized for synthetic data only, is not clinically validated and is not a medical device. Real patient data, clinical decisions and production deployments are out of scope until the external gates are met."],
  ["Is there an installer I can download?", "Yes. Download the unsigned MedScale Beta engineering build for Windows or macOS from the Download page. It is for synthetic-only testing, not clinical use; operating systems may warn or block unsigned software. The Core and CLI can also be built from source."],
  ["Does MedScale need the cloud or an account?", "No. The Rust Core runs locally and network access is denied by default; browsing and egress are explicit, governed actions. Local workspace access is not organizational sign-in, and a full privacy qualification is still an open gate."],
  ["Which AI models does it run?", "Models run locally through MedScale's own runtime, from admitted model packs. The desktop app includes a browsable catalog of models with their source, task, format and size. A model being listed in the catalog does not mean it is qualified for any clinical task."],
  ["What does 'all your health AI in one place' mean today?", "One workspace for records, evidence, local models and agents, sharing one Core. It is the direction of the project, not a claim that every model or workflow is supported. The product page lists what exists now."],
  ["How can I contribute?", "Read the contributing guide, pick an issue or start a discussion. Every report, example and test must use synthetic data. Never include patient information or confidential material."],
];

export default function Home() {
  return (
    <main id="main">
      <section className="hero" aria-labelledby="hero-title">
        <div className="shell">
          <div className="hero-copy">
            <a className="status-pill" href={docs.qualification}><b>Beta</b><span>In active development · synthetic data only</span><ArrowRight size={14} /></a>
            <h1 id="hero-title" className="display">Your health AI.<br /><span className="mute">One open workspace.</span></h1>
            <p className="lede">MedScale brings records, evidence and local AI models into one source-aware desktop workspace. Open source, local-first, and built so a person stays in charge of every decision.</p>
            <div className="actions">
              <Link className="btn btn-primary" href="/download/">Get MedScale <ArrowRight /></Link>
              <a className="btn btn-secondary" href={github}><GitHub size={16} /> View on GitHub</a>
            </div>
            <ModelCatalogScale />
            <div className="hero-meta">
              <span><Monitor /> Desktop for Windows and macOS</span>
              <span><Terminal /> CLI for Windows, macOS and Linux</span>
            </div>
          </div>
          <div className="hero-stage reveal reveal-4">
            <Shot name="insightsHero" title="Insights" priority fade />
          </div>
        </div>
        <div className="strip">
          <div className="shell strip-grid">
            {strip.map(([t, d]) => <div className="strip-item" key={t}><strong>{t}</strong><span>{d}</span></div>)}
          </div>
        </div>
      </section>

      <section className="section" aria-labelledby="tour-title">
        <div className="shell">
          <div className="section-head split">
            <div style={{ display: "grid", gap: 20 }}>
              <span className="eyebrow">One workspace</span>
              <h2 id="tour-title" className="h2"><span className="mute">Stop stitching tools together.</span> One window for all of it.</h2>
            </div>
            <p className="body">Every area runs on the same Rust Core, so the patient you open, the evidence you check and the model you run all follow the same rules for privacy, provenance and authority.</p>
          </div>
          <ProductTour />
        </div>
      </section>

      <section className="section" aria-labelledby="evidence-title">
        <div className="shell">
          <div className="section-head split">
            <div style={{ display: "grid", gap: 20 }}>
              <span className="eyebrow">Evidence first</span>
              <h2 id="evidence-title" className="h2"><span className="mute">Most AI gives you an answer.</span> MedScale shows the source behind it.</h2>
            </div>
            <p className="body">Generated text is a proposal until it is checked. Each statement is either supported by a cited record, in conflict with another one, or without evidence, and MedScale keeps that difference visible.</p>
          </div>
          <EvidenceExample />
        </div>
      </section>

      <section className="section" aria-labelledby="control-title">
        <div className="shell">
          <div className="section-head">
            <span className="eyebrow">Local first · human oversight</span>
            <h2 id="control-title" className="h2"><span className="mute">The intelligence is powerful.</span> The boundaries are explicit.</h2>
          </div>
          <div className="pillars">
            <article className="pillar">
              <div className="pillar-art" aria-hidden="true">
                <svg width="220" height="80" viewBox="0 0 220 80" fill="none" stroke="currentColor" strokeWidth="1.4"><rect x="20" y="14" width="96" height="52" rx="10" /><text x="68" y="45" fill="var(--ink-2)" stroke="none" fontSize="12" textAnchor="middle" fontFamily="var(--font-mono)">Core</text><path d="M120 40h52" strokeDasharray="4 5" /><path d="M176 30l14 20M190 30l-14 20" stroke="var(--orange)" strokeWidth="1.8" /></svg>
              </div>
              <h3 className="h3">Local by default</h3>
              <p className="body">The Core and your workspace stay on your machine. There is no default network egress, and browsing is denied until you allow it.</p>
            </article>
            <article className="pillar">
              <div className="pillar-art" aria-hidden="true">
                <svg width="220" height="80" viewBox="0 0 220 80" fill="none" stroke="currentColor" strokeWidth="1.4"><rect x="12" y="22" width="84" height="36" rx="8" strokeDasharray="4 4" /><text x="54" y="44" fill="var(--ink-3)" stroke="none" fontSize="11" textAnchor="middle" fontFamily="var(--font-mono)">proposal</text><path d="M102 40h22m-6-6 6 6-6 6" /><rect x="130" y="22" width="78" height="36" rx="8" stroke="var(--blue)" /><text x="169" y="44" fill="var(--ink)" stroke="none" fontSize="11" textAnchor="middle" fontFamily="var(--font-mono)">you decide</text></svg>
              </div>
              <h3 className="h3">AI proposes. You decide.</h3>
              <p className="body">Summaries and agent runs are recorded as proposals. Changes that carry authority stay explicit, so nothing important happens silently.</p>
            </article>
            <article className="pillar">
              <div className="pillar-art" aria-hidden="true">
                <svg width="220" height="80" viewBox="0 0 220 80" fill="none" stroke="currentColor" strokeWidth="1.4">{[18, 36, 54].map((y, i) => <g key={y}><circle cx="28" cy={y + 4} r="3" fill={i === 0 ? "var(--blue)" : "currentColor"} stroke="none" /><path d={`M40 ${y + 4}h${110 - i * 22}`} /><path d={`M164 ${y + 4}h26`} stroke="var(--ink-3)" /></g>)}</svg>
              </div>
              <h3 className="h3">Everything leaves a trail</h3>
              <p className="body">Disclosures and exports are recorded by the Core in an audit trail you can read and cannot edit. FHIR export shows what leaves before it does.</p>
            </article>
          </div>
        </div>
      </section>

      <section className="section" aria-labelledby="models-title">
        <div className="shell">
          <div className="section-head split">
            <div style={{ display: "grid", gap: 20 }}>
              <span className="eyebrow">Models</span>
              <h2 id="models-title" className="h2"><span className="mute">A model is more than a name.</span> Know what it is before you use it.</h2>
            </div>
            <div style={{ display: "grid", gap: 20, justifyItems: "start" }}>
              <p className="body">MedScale separates finding a model from trusting it. Each step is a visible state, and the last one is never granted by the software alone.</p>
              <Link className="text-link" href="/models/">How models work in MedScale <ArrowRight size={14} /></Link>
            </div>
          </div>
          <div className="states">
            <div className="state"><small>01</small><strong>Listed</strong><p>In the catalog, with source, task, format and size. Not installed.</p></div>
            <div className="state"><small>02</small><strong>Admitted</strong><p>A local pack whose manifest and digests the Core has verified.</p></div>
            <div className="state"><small>03</small><strong>Running</strong><p>Executed by the local runtime on your device, with a receipt per run.</p></div>
            <div className="state gate"><small>04</small><strong>Qualified</strong><p>Needs independent evaluation for a specific task. No model has this today.</p></div>
          </div>
        </div>
      </section>

      <section className="section" aria-labelledby="arch-title">
        <div className="shell arch">
          <div style={{ display: "grid", gap: 22 }}>
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

      <section className="section" id="download" aria-labelledby="download-title">
        <div className="shell">
          <div className="download-hero">
            <span className="eyebrow">Download</span>
            <h2 id="download-title" className="h2">Get MedScale.</h2>
            <p className="lede">Download MedScale Beta for Windows or macOS, or explore the open-source Core and CLI. Desktop builds are unsigned and intended for synthetic-only evaluation.</p>
          </div>
          <DownloadCards />
          <DownloadCount placement="page" />
          <div className="download-foot"><span>MedScale Beta · synthetic data only</span><Link className="text-link" href="/download/" style={{ fontSize: 13 }}>Full release status</Link></div>
        </div>
      </section>

      <section className="section" aria-labelledby="dev-title">
        <div className="shell dev">
          <div style={{ display: "grid", gap: 22 }}>
            <span className="eyebrow">For developers</span>
            <h2 id="dev-title" className="h2"><span className="mute">Read the code.</span> Run it in four commands.</h2>
            <p className="body">MedScale is a Rust workspace with a desktop app and a CLI. Clone it, build it and run the health check. Contributions start with an issue, a synthetic test case or a design question.</p>
            <div className="actions">
              <a className="btn btn-secondary" href={docs.contributing}>Contributing guide <ArrowUpRight /></a>
              <Link className="btn btn-secondary" href="/docs/">Documentation <ArrowRight /></Link>
            </div>
          </div>
          <Quickstart />
        </div>
      </section>

      <section className="section" aria-labelledby="docs-title">
        <div className="shell">
          <div className="section-head">
            <span className="eyebrow">Explore</span>
            <h2 id="docs-title" className="h2">Find your way in.</h2>
          </div>
          <div className="cards">
            <Link className="card" href="/product/"><div className="card-top"><small>Product</small><ArrowRight /></div><strong>What MedScale does</strong><p>Records, insights, agents, models and governance, area by area.</p></Link>
            <Link className="card" href="/models/"><div className="card-top"><small>Models</small><ArrowRight /></div><strong>Model discovery</strong><p>Catalog, local runtime and the states a model moves through.</p></Link>
            <Link className="card" href="/docs/"><div className="card-top"><small>Docs</small><ArrowRight /></div><strong>Get started</strong><p>Setup, desktop, CLI, security and the qualification boundaries.</p></Link>
          </div>
        </div>
      </section>

      <section className="section" aria-labelledby="faq-title">
        <div className="shell faq">
          <div style={{ display: "grid", gap: 20, alignContent: "start" }}>
            <span className="eyebrow">Questions</span>
            <h2 id="faq-title" className="h2">Before you begin.</h2>
            <p className="body">Plain answers about what MedScale is today, and what it is not yet.</p>
          </div>
          <div className="faq-list">
            {faq.map(([q, a]) => <details key={q}><summary>{q}<Plus /></summary><p>{a}</p></details>)}
          </div>
        </div>
      </section>

      <section className="closing" aria-labelledby="closing-title">
        <div className="shell inner">
          <img src={asset("/brand/mark-white.svg")} alt="" width={53} height={72} />
          <h2 id="closing-title" className="display" style={{ fontSize: "clamp(40px, 6vw, 72px)" }}>Every insight,<br /><span className="mute">with its source.</span></h2>
          <p className="lede" style={{ margin: "0 auto" }}>Open the workspace, inspect the evidence and decide for yourself.</p>
          <div className="actions">
            <Link className="btn btn-primary" href="/download/">Get MedScale <ArrowRight /></Link>
            <a className="btn btn-secondary" href={github}><GitHub size={16} /> Star on GitHub</a>
          </div>
        </div>
      </section>
    </main>
  );
}
