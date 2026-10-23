"use client";

import { useEffect, useState } from "react";
import { fetchInstallerDownloads, type InstallerDownloadStats } from "@/lib/download-stats";

/** Download events reported by GitHub, not completed installations. */
export function DownloadCount({ placement = "hero" }: { placement?: "hero" | "page" }) {
  const [stats, setStats] = useState<InstallerDownloadStats | null>(null);
  useEffect(() => {
    const abort = new AbortController();
    const load = () => {
      void fetchInstallerDownloads(abort.signal)
        .then((value) => { if (!abort.signal.aborted) setStats(value); })
        .catch(() => { if (!abort.signal.aborted) setStats(null); });
    };
    load();
    const interval = window.setInterval(load, 10 * 60 * 1000);
    return () => { abort.abort(); window.clearInterval(interval); };
  }, []);

  return (
    <div className="download-count" data-placement={placement} aria-label="MedScale installer download statistics">
      <span className="download-count-value" aria-live="polite" aria-atomic="true">
        {stats ? stats.total.toLocaleString("en-US") : "—"}
      </span>
      <div className="download-count-copy">
        <strong>Installer downloads to date</strong>
        <span>{stats
          ? `${stats.windows.toLocaleString("en-US")} Windows · ${stats.macos.toLocaleString("en-US")} macOS`
          : "Live statistics temporarily unavailable"}</span>
        <a href="https://github.com/AbdulazizShehri/MedScale/releases">
          From GitHub Releases · downloads, not verified installations
        </a>
      </div>
    </div>
  );
}
