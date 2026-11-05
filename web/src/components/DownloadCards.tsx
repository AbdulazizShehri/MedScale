"use client";
import { useEffect, useState } from "react";
import { detectPlatform, formatSize, installerFor, platformStatus, publicRelease, signingNote, type Platform } from "@/lib/release";
import { docs } from "@/lib/site";
import { ArrowUpRight, PlatformIcon, type PlatformName } from "./Icons";

const order: PlatformName[] = ["windows", "macos", "linux"];
const titles: Record<PlatformName, string> = { windows: "Windows", macos: "macOS", linux: "Linux" };
const fileKind: Record<string, string> = { windows: ".exe", macos: ".dmg" };
const guide: Record<PlatformName, { href: string; label: string }> = {
  windows: { href: docs.desktop, label: "Build the desktop app" },
  macos: { href: docs.desktop, label: "Build the desktop app" },
  linux: { href: docs.cli, label: "Install the CLI" },
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
 * Platform cards. A card offers an installer only when src/lib/release.ts validated one
 * for that platform; every other card links to build instructions.
 */
export function DownloadCards() {
  const visitor = useVisitorPlatform();
  return (
    <div className="platforms">
      {order.map((key) => {
        const s = platformStatus[key];
        const current = visitor === key;
        const installer = installerFor(publicRelease, key);
        return (
          <article key={key} className="platform" data-current={current}>
            <div className="platform-top">
              <span className="platform-icon"><PlatformIcon name={key} size={26} /></span>
              {current && <span className="platform-you">Your platform</span>}
            </div>
            <h3>{titles[key]}</h3>
            <div className="platform-sub">{s.surface} · {s.arch}</div>
            {installer && publicRelease ? (
              <>
                <span className="platform-state" data-available="true">MedScale {publicRelease.version} Beta · {formatSize(installer.size)}</span>
                <p className="platform-note">{signingNote(installer.signing)}</p>
                <a className={"btn " + (current ? "btn-primary" : "btn-secondary")} href={installer.url}>Download for {titles[key]} ({fileKind[key]})</a>
                <div className="platform-hash"><span>SHA-256</span><code>{installer.sha256}</code></div>
              </>
            ) : (
              <>
                <span className="platform-state">{s.state}</span>
                <p className="platform-note">{s.note}</p>
                <a className={"btn " + (current ? "btn-primary" : "btn-secondary")} href={guide[key].href}>{guide[key].label} <ArrowUpRight /></a>
              </>
            )}
          </article>
        );
      })}
    </div>
  );
}
