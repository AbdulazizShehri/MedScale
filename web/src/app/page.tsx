import Link from "next/link";
import { PreviewBoard } from "@/components/PreviewBoard";
import { ProductExplorer } from "@/components/ProductExplorer";
import { DownloadCards } from "@/components/DownloadCards";
import { docs, github } from "@/lib/site";

const principles = [
 {k:"01",title:"The source matters.",copy:"Keep every summary connected to its origin, context, and review state.",icon:"⌁"},
 {k:"02",title:"Local means control.",copy:"A trusted Rust Core with offline-first workflows and governed network actions.",icon:"◎"},
 {k:"03",title:"Models aren't authority.",copy:"Inspect runtime, provenance, and qualification states before you put a model to work.",icon:"◈"}
];
const answers = [
 {q:"Is MedScale ready for real patient records?",a:"No. MedScale is an open-source engineering preview for synthetic data only. Real patient information, production deployment, and clinical decisions are not authorized."},
 {q:"Can I download the desktop app?",a:"The Windows and macOS desktop projects have engineering build paths, but no signed or notarized production installers have been released. You can inspect or build the source using the setup guides. Linux currently supports Core and CLI workflows; the Linux desktop is deferred."},
 {q:"Does MedScale require cloud services?",a:"Basic Core workflows are local-first. Network actions are governed and default-denied. This does not establish complete endpoint security or a production privacy qualification."},
 {q:"Can I contribute to MedScale?",a:"Yes. Start with the contribution guide and open issues. Reports, examples, and tests must use synthetic data and must not include patient information or confidential materials."}
];

export default function Home(){
 return <main id="main" className="premium-home">
  <section className="p-hero" aria-labelledby="hero-title">
   <div className="p-orbit p-orbit-left" aria-hidden="true"/><div className="p-orbit p-orbit-right" aria-hidden="true"/>
   <div className="shell p-hero-inner">
    <div className="p-kicker"><span className="p-kicker-dot"/> INTRODUCING MEDSCALE <span className="p-kicker-slash">/</span> OPEN-SOURCE ENGINEERING PREVIEW</div>
    <h1 id="hero-title">A clearer view of<br/><em>health intelligence.</em></h1>
    <p className="p-hero-desc">Health information is complex. Understanding it shouldn&apos;t hide the evidence. Explore records, models, and governed workflows in one local-first, source-aware workspace.</p>
    <div className="p-actions">
      <Link className="p-primary" href="/features/">Explore the platform <span aria-hidden="true">↗</span></Link>
      <a className="p-secondary" href={github}>View the source <span aria-hidden="true">↗</span></a>
    </div>
    <div className="p-hero-download"><span className="p-mini-orbit" aria-hidden="true">↘</span><span>Built for <strong>Windows + macOS desktop</strong> and <strong>cross-platform Rust CLI</strong></span><Link href="/download/">Build status <span aria-hidden="true">→</span></Link></div>
   </div>
   <div className="shell p-stage-wrap">
    <div className="p-stage-label"><span>THE MEDSCALE EXPERIENCE</span><span>INTERACTIVE CONCEPT · SYNTHETIC EXAMPLES</span></div>
    <div className="p-stage-glow" aria-hidden="true"/>
    <PreviewBoard/>
    <div className="p-stage-bottom"><span><i/> DESIGNED AROUND EVIDENCE</span><span>CONCEPTUAL INTERFACE — NOT A REAL CLINICAL RECORD OR PRODUCT SCREENSHOT</span></div>
   </div>
  </section>
  <div className="p-principle-strip"><div className="shell"><span><i/> LOCAL FIRST</span><span><i/> SOURCE-AWARE</span><span><i/> HUMAN OVERSIGHT</span><span><i/> OPEN SOURCE</span></div></div>
  <section className="p-section shell" id="experience">
    <div className="p-heading"><div className="p-eyebrow"><span/> 01 / A DIFFERENT KIND OF WORKSPACE</div><div><h2>Everything in context.<br/><em>Nothing lost in translation.</em></h2><p>Move between the record, the evidence behind it, the models available, and the action that needs a human decision.</p></div></div>
    <ProductExplorer/>
    <div className="p-between-link"><span>DESIGNED FOR INSPECTION, NOT BLIND AUTOMATION</span><Link href="/features/">Explore all capabilities <span>↗</span></Link></div>
  </section>
  <section className="p-values" aria-labelledby="values-title"><div className="shell">
   <div className="p-values-header"><span className="p-eyebrow">02 / OUR APPROACH</span><h2 id="values-title">The intelligence is powerful.<br/><em>The context is essential.</em></h2><p>MedScale&apos;s architecture separates what is observed, what is inferred, and what is permitted. That distinction is the product.</p></div>
   <div className="p-values-grid">{principles.map(p=><article className="p-value" key={p.k}><div className="p-value-head"><span>{p.k} / PRINCIPLE</span><b aria-hidden="true">{p.icon}</b></div><div className="p-value-art" aria-hidden="true"><span/><span/><span/></div><h3>{p.title}</h3><p>{p.copy}</p></article>)}</div>
  </div></section>
  <section className="p-section p-architecture shell" aria-labelledby="architecture-title">
    <div className="p-heading"><div className="p-eyebrow"><span/> 03 / BUILT ON SOMETHING REAL</div><div><h2 id="architecture-title">One trusted Core.<br/><em>Multiple ways to work.</em></h2><p>Different experiences, one Rust authority boundary. Every supported surface builds around the same source-aware Core.</p></div></div>
    <div className="p-system">
      <div className="p-system-illustration" aria-label="Architecture illustration of a Rust Core providing Desktop and CLI interfaces">
        <div className="p-system-grid" aria-hidden="true"/>
        <div className="p-system-chip p-chip-top">LOCAL-FIRST ARCHITECTURE</div>
        <div className="p-system-node p-node-left"><span>01 / INTERFACE</span><strong>Desktop</strong><small>Tauri · React</small></div>
        <div className="p-system-node p-node-right"><span>02 / INTERFACE</span><strong>CLI</strong><small>Rust commands</small></div>
        <div className="p-system-path p-path-one" aria-hidden="true"/><div className="p-system-path p-path-two" aria-hidden="true"/>
        <div className="p-system-core"><span>SHARED AUTHORITY</span><strong>MEDSCALE CORE</strong><small>Rust · records · provenance · policy</small><i aria-hidden="true"/></div>
        <div className="p-system-footnote">Illustrative architecture — not a runtime monitoring view</div>
      </div>
      <div className="p-system-copy"><span className="p-eyebrow">YOUR DATA / YOUR CONTEXT</span><h3>The work stays close.<br/>The boundaries stay clear.</h3><p>Inspect synthetic FHIR records and local model states without relying on an external account for basic Core workflows. Network actions stay explicitly governed and default-denied.</p><ul><li>Source-aware longitudinal records</li><li>Inspectable model provenance and state</li><li>Bounded FHIR R4 interchange</li><li>Reviewable actions and audit history</li></ul><a href={docs.source} className="p-text-link">Explore the Rust Core <span>↗</span></a></div>
    </div>
  </section>
  <section className="p-download" id="download"><div className="shell">
    <div className="p-heading"><div className="p-eyebrow"><span/> 04 / RUN MEDSCALE</div><div><h2>The platform is open.<br/><em>Start with the source.</em></h2><p>Build and explore locally. Desktop engineering builds exist, but no signed, production-qualified installer is available for direct download yet.</p></div></div>
    <DownloadCards/>
    <div className="p-download-foot"><p>Engineering preview only. No real patient information or clinical use.</p><Link href="/download/">See all platform details <span>↗</span></Link></div>
  </div></section>
  <section className="p-community" id="community"><div className="shell p-community-grid"><div>
   <div className="p-eyebrow"><span/> 05 / BUILT IN THE OPEN</div>
   <h2>Better, together.<br/><em>In the open.</em></h2>
   <p>Everything starts with inspectable source. Study the architecture, contribute a synthetic test case, or take part in the design conversation.</p>
   <div className="p-actions"><Link className="p-primary" href="/community/">Join the community <span>↗</span></Link><a className="p-secondary" href={github}>GitHub repository <span>↗</span></a></div>
  </div><div className="p-community-card"><div className="p-community-card-head"><span>MEDSCALE / OPEN SOURCE</span><span className="p-connected-dot">● PUBLIC REPOSITORY</span></div>
   <div className="p-community-word">The code is the conversation<span>.</span></div>
   <div className="p-community-row"><span>FIRST-PARTY LICENSE</span><strong>Apache-2.0</strong></div><div className="p-community-row"><span>CONTRIBUTIONS</span><strong>Issues · Pull requests</strong></div><div className="p-community-row"><span>RELEASE STATE</span><strong>Engineering preview</strong></div><a href={docs.contribution}>Read the contribution guide <span>↗</span></a>
  </div></div></section>
  <section className="p-faq shell" aria-labelledby="faq-title"><div className="p-faq-intro"><div className="p-eyebrow"><span/> 06 / QUESTIONS</div><h2 id="faq-title">Before you begin.</h2><p>Clear answers about the project&apos;s current boundaries.</p></div><div className="p-faq-list">{answers.map(({q,a})=><details key={q}><summary>{q}<span aria-hidden="true">+</span></summary><p>{a}</p></details>)}</div></section>
  <section className="p-end"><div className="shell"><div className="p-end-ring" aria-hidden="true"/><div className="p-eyebrow">MEDSCALE / THE NEXT CHAPTER</div><h2>Evidence first.<br/><em>Action second.</em></h2><p>Built for better questions, clearer context, and deliberate decisions.</p><div className="p-actions"><Link href="/features/" className="p-primary">Explore MedScale <span>↗</span></Link><a href={docs.qualification} className="p-secondary">Qualification status <span>↗</span></a></div><div className="p-end-disclosure">SYNTHETIC DATA ONLY. NOT CLINICALLY VALIDATED OR AUTHORIZED FOR REAL PATIENT RECORDS.</div></div></section>
 </main>;
}
