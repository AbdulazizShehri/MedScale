import Link from "next/link";
import type { Metadata } from "next";
import { productStage } from "@/lib/release";
import { EvidenceExample } from "@/components/EvidenceExample";
import { ArrowRight, GitHub } from "@/components/Icons";
import { Shot } from "@/components/Shot";
import { asset, github, pageMeta, type ShotKey } from "@/lib/site";

const homeMeta = pageMeta("/", "Your health AI. One open workspace.", "Open-source, local-first workspace for health records, evidence and AI models. " + productStage + ": synthetic workspaces and user-approved licensed nonpatient audio.");
const homeTitle = "MedScale — Your health AI. One open workspace.";
export const metadata: Metadata = { ...homeMeta, title: { absolute: homeTitle }, openGraph: { ...homeMeta.openGraph, title: homeTitle }, twitter: { ...homeMeta.twitter, title: homeTitle } };

/** The product, shown as the work itself: large real screenshots, one line each. */
const work: { shot: ShotKey; title: string; line: string; size: "wide" | "tall" | "half" }[] = [
  { shot: "insights", title: "Insights", line: "Answers that cite the record, or say nothing supports them.", size: "wide" },
  { shot: "patients", title: "Records", line: "What each record covers, before you read it.", size: "half" },
  { shot: "medagent", title: "Agents", line: "Local agents whose output is a proposal with a receipt.", size: "half" },
  { shot: "models", title: "Models", line: "Finding a model is not trusting it. Each step is visible.", size: "wide" },
];

const principles = [
  ["On your computer.", "No account, no cloud. Network access is denied until you allow it."],
  ["AI proposes. You decide.", "Summaries and agent runs are proposals until a person accepts them."],
  ["Every disclosure on record.", "Exports are written to an audit trail the Core keeps."],
];

export default function Home() {
  return (
    <main id="main">
      <section className="home-hero" aria-labelledby="hero-title">
        <div className="shell">
          <h1 id="hero-title" className="display home-display">Your health AI.<br /><span className="mute">One open workspace.</span></h1>
          <div className="home-hero-foot">
            <p className="lede">Records, cited evidence, local models and agents in one free, open-source desktop app.</p>
            <div className="actions">
              <Link className="btn btn-primary" href="/download/">Get MedScale <ArrowRight /></Link>
              <a className="btn btn-secondary" href={github}><GitHub size={16} /> GitHub</a>
            </div>
          </div>
          <div className="home-stage">
            <Shot name="insightsHero" priority />
          </div>
        </div>
      </section>

      <section className="home-section" aria-labelledby="work-title">
        <div className="shell">
          <h2 id="work-title" className="h2 home-h2">One window for the whole workflow.</h2>
          <div className="work-grid">
            {work.map((w) => (
              <Link key={w.shot} href="/product/" className={`work-tile work-${w.size}`}>
                <Shot name={w.shot} caption={false} />
                <span className="work-title">{w.title}</span>
                <span className="work-line">{w.line}</span>
              </Link>
            ))}
          </div>
          <Link className="text-link" href="/product/"><span>See every area</span> <ArrowRight size={14} /></Link>
        </div>
      </section>

      <section className="home-section" aria-labelledby="evidence-title">
        <div className="shell">
          <h2 id="evidence-title" className="h2 home-h2">Most AI gives you an answer.<br /><span className="mute">MedScale shows the source behind it.</span></h2>
          <EvidenceExample />
        </div>
      </section>

      <section className="home-section home-section-last" aria-labelledby="principles-title">
        <div className="shell">
          <h2 id="principles-title" className="sr-only">How MedScale works</h2>
          <div className="principles">
            {principles.map(([t, d]) => <div key={t}><h3 className="h2 h2-sm">{t}</h3><p className="body">{d}</p></div>)}
          </div>
        </div>
      </section>

      <section className="closing" aria-labelledby="closing-title">
        <div className="shell"><div className="closing-inner">
          <img src={asset("/brand/mark-black.svg")} alt="" width={61} height={40} />
          <h2 id="closing-title" className="display display-sub">Every insight,<br /><span className="mute">with its source.</span></h2>
          <div className="actions">
            <Link className="btn btn-primary" href="/download/">Get MedScale <ArrowRight /></Link>
            <a className="btn btn-outline-dark" href={github}><GitHub size={16} /> Star on GitHub</a>
          </div>
        </div></div>
      </section>
    </main>
  );
}
