import Link from "next/link";
import type { Metadata } from "next";
import { productStage } from "@/lib/release";
import { EvidenceExample } from "@/components/EvidenceExample";
import { GitHub } from "@/components/Icons";
import { Shot } from "@/components/Shot";
import { asset, github, pageMeta, type ShotKey } from "@/lib/site";

const homeMeta = pageMeta("/", "Your health AI. One open workspace.", "Open-source, local-first workspace for health records, evidence and AI models. " + productStage + ": synthetic workspaces and user-approved licensed nonpatient audio.");
const homeTitle = "MedScale — Your health AI. One open workspace.";
export const metadata: Metadata = { ...homeMeta, title: { absolute: homeTitle }, openGraph: { ...homeMeta.openGraph, title: homeTitle }, twitter: { ...homeMeta.twitter, title: homeTitle } };

/** The product shown as the work itself: large real screenshots, a title and one line each.
 *  Insights opens the page as the hero, so the grid shows the other three areas. */
const work: { shot: ShotKey; title: string; line: string }[] = [
  { shot: "patients", title: "Records", line: "What each record covers, and what it doesn't, before you read it." },
  { shot: "medagent", title: "Agents", line: "Runs bound to the sources you choose, each with a receipt." },
  { shot: "models", title: "Models", line: "Finding a model and trusting it are separate, visible steps." },
];

export default function Home() {
  return (
    <main id="main">
      <section className="home-hero" aria-labelledby="hero-title">
        <div className="shell">
          <h1 id="hero-title" className="home-display">Your health AI.<br /> One open workspace.</h1>
          <div className="home-hero-foot">
            <p className="home-lede">Records, cited evidence, local models and agents in one free, open-source desktop app.</p>
            <div className="actions">
              <Link className="btn btn-primary" href="/download/">Get MedScale</Link>
              <a className="btn btn-secondary" href={github}><GitHub size={16} /> GitHub</a>
            </div>
          </div>
        </div>
        <div className="shell home-stage">
          <Shot name="insightsHero" priority />
        </div>
      </section>

      <section className="home-section" aria-labelledby="work-title">
        <div className="shell">
          <h2 id="work-title" className="home-h2">Every part of the work, in one window.</h2>
          <div className="work-grid">
            {work.map((w) => (
              <Link key={w.shot} href="/product/" className="work-tile">
                <Shot name={w.shot} caption={false} />
                <span className="work-title">{w.title}</span>
                <span className="work-line">{w.line}</span>
              </Link>
            ))}
          </div>
        </div>
      </section>

      <section className="home-section" aria-labelledby="evidence-title">
        <div className="shell">
          <h2 id="evidence-title" className="home-h2">Every answer shows the source behind it.</h2>
          <EvidenceExample />
        </div>
      </section>

      <section className="home-section home-section-last" aria-labelledby="principle-title">
        <div className="shell home-principle">
          <h2 id="principle-title" className="home-statement">It runs on your computer, and a person decides.</h2>
          <p className="body">No account and no cloud. Network access stays off until you allow it.</p>
          <p className="body">AI output is a proposal with a receipt, and nothing is written to a record.</p>
        </div>
      </section>

      <section className="closing" aria-labelledby="closing-title">
        <div className="shell"><div className="closing-inner">
          <img src={asset("/brand/mark-black.svg")} alt="" width={61} height={40} />
          <h2 id="closing-title" className="display display-sub">Every insight, with its source.</h2>
          <div className="actions">
            <Link className="btn btn-primary" href="/download/">Get MedScale</Link>
            <a className="btn btn-outline-dark" href={github}><GitHub size={16} /> Star on GitHub</a>
          </div>
        </div></div>
      </section>
    </main>
  );
}
