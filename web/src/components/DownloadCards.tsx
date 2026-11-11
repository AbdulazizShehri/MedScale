"use client";
import { useEffect, useState } from "react";
import { archLabel, detectPlatform, downloadsFor, formatSize, kindLabel, platformStatus, publicRelease, signingNote, testedNote, type DownloadGroup, type Platform } from "@/lib/release";
import { docs } from "@/lib/site";
import { ArrowUpRight, PlatformIcon, type PlatformName } from "./Icons";

const order: DownloadGroup[] = ["windows", "macos", "linux", "cli"];
const titles: Record<DownloadGroup, string> = { windows: "Windows", macos: "macOS", linux: "Linux", cli: "Other platforms / CLI" };
const cta: Record<DownloadGroup, string> = { windows: "Download for Windows", macos: "Download for macOS", linux: "Download for Linux", cli: "Download the CLI" };
const fallback: Record<DownloadGroup, { surface: string; arch: string; state: string; note: string; href: string; label: string }> = {
  windows: { ...platformStatus.windows, href: docs.desktop, label: "Build the desktop app" },
  macos: { ...platformStatus.macos, href: docs.desktop, label: "Build the desktop app" },
  linux: { ...platformStatus.linux, href: docs.desktop, label: "Build the desktop app" },
  cli: { surface: "Command-line tool", arch: "Windows, macOS and Linux", state: "Not released yet", note: "Standalone archives for five targets are built and tested in CI. Nothing is published until a release is approved.", href: docs.cli, label: "Install the CLI from source" },
};

function useVisitorPlatform(): Platform {
  const [p, setP] = useState<Platform>("unknown");
  useEffect(() => {
    const nav = navigator as Navigator & { userAgentData?: { platform?: string } };
    setP(detectPlatform(nav.userAgentData?.platform || navigator.platform || ""));
  }, []);
  return p;
}

/**
 * One card per platform plus the CLI. A card lists a download only when
 * src/lib/release.ts validated that exact file; otherwise it says plainly that
 * nothing is released and links to build instructions. No placeholder links.
 */
export function DownloadCards() {
  const visitor = useVisitorPlatform();
  return (
    <div className="platforms">
      {order.map((group) => {
        const current = visitor === group;
        const files = downloadsFor(publicRelease, group);
        const f = fallback[group];
        return (
          <article key={group} className="platform" data-current={current}>
            <div className="platform-top">
              <span className="platform-icon">{group === "cli" ? <span aria-hidden="true" className="mono">&gt;_</span> : <PlatformIcon name={group as PlatformName} size={26} />}</span>
              {current && <span className="platform-you">Your platform</span>}
            </div>
            <h3>{titles[group]}</h3>
            {files.length > 0 && publicRelease ? (
              <>
                <div className="platform-sub">MedScale {publicRelease.version} Beta</div>
                {files.map(({ key, rule, file }, i) => (
                  <div key={key} className="stack" style={{ gap: 6, marginTop: i ? 14 : 4 }}>
                    <a className={"btn " + (current && i === 0 ? "btn-primary" : "btn-secondary")} href={file.url}>
                      {cta[group]} · {archLabel(rule.arch, group)} · {kindLabel(rule.kind)}
                    </a>
                    <span className="platform-state" data-available="true">{file.fileName} · {formatSize(file.size)}</span>
                    <p className="platform-note">{signingNote(file.signing, group, key)} {testedNote(rule.kind)}</p>
                    <div className="platform-hash"><span>SHA-256</span><code>{file.sha256}</code></div>
                  </div>
                ))}
              </>
            ) : (
              <>
                <div className="platform-sub">{f.surface} · {f.arch}</div>
                <span className="platform-state">{f.state}</span>
                <p className="platform-note">{f.note}</p>
                <a className={"btn " + (current ? "btn-primary" : "btn-secondary")} href={f.href}>{f.label} <ArrowUpRight /></a>
              </>
            )}
          </article>
        );
      })}
    </div>
  );
}
