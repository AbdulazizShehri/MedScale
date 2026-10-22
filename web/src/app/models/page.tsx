import type { Metadata } from "next";
import { ArrowUpRight } from "@/components/Icons";
import { Shot } from "@/components/Shot";
import { docs, github, pageMeta } from "@/lib/site";

export const metadata: Metadata = pageMeta("/models/", "Models", "How MedScale lists, admits and runs local AI models, and why a catalog listing is not clinical qualification.");

const states = [
  ["Listed", "The model appears in the catalog with its source, task, format and download size.", "Not installed. Not reviewed. Nothing has run."],
  ["Admitted", "A local model pack folder is admitted. The Core verifies its manifest, digests and signature.", "Pack registry state is per session today."],
  ["Running", "The local runtime executes the model on your device and records a receipt for each run.", "Output is a proposal, never an authoritative record."],
  ["Qualified", "Independent evaluation shows the model is fit for one specific task.", "Not granted to any model today, and never by the software alone."],
];

export default function Models() {
  return (
    <main id="main">
      <section className="shell page-hero">
        <span className="eyebrow">Models</span>
        <h1 className="display" style={{ fontSize: "clamp(40px, 6vw, 72px)" }}>Find a model.<br /><span className="mute">Then decide if you trust it.</span></h1>
        <p className="lede">MedScale treats models as things to inspect, not as authority. You can see where a model came from, what runs it and what state it is in, before you put it to work.</p>
      </section>
      <section className="page-section">
        <div className="shell" style={{ display: "grid", gap: 32 }}>
          <div className="section-head" style={{ marginBottom: 0 }}>
            <h2 className="h2" style={{ fontSize: "clamp(28px, 3.4vw, 40px)" }}><span className="mute">Four states,</span> always visible.</h2>
          </div>
          <div className="table-wrap" role="region" aria-label="Scrollable table" tabIndex={0}>
            <table>
              <thead><tr><th scope="col">State</th><th scope="col">What it means</th><th scope="col">What it does not mean</th></tr></thead>
              <tbody>{states.map(([s, m, n]) => <tr key={s}><td>{s}</td><td>{m}</td><td>{n}</td></tr>)}</tbody>
            </table>
          </div>
        </div>
      </section>
      <section className="page-section">
        <div className="shell feature-row">
          <div className="feature-copy">
            <span className="eyebrow">Catalog</span>
            <h2 className="h2" style={{ fontSize: "clamp(28px, 3.4vw, 40px)" }}><span className="mute">Browse first.</span> Admit only what you choose.</h2>
            <p className="body">The catalog in the desktop app lists models with their task, format, size and source identity. Search by name, family or language and filter by task, format and status. Browsing downloads nothing; a model runs only after you admit a local pack folder for it.</p>
            <p className="body">External model registries are sources, not authority. MedScale records where an entry came from; it does not inherit trust from it.</p>
          </div>
          <Shot name="models" title="Models" />
        </div>
      </section>
      <section className="page-section">
        <div className="shell" style={{ display: "grid", gap: 28 }}>
          <div className="section-head" style={{ marginBottom: 0 }}>
            <span className="eyebrow">Runtimes</span>
            <h2 className="h2" style={{ fontSize: "clamp(28px, 3.4vw, 40px)" }}><span className="mute">On your machine,</span> within clear limits.</h2>
          </div>
          <div className="cards">
            <div className="card"><div className="card-top"><small>Runtime</small></div><strong>ONNX</strong><p>Admitted packs run on tract, a pure-Rust ONNX runtime, inside the Core. No native runtime to install.</p></div>
            <div className="card"><div className="card-top"><small>Runtime</small></div><strong>Rust encoders</strong><p>Small encoder models such as token classifiers run on Candle, in Rust, on the CPU.</p></div>
            <div className="card"><div className="card-top"><small>Audio</small></div><strong>Local speech-to-text</strong><p>Transcription runs on device. It has been checked on synthetic speech only.</p></div>
          </div>
          <div className="notice"><strong>Current limits</strong><p>Model packs are bounded to 1 GiB, so large generative and multimodal models are not supported yet. No model is qualified for any clinical task.</p></div>
          <div className="actions"><a className="btn btn-secondary" href={docs.qualification}>Qualification boundaries <ArrowUpRight /></a><a className="btn btn-secondary" href={`${github}/tree/main/crates/medscale-pack`}>medscale-pack source <ArrowUpRight /></a></div>
        </div>
      </section>
    </main>
  );
}
