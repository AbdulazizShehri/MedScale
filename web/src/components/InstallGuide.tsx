import { installerFor, releaseAssetUrl, signingNote, type PublicRelease } from "@/lib/release";
import { docs } from "@/lib/site";

/** Install, update and support steps for a qualified release. Only rendered when one exists. */
export function InstallGuide({ release }: { release: PublicRelease }) {
  const windows = installerFor(release, "windows");
  const macos = installerFor(release, "macos");
  const anyInstaller = windows ?? macos;
  return (
    <>
      <section className="page-section" aria-labelledby="install-title">
        <div className="shell" style={{ display: "grid", gap: 24 }}>
          <h2 id="install-title" className="h2" style={{ fontSize: "clamp(28px, 3.4vw, 40px)" }}><span className="mute">Install</span> in a few steps.</h2>
          <p className="body">No account, app store or GitHub login. Compare the checksum before you open the file. If it differs, delete the file and report it.</p>
          <div className="install-steps">
            {windows && (
              <article className="install-card">
                <h3>Windows 10 or 11 (x64)</h3>
                <ol>
                  <li>Download <code>{windows.fileName}</code>.</li>
                  <li>In PowerShell, run <code>Get-FileHash .\{windows.fileName} -Algorithm SHA256</code> and compare it with the SHA-256 above.</li>
                  <li>Open the file and follow the installer. It installs for your user only.</li>
                  {windows.signing === "unsigned" && <li>If SmartScreen says it protected your PC, this build isn&apos;t code-signed. If the checksum matched and your organization allows it, choose More info, then Run anyway. Never turn SmartScreen off.</li>}
                  <li>Open MedScale and choose Try MedScale with sample data.</li>
                </ol>
                <p className="platform-note">{signingNote(windows.signing)} Needs the Microsoft Edge WebView2 Runtime, which Windows 11 includes.</p>
              </article>
            )}
            {macos && (
              <article className="install-card">
                <h3>macOS (Apple silicon)</h3>
                <ol>
                  <li>Download <code>{macos.fileName}</code>. Intel Macs aren&apos;t supported.</li>
                  <li>In Terminal, run <code>shasum -a 256 {macos.fileName}</code> and compare it with the SHA-256 above.</li>
                  <li>Open the disk image and drag MedScale to Applications.</li>
                  {macos.signing === "ad-hoc" && <li>macOS blocks the first open because this build isn&apos;t notarized. If the checksum matched and your organization allows it, approve this app under System Settings, Privacy &amp; Security, Open Anyway. Never turn Gatekeeper off.</li>}
                  <li>Open MedScale and choose Try MedScale with sample data.</li>
                </ol>
                <p className="platform-note">{signingNote(macos.signing)}</p>
              </article>
            )}
          </div>
        </div>
      </section>
      <section className="page-section" aria-labelledby="notes-title">
        <div className="shell" style={{ display: "grid", gap: 24 }}>
          <h2 id="notes-title" className="h2" style={{ fontSize: "clamp(28px, 3.4vw, 40px)" }}><span className="mute">MedScale</span> {release.version} Beta.</h2>
          <ul className="body release-notes">{release.notes.map((n) => <li key={n}>{n}</li>)}</ul>
          <p className="body">MedScale runs on your computer. It sends nothing unless you approve a download, and model downloads ask first. Real patient records and clinical decisions aren&apos;t authorized, and MedScale isn&apos;t clinically validated.</p>
          {anyInstaller && (
            <p className="body">
              Built from commit <code>{release.sourceSha}</code>.{" "}
              <a className="text-link" href={releaseAssetUrl(release.tag, anyInstaller.companions.checksums)}>Checksums</a> ·{" "}
              <a className="text-link" href={releaseAssetUrl(release.tag, anyInstaller.companions.sbom)}>SBOM</a> ·{" "}
              <a className="text-link" href={releaseAssetUrl(release.tag, anyInstaller.companions.notices)}>Notices</a> ·{" "}
              <a className="text-link" href={`${docs.releases}/tag/${release.tag}`}>Full release notes</a>
            </p>
          )}
        </div>
      </section>
      <section className="page-section" aria-labelledby="after-title">
        <div className="shell" style={{ display: "grid", gap: 24 }}>
          <h2 id="after-title" className="h2" style={{ fontSize: "clamp(28px, 3.4vw, 40px)" }}><span className="mute">Updates</span> and help.</h2>
          <p className="body">MedScale doesn&apos;t update itself. Check this page for new versions, back up your workspace, then run the newer installer. Keep your passphrase and recovery code somewhere safe: nobody can recover an encrypted workspace without them.</p>
          <p className="body">
            <a className="text-link" href={docs.firstRun}>First-run guide</a> ·{" "}
            <a className="text-link" href={docs.bugReport}>Report a problem</a> ·{" "}
            <a className="text-link" href={docs.security}>Report a security issue</a>
          </p>
          <p className="platform-note">Never attach patient information, passphrases, recovery codes or workspace files to a report.</p>
        </div>
      </section>
    </>
  );
}
