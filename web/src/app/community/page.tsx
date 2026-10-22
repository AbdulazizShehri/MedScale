import type { Metadata } from "next";
import { ArrowUpRight, GitHub } from "@/components/Icons";
import { docs, github, pageMeta } from "@/lib/site";

export const metadata: Metadata = pageMeta("/community/", "Community", "Contribute to MedScale: issues, pull requests, discussions and the rules for synthetic data.");

export default function Community() {
  return (
    <main id="main">
      <section className="shell page-hero">
        <span className="eyebrow">Community</span>
        <h1 className="display" style={{ fontSize: "clamp(40px, 6vw, 72px)" }}>Built in the open.<br /><span className="mute">Better together.</span></h1>
        <p className="lede">MedScale is Apache-2.0 open source. Read the code, run the tests, report what breaks and help shape what comes next.</p>
        <div className="actions"><a className="btn btn-primary" href={github}><GitHub size={16} /> Open the repository</a><a className="btn btn-secondary" href={docs.contributing}>Contributing guide <ArrowUpRight /></a></div>
      </section>
      <section className="page-section">
        <div className="shell cards">
          <a className="card" href={docs.issues}><div className="card-top"><small>Issues</small><ArrowUpRight /></div><strong>Report or pick up work</strong><p>Bugs, gaps and good first changes, tracked in the open.</p></a>
          <a className="card" href={docs.discussions}><div className="card-top"><small>Discussions</small><ArrowUpRight /></div><strong>Ask and propose</strong><p>Design questions, ideas and help with building from source.</p></a>
          <a className="card" href={`${github}/pulls`}><div className="card-top"><small>Pull requests</small><ArrowUpRight /></div><strong>Change the code</strong><p>Every pull request runs the Rust, desktop and supply-chain checks.</p></a>
        </div>
      </section>
      <section className="page-section">
        <div className="shell prose">
          <h2>The one rule that never bends</h2>
          <p>Use synthetic data only. Issues, examples, tests, screenshots and logs must never contain real patient information or confidential material. If you are not sure whether something is synthetic, leave it out.</p>
          <h2>Good places to start</h2>
          <ul>
            <li>A synthetic test case that exposes a bug or an edge case</li>
            <li>Documentation that was hard to follow when you set up</li>
            <li>Accessibility and keyboard issues in the desktop app</li>
          </ul>
          <p>Security problems go through the <a className="text-link" href={docs.security}>security policy</a>, not public issues. Everyone follows the <a className="text-link" href={docs.conduct}>code of conduct</a>.</p>
        </div>
      </section>
    </main>
  );
}
