"use client";
import { useEffect, useState } from "react";
import { detectPlatform, platformStatus, selectUnsignedPreview, type Platform, type UnsignedPreview, type UnsignedPreviewRelease } from "@/lib/release";
import { docs } from "@/lib/site";
import { ArrowUpRight, PlatformIcon, type PlatformName } from "./Icons";

const order: PlatformName[] = ["windows", "macos", "linux"];
const titles: Record<PlatformName, string> = { windows: "Windows", macos: "macOS", linux: "Linux" };
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
 * Official signed releases remain closed. Optional, explicitly unsigned
 * engineering previews are discovered only from this repo's GitHub pre-releases.
 * They are labelled synthetic-only and never imply medical/production approval.
 */
export function DownloadCards() {
  const visitor = useVisitorPlatform();
  const [preview, setPreview] = useState<UnsignedPreview>();
  useEffect(() => {
    let active = true;
    fetch("https://api.github.com/repos/AbdulazizShehri/MedScale/releases?per_page=10", {
      headers: { Accept: "application/vnd.github+json" },
    })
      .then((r) => {
        if (!r.ok) throw new Error("release lookup failed");
        return r.json();
      })
      .then((data: unknown) => {
        if (active && Array.isArray(data)) setPreview(selectUnsignedPreview(data as UnsignedPreviewRelease[]));
      })
      .catch(() => { /* No preview is safer than a stale or unverified installer link. */ });
    return () => { active = false; };
  }, []);
  return (
    <div className="platforms">
      {order.map((key) => {
        const s = platformStatus[key];
        const current = visitor === key;
        const unsigned = key === "windows" ? preview?.windows : key === "macos" ? preview?.macos : undefined;
        return (
          <article key={key} className="platform" data-current={current}>
            <div className="platform-top">
              <span className="platform-icon"><PlatformIcon name={key} size={26} /></span>
              {current && <span className="platform-you">Your platform</span>}
            </div>
            <h3>{titles[key]}</h3>
            <div className="platform-sub">{s.surface} · {s.arch}</div>
            <span className="platform-state">{s.state}</span>
            <p className="platform-note">{s.note}</p>
            {unsigned && (
              <>
                <p className="platform-note"><strong>MedScale Beta — unsigned · synthetic data only.</strong> Still in active development; not validated for patient records or clinical use. Windows SmartScreen or macOS Gatekeeper may block first launch. Verify the release checksum before running.</p>
                <a className={"btn " + (current ? "btn-primary" : "btn-secondary")} href={unsigned.browser_download_url} rel="noopener noreferrer">
                  Download MedScale Beta <ArrowUpRight />
                </a>
              </>
            )}
            <a className={"btn " + (unsigned ? "btn-secondary" : current ? "btn-primary" : "btn-secondary")} href={guide[key].href}>{guide[key].label} <ArrowUpRight /></a>
          </article>
        );
      })}
    </div>
  );
}
