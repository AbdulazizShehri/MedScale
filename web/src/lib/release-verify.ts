// Checks a release against the bytes visitors would actually receive, without
// credentials. Node only: run by scripts/verify-release.mjs, never by the site.
import { createHash } from "node:crypto";
import { releaseAssetUrl, type InstallerPlatform, type PublicRelease } from "./release.ts";

const DOWNLOAD_HOSTS = new Set(["github.com", "release-assets.githubusercontent.com", "objects.githubusercontent.com"]);
type Fetcher = (url: string, init: RequestInit) => Promise<Response>;

/** GETs a URL with no credentials, refusing a redirect to any host outside GitHub's download hosts. */
async function get(url: string, fetcher: Fetcher): Promise<Response> {
  let current = url;
  for (let hop = 0; hop < 5; hop++) {
    const parsed = new URL(current);
    if (parsed.protocol !== "https:" || !DOWNLOAD_HOSTS.has(parsed.hostname)) throw new Error(`refused host ${parsed.hostname}`);
    const response = await fetcher(current, { redirect: "manual", credentials: "omit" });
    if (response.status >= 300 && response.status < 400) {
      const next = response.headers.get("location");
      if (!next) throw new Error(`redirect without a location from ${parsed.hostname}`);
      current = new URL(next, current).toString();
      continue;
    }
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return response;
  }
  throw new Error("too many redirects");
}

async function text(tag: string, file: string, fetcher: Fetcher): Promise<string> {
  return (await (await get(releaseAssetUrl(tag, file), fetcher)).text()).replace(/^﻿/, "");
}

/** Every problem found with one platform's published assets; empty means they match. */
export async function verifyPlatformAssets(release: PublicRelease, key: InstallerPlatform, fetcher: Fetcher = fetch): Promise<string[]> {
  const p = release.platforms[key];
  if (!p) return [`${key}: not in the release`];
  const errors: string[] = [];
  const attempt = async (what: string, run: () => Promise<void>) => {
    try { await run(); } catch (e) { errors.push(`${key}: ${what}: ${(e as Error).message}`); }
  };

  await attempt("installer", async () => {
    const body = (await get(p.url, fetcher)).body;
    if (!body) throw new Error("empty response");
    const hash = createHash("sha256");
    let size = 0;
    for await (const chunk of body as unknown as AsyncIterable<Uint8Array>) { hash.update(chunk); size += chunk.length; }
    const digest = hash.digest("hex");
    if (digest !== p.sha256) errors.push(`${key}: installer SHA-256 is ${digest}, manifest says ${p.sha256}`);
    if (size !== p.size) errors.push(`${key}: installer is ${size} bytes, manifest says ${p.size}`);
  });
  await attempt("checksums", async () => {
    const line = (await text(release.tag, p.companions.checksums, fetcher)).split(/\r?\n/).find((l) => l.trim().endsWith(p.fileName));
    if (!line) errors.push(`${key}: checksums file has no entry for ${p.fileName}`);
    else if (line.trim().split(/\s+/)[0].toLowerCase() !== p.sha256) errors.push(`${key}: checksums file disagrees with the manifest`);
  });
  await attempt("package manifest", async () => {
    const m = JSON.parse(await text(release.tag, p.companions.packageManifest, fetcher));
    if (m.source_sha !== release.sourceSha) errors.push(`${key}: package manifest source is ${m.source_sha}, manifest says ${release.sourceSha}`);
    if (m.installer?.sha256 !== p.sha256) errors.push(`${key}: package manifest installer digest disagrees`);
  });
  for (const [what, file] of [["SBOM", p.companions.sbom], ["notices", p.companions.notices]] as const) {
    await attempt(what, async () => { await get(releaseAssetUrl(release.tag, file), fetcher); });
  }
  return errors;
}
