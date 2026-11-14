import type React from "react";
import { downloadsFor, installerFor, releaseAssetUrl, signingNote, type PublicRelease } from "@/lib/release";
import { docs } from "@/lib/site";

const DATA_FOLDERS = "%LOCALAPPDATA%\\org.medscale.desktop.preview on Windows, ~/Library/Application Support/org.medscale.desktop.preview on a Mac, ~/.local/share/org.medscale.desktop.preview on Linux";

/** Install, update and support steps for a qualified release. Only rendered when one exists. */
export function InstallGuide({ release }: { release: PublicRelease }) {
  const windows = installerFor(release, "windows");
  const portable = release.platforms["windows-portable"];
  const macs = downloadsFor(release, "macos");
  const macos = macs[0]?.file;
  const linux = downloadsFor(release, "linux");
  const cli = downloadsFor(release, "cli");
  const anyInstaller = windows ?? macos ?? linux[0]?.file ?? cli[0]?.file;
  const names = (list: { key: string; file: { fileName: string } }[], sep: string) =>
    list.map((m) => <code key={m.key}>{m.file.fileName}</code>).reduce<React.ReactNode[]>((a, c, i) => (i ? [...a, sep, c] : [c]), []);
  return (
    <>
      <section className="page-section" aria-labelledby="install-title">
        <div className="shell stack-lg">
          <h2 id="install-title" className="h2 h2-sm"><span className="mute">Install</span> in a few steps.</h2>
          <p className="body">No account, app store or GitHub login. Compare the checksum before you open the file. If it differs, delete the file and report it.</p>
          <div className="install-steps">
            {windows && (
              <article className="install-card">
                <h3>Windows (x64)</h3>
                <ol>
                  <li>Download <code>{windows.fileName}</code>.</li>
                  <li>In PowerShell, run <code>Get-FileHash $HOME\Downloads\{windows.fileName} -Algorithm SHA256</code> and compare it with the SHA-256 above.</li>
                  <li>Open the file and follow the installer. It installs for your user only.</li>
                  {windows.signing === "unsigned" && <li>If SmartScreen says it protected your PC, this build isn&apos;t code-signed. If the checksum matched and your organization allows it, choose More info, then Run anyway. Never turn SmartScreen off.</li>}
                  <li>Open MedScale and choose Try MedScale with sample data.</li>
                </ol>
                <p className="platform-note">{signingNote(windows.signing)} Needs the Microsoft Edge WebView2 Runtime, which Windows 11 includes. Without it, MedScale shows a message and doesn&apos;t start. Automated tests ran on Windows Server, so desktop editions of Windows are less tested.</p>
              </article>
            )}
            {macos && (
              <article className="install-card">
                <h3>macOS</h3>
                <ol>
                  <li>Download the DMG for your Mac: {names(macs, " or ")}. Apple menu, About This Mac, shows Apple silicon or Intel.</li>
                  <li>In Terminal, run <code>shasum -a 256 ~/Downloads/MedScale_*.dmg</code> and compare it with the SHA-256 above.</li>
                  <li>Open the disk image and drag MedScale to Applications.</li>
                  {macos.signing === "ad-hoc" && <li>macOS blocks the first open because this build isn&apos;t notarized. If the checksum matched and your organization allows it, approve this app under System Settings, Privacy &amp; Security, Open Anyway. Never turn Gatekeeper off.</li>}
                  <li>Open MedScale and choose Try MedScale with sample data.</li>
                </ol>
                <p className="platform-note">{signingNote(macos.signing)} Needs macOS 10.15 or later. Automated tests ran on a current macOS version only. To uninstall, move MedScale from Applications to the Bin; your data stays until you delete its folder.</p>
              </article>
            )}
            {portable && (
              <article className="install-card">
                <h3>Windows portable (x64)</h3>
                <ol>
                  <li>Download <code>{portable.fileName}</code> and check its SHA-256 with <code>Get-FileHash</code>.</li>
                  <li>Extract it to a folder you own, for example in Documents.</li>
                  <li>Run <code>MedScale.exe</code>. Nothing is installed: delete the folder to remove it.</li>
                </ol>
                <p className="platform-note">{signingNote(portable.signing, "windows", "windows-portable")} Your data is stored in the same place as the installed app.</p>
              </article>
            )}
            {linux.length > 0 && (
              <article className="install-card">
                <h3>Linux (x64 and arm64)</h3>
                <ol>
                  <li>Download the file for your machine: {names(linux, ", ")}. <code>uname -m</code> says x86_64 or aarch64.</li>
                  <li>Check it with <code>sha256sum</code> against the SHA-256 above.</li>
                  <li>Debian or Ubuntu: <code>sudo apt install ./MedScale_*.deb</code>, then open MedScale from your applications. Remove it with <code>sudo apt remove medscale</code>.</li>
                  <li>AppImage: <code>chmod +x MedScale_*.AppImage</code> and run it. Delete the file to remove it.</li>
                </ol>
                <p className="platform-note">Not signed. Needs WebKitGTK 4.1 (Ubuntu 22.04 or newer). There&apos;s no package repository yet, so install a new version over the old one by hand.</p>
              </article>
            )}
            {cli.length > 0 && (
              <article className="install-card">
                <h3>Command-line tool</h3>
                <ol>
                  <li>Download the archive for your system ({names(cli, ", ")}) and check it against <code>SHA256SUMS</code>.</li>
                  <li>Extract it anywhere and put the folder on your <code>PATH</code>. No Rust toolchain or runtime is needed.</li>
                  <li>Run <code>medscale --version</code> and <code>medscale status --json</code>.</li>
                </ol>
                <p className="platform-note">Not signed. The same SmartScreen and Gatekeeper prompts apply to the binary.</p>
              </article>
            )}
          </div>
        </div>
      </section>
      <section className="page-section" aria-labelledby="notes-title">
        <div className="shell stack-lg">
          <h2 id="notes-title" className="h2 h2-sm"><span className="mute">MedScale</span> {release.version} Beta.</h2>
          <ul className="body release-notes">{release.notes.map((n) => <li key={n}>{n}</li>)}</ul>
          <p className="body">MedScale runs on your computer, with network access denied by default. Model downloads ask first. This isn&apos;t a certified no-network guarantee. Real patient records and clinical decisions aren&apos;t authorized, and MedScale isn&apos;t clinically validated.</p>
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
        <div className="shell stack-lg">
          <h2 id="after-title" className="h2 h2-sm"><span className="mute">Updates</span> and help.</h2>
          <p className="body">MedScale doesn&apos;t update itself. Check this page for new versions and run the newer installer. This is the first Beta, so updating between versions hasn&apos;t been tested yet: before you update, quit MedScale and copy its data folder ({DATA_FOLDERS}). Keep your passphrase and recovery code somewhere safe: nobody can recover an encrypted workspace without them.</p>
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
