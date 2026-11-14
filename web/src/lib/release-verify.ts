// Checks a release against what visitors would actually receive, and against the
// records behind it. Node only: run by scripts/verify-release.mjs, never by the site.
import { createHash } from "node:crypto";
import { DOWNLOADS, REPOSITORY, releaseAssetUrl, type InstallerPlatform, type PublicRelease, type SigningState } from "./release.ts";

const DOWNLOAD_HOSTS = new Set(["github.com", "release-assets.githubusercontent.com", "objects.githubusercontent.com"]);
const MAX_COMPANION_BYTES = 64 * 1024 * 1024;
/** The founder's numeric GitHub account id. A login can be renamed and reused; the id can't. */
const OWNER_ID = 222180173;
/** Workflows allowed to qualify a release, and the event each must have run on. */
const QUALIFIERS: Record<string, { path: string; event: string }> = {
  "release-installed-journey": { path: ".github/workflows/release-installed-journey.yml", event: "push" },
  "release-candidate": { path: ".github/workflows/release-candidate.yml", event: "workflow_dispatch" },
};
const API = `https://api.github.com/repos/${REPOSITORY}`;
/** The workflow that runs the journey on the exact installer. */
export const QUALIFICATION_WORKFLOW = "release-installed-journey";
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

/** A companion file's bytes, refusing anything larger than a companion should be. */
async function companion(tag: string, file: string, fetcher: Fetcher): Promise<Buffer> {
  const body = (await get(releaseAssetUrl(tag, file), fetcher)).body;
  if (!body) throw new Error("empty response");
  const chunks: Uint8Array[] = [];
  let size = 0;
  for await (const chunk of body as unknown as AsyncIterable<Uint8Array>) {
    size += chunk.length;
    if (size > MAX_COMPANION_BYTES) throw new Error("larger than a companion file should be");
    chunks.push(chunk);
  }
  return Buffer.concat(chunks);
}
const sha256 = (bytes: Uint8Array) => createHash("sha256").update(bytes).digest("hex");
const text = (bytes: Buffer) => bytes.toString("utf8").replace(/^﻿/, "");

const SIGNED: Record<SigningState, { signed: boolean; notarized: boolean }> = {
  authenticode: { signed: true, notarized: false },
  unsigned: { signed: false, notarized: false },
  "developer-id-notarized": { signed: true, notarized: true },
  "ad-hoc": { signed: false, notarized: false },
};

/** Every problem found with one platform's published assets; empty means they match. */
export async function verifyPlatformAssets(release: PublicRelease, key: InstallerPlatform, fetcher: Fetcher = fetch): Promise<string[]> {
  const p = release.platforms[key];
  if (!p) return [`${key}: not in the release`];
  const errors: string[] = [];
  const attempt = async (what: string, run: () => Promise<void>) => {
    try { await run(); } catch (e) { errors.push(`${key}: ${what}: ${(e as Error).message}`); }
  };

  const names = Object.values(p.companions);
  if (new Set(names).size !== names.length || names.includes(p.fileName)) errors.push(`${key}: companion files must be distinct`);

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
    const line = text(await companion(release.tag, p.companions.checksums, fetcher))
      .split(/\r?\n/)
      .map((l) => l.trim().split(/\s+/))
      .find((fields) => fields.length === 2 && fields[1].split(/[\\/]/).pop()?.replace(/^\*/, "") === p.fileName);
    if (!line) errors.push(`${key}: checksums file has no entry for ${p.fileName}`);
    else if (line[0].toLowerCase() !== p.sha256) errors.push(`${key}: checksums file disagrees with the manifest`);
  });
  if (!p.companions.packageManifest) {
    // Portable zips, AppImages and CLI archives have no package manifest: their
    // SBOM and notices must appear, with matching digests, in the checksums file.
    await attempt("companions", async () => {
      const sums = text(await companion(release.tag, p.companions.checksums, fetcher));
      for (const [what, file] of [["SBOM", p.companions.sbom], ["notices", p.companions.notices]] as const) {
        const digest = sha256(await companion(release.tag, file, fetcher));
        if (!sums.split(/\r?\n/).some((l) => l.trim() === `${digest}  ${file}`)) errors.push(`${key}: ${what} isn't in the checksums file with its digest`);
      }
    });
    return errors;
  }
  const manifestPlatform = DOWNLOADS[key].manifestPlatform;
  await attempt("package manifest", async () => {
    const m = JSON.parse(text(await companion(release.tag, p.companions.packageManifest as string, fetcher)));
    if (m.source_sha !== release.sourceSha) errors.push(`${key}: package manifest source is ${m.source_sha}, manifest says ${release.sourceSha}`);
    if (m.installer?.sha256 !== p.sha256) errors.push(`${key}: package manifest installer digest disagrees`);
    if (m.platform !== manifestPlatform) errors.push(`${key}: package manifest is for ${m.platform}`);
    if (m.version !== release.version.replace(/-beta\.\d+$/, "")) errors.push(`${key}: package manifest version is ${m.version}`);
    const claimed = SIGNED[p.signing];
    if (m.signed !== claimed.signed) errors.push(`${key}: package manifest says signed=${m.signed}, release says ${p.signing}`);
    if (m.notarized !== claimed.notarized) errors.push(`${key}: package manifest says notarized=${m.notarized}, release says ${p.signing}`);
    for (const [what, file, expected] of [["SBOM", p.companions.sbom, m.sbom_sha256], ["notices", p.companions.notices, m.notice_sha256]] as const) {
      await attempt(what, async () => {
        if (sha256(await companion(release.tag, file, fetcher)) !== expected) errors.push(`${key}: ${what} digest disagrees with the package manifest`);
      });
    }
  });
  return errors;
}

/**
 * Checks the records behind a release: its tag, the founder's approval of these
 * exact bytes, and the runs that qualified them. A URL's shape proves none of this.
 */
export async function verifyReleaseRecords(release: PublicRelease, fetcher: Fetcher = fetch): Promise<string[]> {
  const errors: string[] = [];
  const api = async (path: string) => {
    const response = await fetcher(`${API}/${path}`, { redirect: "follow", credentials: "omit", headers: { accept: "application/vnd.github+json" } });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return response.json();
  };
  const attempt = async (what: string, run: () => Promise<void>) => {
    try { await run(); } catch (e) { errors.push(`${what}: ${(e as Error).message}`); }
  };
  // Anyone with write access can edit another person's comment, so an edited
  // comment proves nothing about what the founder wrote.
  const founderComment = async (url: string) => {
    const [issuePart, id] = url.split("#issuecomment-");
    const comment = await api(`issues/comments/${id}`);
    if (comment.user?.id !== OWNER_ID) throw new Error(`written by ${comment.user?.login}`);
    if (!comment.created_at || comment.updated_at !== comment.created_at) throw new Error("the comment was edited after it was posted");
    const issue = issuePart.split("/").pop();
    if (!String(comment.issue_url ?? "").endsWith(`/issues/${issue}`)) throw new Error("the comment belongs to a different issue");
    return String(comment.body ?? "").replace(/\r/g, "");
  };
  const hasLine = (body: string, line: string) => body.split("\n").some((l) => l.trim() === line);

  await attempt("release", async () => {
    const r = await api(`releases/tags/${release.tag}`);
    if (r.draft) errors.push("the release is still a draft");
    if (r.immutable !== true) errors.push("the release is not immutable, so its assets could be replaced");
  });
  await attempt("tag", async () => {
    const sha = (await api(`commits/${release.tag}`)).sha;
    if (sha !== release.sourceSha) errors.push(`the tag is at ${sha}, manifest says ${release.sourceSha}`);
  });
  await attempt("founder approval", async () => {
    const body = await founderComment(release.founderApproval);
    for (const [key, p] of Object.entries(release.platforms)) {
      if (!hasLine(body, `APPROVE ${p.sha256}`)) errors.push(`founder approval doesn't have the line "APPROVE <${key} installer SHA-256>"`);
    }
  });
  for (const [key, p] of Object.entries(release.platforms)) {
    await attempt(`${key} signing decision`, async () => {
      const body = await founderComment(p.signingDecision);
      if (!hasLine(body, `SIGNING ${key} ${p.signing}`)) errors.push(`${key}: signing decision doesn't name the state ${p.signing}`);
    });
    await attempt(`${key} qualification run`, async () => {
      const run = await api(`actions/runs/${p.qualificationRun.split("/").pop()}`);
      if (run.conclusion !== "success") errors.push(`${key}: qualification run concluded ${run.conclusion}`);
      const qualifier = QUALIFIERS[run.name];
      if (!qualifier) errors.push(`${key}: qualification run is from ${run.name}`);
      // A run's name comes from whatever workflow file ran, so the file, trigger
      // and repository are checked too.
      else {
        if (run.path !== qualifier.path) errors.push(`${key}: qualification run used ${run.path}`);
        if (run.event !== qualifier.event) errors.push(`${key}: qualification run was triggered by ${run.event}`);
      }
      if (run.head_repository?.full_name !== REPOSITORY) errors.push(`${key}: qualification run came from ${run.head_repository?.full_name}`);
    });
  }
  return errors;
}
