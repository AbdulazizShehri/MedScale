import type { Metadata } from "next";
import { DownloadCards } from "@/components/DownloadCards";
import { ArrowUpRight } from "@/components/Icons";
import { Quickstart } from "@/components/Quickstart";
import { docs, pageMeta } from "@/lib/site";

export const metadata: Metadata = pageMeta("/download/", "Download", "MedScale desktop for Windows and macOS and the CLI for Windows, macOS and Linux. Current release status and build-from-source steps.");

const status = [
  ["Windows x64", "Desktop app + CLI", "Build from source", "CI builds an unsigned per-user NSIS installer to test packaging. It is not signed and is not published."],
  ["macOS (Apple silicon)", "Desktop app + CLI", "Build from source", "CI builds an ad-hoc signed DMG to test packaging. It is not notarized and is not published."],
  ["Linux x86_64", "CLI", "CLI from source", "The Rust Core and CLI are tested on Ubuntu. A Linux desktop app is deferred."],
];

export default function Download() {
  return (
    <main id="main">
      <section className="shell page-hero">
        <span className="eyebrow">Download</span>
        <h1 className="display" style={{ fontSize: "clamp(40px, 6vw, 72px)" }}>Get MedScale.<br /><span className="mute">Start from the source.</span></h1>
        <p className="lede">There is no signed public installer yet. When a release is signed and qualified, this page will offer the right installer for your platform. Until then, every option here is real and links to working instructions.</p>
      </section>
      <section className="page-section" aria-labelledby="platforms-title">
        <div className="shell">
          <h2 id="platforms-title" className="sr-only">Platforms</h2>
          <DownloadCards />
        </div>
      </section>
      <section className="page-section" aria-labelledby="cli-title">
        <div className="shell dev">
          <div style={{ display: "grid", gap: 20 }}>
            <span className="eyebrow">Build from source</span>
            <h2 id="cli-title" className="h2" style={{ fontSize: "clamp(28px, 3.4vw, 40px)" }}><span className="mute">The CLI,</span> in four commands.</h2>
            <p className="body">You need Git and the Rust toolchain. On Windows, the vendored OpenSSL build also needs Strawberry Perl. The desktop app adds Node.js and the Tauri prerequisites; its guide has the exact steps.</p>
            <div className="actions"><a className="btn btn-secondary" href={docs.desktop}>Desktop build guide <ArrowUpRight /></a><a className="btn btn-secondary" href={docs.cli}>CLI guide <ArrowUpRight /></a></div>
          </div>
          <Quickstart />
        </div>
      </section>
      <section className="page-section" aria-labelledby="status-title">
        <div className="shell" style={{ display: "grid", gap: 24 }}>
          <h2 id="status-title" className="h2" style={{ fontSize: "clamp(28px, 3.4vw, 40px)" }}><span className="mute">Release status,</span> plainly.</h2>
          <div className="table-wrap" role="region" aria-label="Scrollable table" tabIndex={0}>
            <table>
              <thead><tr><th scope="col">Platform</th><th scope="col">Surface</th><th scope="col">Today</th><th scope="col">Packaging</th></tr></thead>
              <tbody>{status.map((r) => <tr key={r[0]}>{r.map((c, i) => <td key={i}>{c}</td>)}</tr>)}</tbody>
            </table>
          </div>
          <div className="notice"><strong>Not a production release</strong><p>Engineering builds are for testing the release process. Production signing, notarization, clinical validation and permission to use real patient data are external gates that have not been met. <a className="text-link" style={{ fontSize: 14 }} href={docs.qualification}>Read the external gates</a> · <a className="text-link" style={{ fontSize: 14 }} href={docs.releases}>GitHub Releases</a></p></div>
        </div>
      </section>
    </main>
  );
}
