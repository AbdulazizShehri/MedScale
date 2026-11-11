import type { Metadata } from "next";
import { DownloadCards } from "@/components/DownloadCards";
import { ArrowUpRight } from "@/components/Icons";
import { InstallGuide } from "@/components/InstallGuide";
import { Quickstart } from "@/components/Quickstart";
import { publicRelease } from "@/lib/release";
import { docs, pageMeta } from "@/lib/site";

export const metadata: Metadata = pageMeta("/download/", "Download", "MedScale desktop and CLI for Windows, macOS and Linux. Release status, install guides, checksums and build-from-source steps.");

const status = [
  ["Windows x64", "Desktop app + CLI", "Not released yet", "CI builds and tests an unsigned per-user installer (installed, opened, uninstalled) and a portable zip (opened). Not code-signed."],
  ["macOS (Apple silicon and Intel)", "Desktop app + CLI", "Not released yet", "CI builds and tests DMGs on Apple silicon and on Intel (installed, opened, removed). Ad-hoc signed, not notarized."],
  ["Linux (x64 and arm64)", "Desktop app + CLI", "Not released yet", "CI builds and tests .deb packages (installed, opened, removed) and AppImages (opened). Not signed."],
];

export default function Download() {
  return (
    <main id="main">
      <section className="shell page-hero">
        <span className="eyebrow">Download</span>
        {publicRelease ? (
          <>
            <h1 className="display display-sub">Get MedScale Beta.<br /><span className="mute">Free and local-first.</span></h1>
            <p className="lede">Download, install and open it. You don&apos;t need an account, an app store or a GitHub login.</p>
          </>
        ) : (
          <>
            <h1 className="display display-sub">Get MedScale.<br /><span className="mute">Start from the source.</span></h1>
            <p className="lede">There is no qualified public installer yet. When a release is qualified and approved, this page will offer the right installer for your platform. Until then, every option here is real and links to working instructions.</p>
          </>
        )}
      </section>
      <section className="page-section" aria-labelledby="platforms-title">
        <div className="shell">
          <h2 id="platforms-title" className="sr-only">Platforms</h2>
          <DownloadCards />
        </div>
      </section>
      {publicRelease && <InstallGuide release={publicRelease} />}
      <section className="page-section" aria-labelledby="cli-title">
        <div className="shell dev">
          <div className="stack">
            <span className="eyebrow">Build from source</span>
            <h2 id="cli-title" className="h2 h2-sm"><span className="mute">The CLI,</span> in four commands.</h2>
            <p className="body">You need Git and the Rust toolchain. On Windows, the vendored OpenSSL build also needs Strawberry Perl. The desktop app adds Node.js and the Tauri prerequisites; its guide has the exact steps.</p>
            <div className="actions"><a className="btn btn-secondary" href={docs.desktop}>Desktop build guide <ArrowUpRight /></a><a className="btn btn-secondary" href={docs.cli}>CLI guide <ArrowUpRight /></a></div>
          </div>
          <Quickstart />
        </div>
      </section>
      <section className="page-section" aria-labelledby="status-title">
        <div className="shell">
          <div className="section-head"><h2 id="status-title" className="h2 h2-sm"><span className="mute">Release status,</span> plainly.</h2></div>
          <div className="stack-lg">
          <div className="table-wrap" role="region" aria-label="Scrollable table" tabIndex={0}>
            <table className="table-cols-4">
              <thead><tr><th scope="col">Platform</th><th scope="col">Surface</th><th scope="col">Today</th><th scope="col">Packaging</th></tr></thead>
              <tbody>{status.map((r) => <tr key={r[0]}>{r.map((c, i) => <td key={i}>{c}</td>)}</tr>)}</tbody>
            </table>
          </div>
          <div className="notice"><strong>Not a production release</strong><p>{publicRelease ? "MedScale is in Beta, for synthetic data only." : "Engineering builds are for testing the release process."} No build is code-signed by a trusted publisher; each download says what that means. Clinical validation and permission to use real patient data are external gates that have not been met. Each installer states its own signing status. <a className="text-link" href={docs.qualification}>Read the external gates</a> · <a className="text-link" href={docs.releases}>GitHub Releases</a></p></div>
          </div>
        </div>
      </section>
    </main>
  );
}
