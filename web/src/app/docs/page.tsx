import type { Metadata } from "next";
import { ArrowUpRight } from "@/components/Icons";
import { docs, pageMeta } from "@/lib/site";

export const metadata: Metadata = pageMeta("/docs/", "Documentation", "Get started with MedScale: setup, the desktop app, the CLI, qualification boundaries, security and contributing.");

const groups: { id: string; title: string; items: [string, string, string][] }[] = [
  { id: "start", title: "Start", items: [["Read me first", "What MedScale is, how the repository is organized and how to build it.", docs.readme], ["Product definition", "The authoritative statement of what MedScale is for, and what it is not.", docs.product], ["Release status", "Every external gate between this preview and a production release.", docs.qualification]] },
  { id: "use", title: "Use", items: [["Desktop app", "Build and run the Tauri desktop app on Windows or macOS.", docs.desktop], ["Command line", "Every CLI command, with the same authority model as the desktop.", docs.cli], ["Rust workspace", "Core, storage, FHIR, packs, network policy and keys, crate by crate.", docs.crates]] },
  { id: "contribute", title: "Contribute", items: [["Contributing", "How to propose changes, run the tests and use synthetic data.", docs.contributing], ["Security", "How to report a vulnerability privately.", docs.security], ["Governance", "How decisions are made and who maintains the project.", docs.governance]] },
];

export default function Docs() {
  return (
    <main id="main">
      <section className="shell page-hero">
        <span className="eyebrow">Documentation</span>
        <h1 className="display" style={{ fontSize: "clamp(40px, 6vw, 72px)" }}>Everything is<br /><span className="mute">in the repository.</span></h1>
        <p className="lede">MedScale&apos;s documentation lives next to the code, so it changes with it. Start here and follow the links into GitHub.</p>
      </section>
      {groups.map((g) => (
        <section key={g.id} className="page-section" aria-labelledby={`g-${g.id}`}>
          <div className="shell" style={{ display: "grid", gap: 24 }}>
            <h2 id={`g-${g.id}`} className="h3" style={{ fontSize: 24 }}>{g.title}</h2>
            <div className="cards">{g.items.map(([t, d, href]) => <a key={t} className="card" href={href}><div className="card-top"><small>GitHub</small><ArrowUpRight /></div><strong>{t}</strong><p>{d}</p></a>)}</div>
          </div>
        </section>
      ))}
    </main>
  );
}
