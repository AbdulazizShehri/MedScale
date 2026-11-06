import Link from "next/link";
import type { Metadata } from "next";
import { DownloadCards } from "@/components/DownloadCards";
import { productStage, publicRelease } from "@/lib/release";
import { EvidenceExample } from "@/components/EvidenceExample";
import { ArrowRight, ArrowUpRight, GitHub, Monitor, Plus, Terminal } from "@/components/Icons";
import { ProductTour } from "@/components/ProductTour";
import { Quickstart } from "@/components/Quickstart";
import { Shot } from "@/components/Shot";
import { count, PROOF } from "@/lib/proof";
import { asset, docs, github, pageMeta } from "@/lib/site";

const homeMeta = pageMeta("/", "Your health AI. One open workspace.", "Open-source, local-first workspace for health records, evidence and AI models. " + productStage + ": synthetic workspaces and user-approved licensed nonpatient audio.");
const homeTitle = "MedScale — Your health AI. One open workspace.";
export const metadata: Metadata = { ...homeMeta, title: { absolute: homeTitle }, openGraph: { ...homeMeta.openGraph, title: homeTitle }, twitter: { ...homeMeta.twitter, title: homeTitle } };

const proofSmall = [
  [String(PROOF.catalogLanguages), "Languages", "Tagged across that catalog."],
  [String(PROOF.catalogFamilies), "Model families", "Including entity recognition and PII detection."],
  [`${count(PROOF.rustTestsFloor)}+`, "Automated Rust tests", "They run in public CI, and you can run them too."],
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

const story = [
  ["Open a record", "See what each subject's record covers, and what it doesn't, before you read further."],
  ["Check the evidence", "Conflicts and gaps stay visible. When two sources disagree, MedScale shows both instead of picking one."],
  ["Ask the evidence", "Answers quote the documents in your workspace with citations, or say that nothing supports them."],
  ["Run a local model", "Agents run on model packs you admit, on your own machine. Their output is a proposal with a receipt, and a person decides what happens next."],
  ["Share with disclosure", "An export reports what the format could not carry, and the Core records it in an audit trail you can read. There is no way to edit that trail in the app, though restoring an older backup can shorten it."],
];

const audiences = [
  ["Clinicians and informaticists", "Try the evidence workflow on synthetic records, and tell us what's missing. MedScale isn't for patient care today."],
  ["Researchers", "Organize projects, query pinned snapshots and keep provenance attached to every result."],
  ["Developers", "Read the Rust Core, run the tests and build on the desktop app or the CLI."],
];

const faq = [
  ["Can I use MedScale with real patient records?", `No. MedScale is an open-source ${productStage.toLowerCase()}. Synthetic workspace fixtures and user-approved licensed nonpatient audio are supported for local testing and transcription. MedScale is not clinically validated; real patient records, clinical decisions and production deployments are not authorized until external qualification gates are met.`],
  ["Is there an installer I can download?", publicRelease ? "Yes. The Download page offers each qualified installer with its checksum and signing status. You don't need an account, an app store or a GitHub login." : "Not a public one yet. CI builds unsigned engineering-preview packages for Windows and macOS to test the release process, but they are not qualified for public use. You can build the desktop app from source on Windows and macOS today, and the CLI on Windows, macOS and Linux."],
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
            <a className="status-pill" href={docs.qualification}><b>{publicRelease ? "Beta" : "Preview"}</b><span>Open source · local first</span><ArrowRight size={14} /></a>
            <h1 id="hero-title" className="display">Your health AI.<br /><span className="mute">One open workspace.</span></h1>
            <p className="lede">Free to use. Built to run on your computer. Records, cited evidence, local AI models and agents in one open-source desktop app, with a person in charge of every decision.</p>
            <div className="actions">
              <Link className="btn btn-primary" href="/download/">Get MedScale <ArrowRight /></Link>
              <a className="btn btn-secondary" href={github}><GitHub size={16} /> View on GitHub</a>
            </div>
            <div className="hero-meta">
              <span><Monitor /> Desktop for Windows and macOS</span>
              <span><Terminal /> CLI for Windows, macOS and Linux</span>
            </div>
            <p className="hero-note">{productStage} · synthetic data and licensed nonpatient audio · not for clinical use</p>
          </div>
        </div>
        <section className="proof" aria-labelledby="proof-title">
          <div className="shell">
            <h2 id="proof-title" className="eyebrow">By the numbers</h2>
            <div className="proof-lead">
              <div className="proof-item proof-accent"><strong>$0</strong><span>To use</span><small>Free, with Apache-2.0 source code.</small></div>
              <div className="proof-item"><strong>0</strong><span>Cloud accounts</span><small>No sign-in. The Core runs on your computer.</small></div>
              <Link className="proof-item" href="/models/">
                <strong>{count(PROOF.catalogModels)}</strong>
                <span>Models to browse</span>
                <small>From OpenMed&apos;s v3.0.0 public catalog, pinned by checksum. A count says nothing about quality: listed doesn&apos;t mean installed or qualified.</small>
              </Link>
            </div>
            <div className="proof-row">
              {proofSmall.map(([n, t, d]) => <div className="proof-item" key={t}><strong>{n}</strong><span>{t}</span><small>{d}</small></div>)}
            </div>
            <p className="proof-note">You can check each number against the public repository. They count the catalog and the code, not users or clinical outcomes, because MedScale doesn&apos;t have that data yet.</p>
          </div>
        </section>
        <div className="shell">
          <div className="hero-stage reveal reveal-4">
            <Shot name="insightsHero" priority />
          </div>
        </div>
      </section>

      <section className="section" aria-labelledby="story-title">
        <div className="shell">
          <div className="section-head">
            <span className="eyebrow">How it works</span>
            <h2 id="story-title" className="h2"><span className="mute">Good decisions depend on knowing where a fact came from.</span> MedScale keeps that in view.</h2>
          </div>
          <ol className="story">
            {story.map(([t, d], i) => <li key={t}><span className="story-n">{String(i + 1).padStart(2, "0")}</span><h3 className="h3">{t}</h3><p className="body">{d}</p></li>)}
          </ol>
          <div className="section-sub">
            <h3 className="eyebrow">Who it&apos;s for today</h3>
            <div className="cards">
              {audiences.map(([t, d]) => <div className="card card-plain" key={t}><strong>{t}</strong><p>{d}</p></div>)}
            </div>
          </div>
        </div>
      </section>

      <section className="section" aria-labelledby="tour-title">
        <div className="shell">
          <div className="section-head split">
            <div className="stack">
              <span className="eyebrow">One workspace</span>
              <h2 id="tour-title" className="h2"><span className="mute">Stop stitching tools together.</span> One window for all of it.</h2>
            </div>
            <p className="body">Every area runs on the same Rust Core, so the subject you open, the evidence you check and the model you run all follow the same rules for privacy, provenance and authority.</p>
          </div>
          <ProductTour />
        </div>
      </section>

      <section className="section" aria-labelledby="evidence-title">
        <div className="shell">
          <div className="section-head split">
            <div className="stack">
              <span className="eyebrow">Evidence first</span>
              <h2 id="evidence-title" className="h2"><span className="mute">Most AI gives you an answer.</span> MedScale shows the source behind it.</h2>
            </div>
            <p className="body">Generated text is a proposal until it is checked. Each statement is either supported by a cited record, in conflict with another one, or without evidence, and MedScale keeps that difference visible.</p>
          </div>
          <EvidenceExample />
        </div>
      </section>

      <section className="section band" aria-labelledby="control-title">
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
              <h3 className="h3">Disclosures leave a trail</h3>
              <p className="body">Disclosures and exports are recorded by the Core in an audit trail you can read. There is no way to edit it in the app, though restoring an older backup can shorten it. A FHIR export reports what the format could not carry.</p>
            </article>
          </div>
        </div>
      </section>

      <section className="section" aria-labelledby="models-title">
        <div className="shell">
          <div className="section-head split">
            <div className="stack">
              <span className="eyebrow">Models</span>
              <h2 id="models-title" className="h2"><span className="mute">A model is more than a name.</span> Know what it is before you use it.</h2>
            </div>
            <div className="stack">
              <p className="body">MedScale separates finding a model from trusting it. Each step is a visible state, and the last one is never granted by the software alone.</p>
              <Link className="text-link" href="/models/"><span>How models work in MedScale</span> <ArrowRight size={14} /></Link>
            </div>
          </div>
          <div className="states">
            <div className="state"><small>01</small><strong>Listed</strong><p>In the catalog, with source, task, format and expected size. Not installed.</p></div>
            <div className="state"><small>02</small><strong>Admitted</strong><p>A local pack whose manifest and digests the Core has verified.</p></div>
            <div className="state"><small>03</small><strong>Running</strong><p>Executed by the local runtime on your device, with a receipt per run.</p></div>
            <div className="state gate"><small>04</small><strong>Qualified</strong><p>Needs independent evaluation for a specific task. No model has this today.</p></div>
          </div>
        </div>
      </section>

      <section className="section band" aria-labelledby="compare-title">
        <div className="shell">
          <div className="section-head">
            <span className="eyebrow">How it compares</span>
            <h2 id="compare-title" className="h2"><span className="mute">Different tools, different trade-offs.</span> MedScale puts a whole workspace on your computer.</h2>
          </div>
          <div className="stack-lg">
            <div className="table-wrap" role="region" aria-label="How MedScale compares" tabIndex={0}>
              <table className="table-cols-4 compare">
                <thead><tr><th scope="col"><span className="sr-only">Question</span></th><th scope="col">MedScale</th><th scope="col">Cloud clinical AI</th><th scope="col">Open model libraries</th></tr></thead>
                <tbody>{compare.map(([q, a, b, c]) => <tr key={q}><th scope="row">{q}</th><td>{a}</td><td>{b}</td><td>{c}</td></tr>)}</tbody>
              </table>
            </div>
            <p className="proof-note">Our own general reading of each category, not a survey. Individual products differ.</p>
          </div>
        </div>
      </section>

      <section className="section" id="download" aria-labelledby="download-title">
        <div className="shell">
          <div className="download-hero">
            <span className="eyebrow">Download</span>
            <h2 id="download-title" className="h2">Get MedScale.</h2>
            <p className="lede">{publicRelease ? "Download the installer for your computer. No account, app store or GitHub login." : "The source is open today. A public installer is not released yet, so each platform shows exactly what you can run now."}</p>
          </div>
          <DownloadCards />
          <div className="download-foot"><span>{productStage} · synthetic workspaces + licensed nonpatient audio · not for clinical use</span><Link className="text-link text-link-sm" href="/download/">Full release status</Link></div>
        </div>
      </section>

      <section className="section" aria-labelledby="status-title">
        <div className="shell">
          <div className="section-head">
            <span className="eyebrow">Where it stands</span>
            <h2 id="status-title" className="h2"><span className="mute">No promises we can&apos;t show.</span> Here is today.</h2>
          </div>
          <div className="stack-lg">
            <div className="table-wrap" role="region" aria-label="Current status" tabIndex={0}>
              <table className="table-cols-3">
                <thead><tr><th scope="col">Area</th><th scope="col">Today</th><th scope="col">What it means</th></tr></thead>
                <tbody>
                  <tr><td>Stage</td><td>{productStage}</td><td>{publicRelease ? `Version ${publicRelease.version}, for testing with synthetic data.` : "No released version yet. You can build it from source."}</td></tr>
                  <tr><td>Real patient data</td><td>Not authorized</td><td>Use synthetic workspaces and licensed nonpatient audio only.</td></tr>
                  <tr><td>Clinical validation</td><td>Not done</td><td>MedScale is not clinically validated. This is an external gate that hasn&apos;t been met.</td></tr>
                  <tr><td>Platforms</td><td>Windows, macOS, Linux</td><td>Desktop for Windows and macOS. CLI on all three.</td></tr>
                  <tr><td>Installers</td><td>{publicRelease ? "Signing varies" : "Not published"}</td><td>{publicRelease ? "Each installer states its own signing status and checksum." : "CI builds packages to test the release process. They aren't qualified for public use."}</td></tr>
                </tbody>
              </table>
            </div>
            <a className="text-link" href={docs.qualification}>Read the external gates <ArrowUpRight /></a>
          </div>
        </div>
      </section>

      <section className="section" aria-labelledby="dev-title">
        <div className="shell dev">
          <div className="stack">
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
        <div className="shell section-sub">
          <h3 className="eyebrow">Get involved</h3>
          <div className="cards">
            <a className="card" href={docs.bugReport}><div className="card-top"><small>Feedback</small><ArrowUpRight /></div><strong>Report a problem</strong><p>Tell us what broke. Never include patient information.</p></a>
            <a className="card" href={docs.discussions}><div className="card-top"><small>Support</small><ArrowUpRight /></div><strong>Ask a question</strong><p>Design questions, ideas and help with building from source.</p></a>
            <a className="card" href={docs.security}><div className="card-top"><small>Security</small><ArrowUpRight /></div><strong>Report a vulnerability</strong><p>Use the private route in the security policy, not a public issue.</p></a>
          </div>
        </div>
      </section>

      <section className="section" aria-labelledby="faq-title">
        <div className="shell faq">
          <div className="stack">
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
        <div className="shell"><div className="closing-inner">
          <img src={asset("/brand/mark-black.svg")} alt="" width={61} height={40} />
          <h2 id="closing-title" className="display display-sub">Every insight,<br /><span className="mute">with its source.</span></h2>
          <p className="lede">Open the workspace, inspect the evidence and decide for yourself.</p>
          <div className="actions">
            <Link className="btn btn-primary" href="/download/">Get MedScale <ArrowRight /></Link>
            <a className="btn btn-outline-dark" href={github}><GitHub size={16} /> Star on GitHub</a>
          </div>
        </div></div>
      </section>
    </main>
  );
}
