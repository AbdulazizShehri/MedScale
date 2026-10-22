import Link from "next/link";
import { PreviewBoard } from "@/components/PreviewBoard";
import { ProductExplorer } from "@/components/ProductExplorer";
import { DownloadCards } from "@/components/DownloadCards";
import { docs, github } from "@/lib/site";

export default function Home() {
 return <main id="main">
  <section className="hero">
    <div className="shell hero-grid">
      <div className="hero-copy">
        <div className="eyebrow"><span className="tiny-bar"/> LOCAL-FIRST HEALTH INTELLIGENCE</div>
        <h1>Health intelligence.<br/><em>With the evidence<br/>in view.</em></h1>
        <p className="hero-lead">An open-source workspace that brings records, evidence, local models, and governed workflows together. Built for clarity, traceability, and human oversight.</p>
        <div className="hero-actions"><Link className="button button-white" href="/features/">Explore the platform <span>↗</span></Link><Link className="button button-outline" href="/download/">Get MedScale <span>↓</span></Link></div>
        <div className="hero-meta"><div><span className="meta-plus">+</span> OPEN SOURCE <small>Apache-2.0 first-party code</small></div><div><span className="meta-plus">◇</span> LOCAL FIRST <small>Rust Core + CLI + desktop</small></div></div>
      </div>
      <div className="hero-visual"><div className="visual-above"><span>MEDSCALE / PRODUCT CONCEPT</span><span>01—04</span></div><PreviewBoard/><div className="visual-below"><span><i/> DESIGN ILLUSTRATION</span><span>NOT A REAL CLINICAL RECORD OR SCREENSHOT</span></div></div>
    </div>
  </section>

  <section className="system-strip"><div className="shell"><span>BUILT AROUND ONE TRUSTED CORE</span><div><b>Rust</b><b>Tauri</b><b>FHIR R4</b><b>Local models</b><b>Evidence provenance</b></div></div></section>

  <section className="section shell" id="product"><div className="section-heading"><div className="eyebrow"><span className="tiny-bar"/> 01 / THE PLATFORM</div><div><h2>One workspace.<br/><em>More context.</em></h2><p>Navigate the work, understand the evidence, and preserve the boundary between a proposal and a verified decision.</p></div></div>
    <ProductExplorer/>
  </section>

  <section className="principle-band"><div className="shell principle-wrap"><div className="principle-header"><div className="eyebrow dark-eyebrow">02 / HOW IT WORKS</div><h2>Evidence first.<br/><em>Action second.</em></h2><p>Built to put reviewable context ahead of automation. MedScale doesn't turn a model output into authority just because it sounds convincing.</p></div>
    <div className="steps"><article><span>01</span><h3>Prepare</h3><p>Work with governed local data and source-aware inputs.</p></article><article><span>02</span><h3>Understand</h3><p>Inspect records, references, gaps, uncertainty, and model status.</p></article><article><span>03</span><h3>Review</h3><p>Keep consequential steps explicit, auditable, and subject to permission.</p></article></div>
    <div className="principle-bottom"><span>VERIFIABLE CONTEXT</span><span>HUMAN AUTHORITY BY DESIGN</span></div>
  </div></section>

  <section className="section shell" id="foundations"><div className="section-heading"><div className="eyebrow"><span className="tiny-bar"/> 03 / ENGINEERING</div><div><h2>Serious foundations.<br/><em>Visible limitations.</em></h2><p>Core architecture, integrity boundaries, and offline-first thinking form the basis of the open-source engineering preview.</p></div></div>
    <div className="pillar-grid">
      <article><span className="pillar-number">01 / SOURCE</span><span className="pillar-symbol">↗</span><h3>Traceable by design.</h3><p>Sources, provenance, and audit state remain distinct from derived summaries.</p></article>
      <article><span className="pillar-number">02 / BOUNDARIES</span><span className="pillar-symbol">⌑</span><h3>Controlled local data.</h3><p>Encrypted vault controls and default-denied network actions, without claiming end-to-end production privacy qualification.</p></article>
      <article><span className="pillar-number">03 / MODELS</span><span className="pillar-symbol">◈</span><h3>Inspect model state.</h3><p>Model catalogs, admission, local runtime availability, and clinical qualification are different states.</p></article>
      <article><span className="pillar-number">04 / INTERCHANGE</span><span className="pillar-symbol">⇄</span><h3>FHIR with scope.</h3><p>Bounded FHIR R4 workflows and synthetic testing, not live EHR connectivity or full conformance certification.</p></article>
    </div>
    <Link href="/features/" className="inline-arrow">View all platform areas <span>↗</span></Link>
  </section>

  <section className="section section-download" id="download"><div className="shell"><div className="section-heading"><div className="eyebrow"><span className="tiny-bar"/> 04 / GET MEDSCALE</div><div><h2>Start exploring.<br/><em>On your terms.</em></h2><p>Windows and macOS engineering desktop paths, and Rust Core + CLI across supported systems. No production installer has been released.</p></div></div><DownloadCards/><div className="download-end"><Link className="inline-arrow" href="/download/">View installation and release status <span>↗</span></Link><p>Signed public downloads will appear here only after qualification and release.</p></div></div></section>

  <section className="community-promo"><div className="shell community-promo-inner"><div><div className="eyebrow"><span className="tiny-bar"/> 05 / COMMUNITY</div><h2>Open code.<br/><em>Open collaboration.</em></h2><p>Read the architecture, review a bug with synthetic fixtures, and help improve MedScale. Technical clarity starts in the open.</p><div className="hero-actions"><a className="button button-white" href={github}>View on GitHub <span>↗</span></a><Link className="button button-outline" href="/community/">Join the community <span>↗</span></Link></div></div><div className="community-panel"><div className="panel-row"><span>LICENSE</span><strong>Apache-2.0</strong></div><div className="panel-row"><span>CONTRIBUTIONS</span><strong>Welcome</strong></div><div className="panel-row"><span>STATUS</span><strong>Engineering preview</strong></div><div className="panel-row"><span>REAL PATIENT DATA</span><strong>Not authorized</strong></div><a href={docs.contribution} className="community-panel-link">Read contributor guidelines <span>↗</span></a></div></div></section>

  <section className="release-disclosure"><div className="shell disclosure-grid"><div className="eyebrow">CURRENT QUALIFICATION</div><div><h2>Built in public.<br/>Not yet clinically qualified.</h2><p>MedScale is an engineering preview for <strong>synthetic data only</strong>. It is not clinically validated, approved for handling real patient information, or released as a signed production desktop application. No diagnosis or treatment decisions should rely on it.</p><a href={docs.qualification} className="inline-arrow">Read qualification gates <span>↗</span></a></div></div></section>
 </main>;
}
