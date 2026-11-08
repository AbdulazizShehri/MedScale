// node --test --experimental-strip-types src/lib/release.test.ts
// Cases follow TheHalfMoon/CommunityFinance-CoFi tests/site-release-selector.test.cjs, plus MedScale's gate.
import { test } from "node:test";
import assert from "node:assert/strict";
import { QUALIFIED_RELEASE_TAGS, assetArch, detectPlatform, formatSize, installerFor, pickInstaller, platformLabel, publicRelease, scoreAsset, signingNote, validateRelease, type PublicRelease, type ReleaseAsset } from "./release.ts";
import { QUALIFICATION_WORKFLOW, verifyPlatformAssets, verifyReleaseRecords } from "./release-verify.ts";
import { createHash } from "node:crypto";
import fs from "node:fs";

const a = (name: string): ReleaseAsset => ({ name, browser_download_url: `https://example.test/${name}`, size: 50 * 1024 * 1024 });

test("the gate and the built-in manifest agree", () => {
  assert.equal(QUALIFIED_RELEASE_TAGS.length > 0, publicRelease !== undefined);
  assert.equal(QUALIFIED_RELEASE_TAGS.length > 0, installerFor(publicRelease, "windows") !== undefined || installerFor(publicRelease, "macos") !== undefined);
});

test("platform detection", () => {
  assert.equal(detectPlatform("Win32"), "windows");
  assert.equal(detectPlatform("Windows"), "windows");
  assert.equal(detectPlatform("MacIntel"), "macos");
  assert.equal(detectPlatform("macOS"), "macos");
  assert.equal(detectPlatform("Darwin"), "macos", "darwin must not match win");
  assert.equal(detectPlatform("Linux x86_64"), "linux");
  assert.equal(detectPlatform("Linux armv81 Android"), "unknown");
  assert.equal(detectPlatform("iPhone"), "unknown");
});

test("scoring and architecture", () => {
  assert.equal(scoreAsset("MedScale_0.1.0_x64-setup.exe", "windows"), 40);
  assert.equal(scoreAsset("MedScale_0.1.0_x64-setup.exe", "macos"), 0);
  assert.equal(scoreAsset("MedScale_0.1.0_aarch64.dmg", "macos"), 50);
  assert.equal(scoreAsset("SBOM.cdx.json", "linux"), 0);
  assert.equal(scoreAsset("MedScale_0.1.0_x64-setup.exe.sig", "windows"), 0);
  assert.equal(assetArch("MedScale_0.1.0_aarch64.dmg"), "arm64");
  assert.equal(assetArch("MedScale_0.1.0_x64-setup.exe"), "x64");
  assert.equal(formatSize(50 * 1024 * 1024), "50.0 MB");
  assert.equal(platformLabel("macos"), "macOS");
});

test("installer selection never guesses between architectures", () => {
  assert.equal(pickInstaller([], "windows"), undefined);
  assert.equal(pickInstaller([a("MedScale_0.1.0_x64-setup.exe")], "windows")?.name, "MedScale_0.1.0_x64-setup.exe");
  const mac = [a("MedScale_0.1.0_aarch64.dmg"), a("MedScale_0.1.0_x64.dmg")];
  assert.equal(pickInstaller(mac, "macos"), undefined, "ambiguous without a known architecture");
  assert.equal(pickInstaller(mac, "macos", "arm64")?.name, "MedScale_0.1.0_aarch64.dmg");
  assert.equal(pickInstaller([...mac, a("MedScale_0.1.0_universal.dmg")], "macos")?.name, "MedScale_0.1.0_universal.dmg");
  assert.equal(pickInstaller([a("MedScale_0.1.0_x64-setup.exe")], "linux"), undefined, "no Linux desktop build");
});

const TAG = "v0.0.0-beta.0";
const fixture = () => JSON.parse(fs.readFileSync(new URL("./release.fixture.json", import.meta.url), "utf8"));
const validate = (m: unknown, tags: readonly string[] = [TAG]) => validateRelease(m, tags);
/** Applies a change to the fixture's Windows entry and returns its errors. */
const windowsErrors = (change: (w: Record<string, unknown>) => void) => {
  const m = fixture();
  change(m.release.platforms.windows);
  const r = validate(m);
  assert.equal(r.release, undefined, "a bad only-platform leaves nothing to offer");
  return r.errors.join("; ");
};
const macEntry = (over: Record<string, unknown> = {}) => ({
  fileName: "MedScale_0.0.0_aarch64.dmg",
  url: `https://github.com/AbdulazizShehri/MedScale/releases/download/${TAG}/MedScale_0.0.0_aarch64.dmg`,
  sha256: "b".repeat(64),
  size: 60 * 1024 * 1024,
  signing: "ad-hoc",
  signingDecision: "https://github.com/AbdulazizShehri/MedScale/issues/436#issuecomment-2",
  qualificationRun: "https://github.com/AbdulazizShehri/MedScale/actions/runs/2",
  companions: { checksums: "macos-SHA256SUMS", sbom: "macos-SBOM.cdx.json", notices: "macos-NOTICE.md", packageManifest: "macos-package-manifest.json" },
  ...over,
});

test("the empty manifest offers nothing and has no errors", () => {
  assert.deepEqual(validate({ schema: 1, release: null }), { errors: [] });
  assert.match(validate({ schema: 2, release: null }).errors[0], /schema/);
  assert.match(validate(null).errors[0], /schema/);
});

test("a valid release is offered only for the platforms it contains", () => {
  const r = validate(fixture());
  assert.deepEqual(r.errors, []);
  assert.equal(installerFor(r.release, "windows")?.fileName, "MedScale_0.0.0_x64-setup.exe");
  assert.equal(installerFor(r.release, "macos"), undefined, "unqualified platform stays unavailable");
  assert.equal(installerFor(r.release, "linux"), undefined);
  assert.equal(installerFor(r.release, "unknown"), undefined);
});

test("the gate refuses a tag it doesn't list, whatever the manifest says", () => {
  assert.match(validate(fixture(), []).errors[0], /not in the qualification gate/);
  assert.equal(validate(fixture(), []).release, undefined);
  assert.equal(validateRelease(fixture()).release, undefined, "the shipped gate is closed");
});

test("malicious or mutable asset URLs are refused", () => {
  for (const url of [
    "https://evil.example/MedScale_0.0.0_x64-setup.exe",
    "http://github.com/AbdulazizShehri/MedScale/releases/download/v0.0.0-beta.0/MedScale_0.0.0_x64-setup.exe",
    "https://github.com/AbdulazizShehri/MedScale/releases/latest/download/MedScale_0.0.0_x64-setup.exe",
    "https://github.com/someone-else/MedScale/releases/download/v0.0.0-beta.0/MedScale_0.0.0_x64-setup.exe",
    "https://github.com/AbdulazizShehri/MedScale/releases/download/v0.0.0-beta.1/MedScale_0.0.0_x64-setup.exe",
    "https://github.com/AbdulazizShehri/MedScale/releases/download/v0.0.0-beta.0/MedScale_0.0.0_x64-setup.exe?x=1",
    "javascript:alert(1)",
    undefined,
  ]) assert.match(windowsErrors((w) => { w.url = url; }), /immutable asset/, String(url));
});

test("wrong or missing installer details are refused", () => {
  assert.match(windowsErrors((w) => { w.fileName = "MedScale_0.0.0_x64.msi"; }), /file name/);
  assert.match(windowsErrors((w) => { w.fileName = "../MedScale_0.0.0_x64-setup.exe"; }), /file name/);
  assert.match(windowsErrors((w) => { w.fileName = "MedScale_0.0.0_aarch64-setup.exe"; }), /file name/, "wrong architecture");
  assert.match(windowsErrors((w) => { w.fileName = "MedScale_9.9.9_x64-setup.exe"; w.url = `https://github.com/AbdulazizShehri/MedScale/releases/download/${TAG}/MedScale_9.9.9_x64-setup.exe`; }), /version differs/);
  assert.match(windowsErrors((w) => { w.sha256 = "ABC"; }), /sha256/);
  assert.match(windowsErrors((w) => { delete w.sha256; }), /sha256/);
  assert.match(windowsErrors((w) => { w.sha256 = "A".repeat(64); }), /sha256/, "uppercase isn't the canonical form");
  assert.match(windowsErrors((w) => { w.size = 0; }), /size/);
  assert.match(windowsErrors((w) => { w.size = "52428800"; }), /size/);
  assert.match(windowsErrors((w) => { delete w.qualificationRun; }), /qualification run/);
  assert.match(windowsErrors((w) => { w.qualificationRun = "https://evil.example/actions/runs/1"; }), /qualification run/);
  assert.match(windowsErrors((w) => { delete w.companions; }), /checksums/);
  assert.match(windowsErrors((w) => { (w.companions as Record<string, string>).sbom = "../x"; }), /sbom/);
});

test("a platform without a recorded signing decision is refused", () => {
  assert.match(windowsErrors((w) => { delete w.signing; }), /signing state/);
  assert.match(windowsErrors((w) => { delete w.signingDecision; }), /signing decision/);
  assert.match(windowsErrors((w) => { w.signingDecision = "https://evil.example/#issuecomment-1"; }), /signing decision/);
  assert.match(windowsErrors((w) => { w.signing = "developer-id-notarized"; }), /signing state/, "Apple's state can't apply to Windows");
});

test("an ad-hoc DMG can't be described as notarized", () => {
  const m = fixture();
  m.release.platforms.macos = macEntry({ signing: "notarized" });
  assert.match(validate(m).errors.join(), /macos: signing state/);
  assert.match(signingNote("ad-hoc"), /not notarized/);
  assert.match(signingNote("unsigned"), /Not code-signed/);
  m.release.platforms.macos = macEntry();
  assert.equal(installerFor(validate(m).release, "macos")?.signing, "ad-hoc");
});

test("one bad platform doesn't hide the good one, and Linux can't be offered", () => {
  const m = fixture();
  m.release.platforms.macos = macEntry({ sha256: "nope" });
  m.release.platforms.linux = macEntry();
  const r = validate(m);
  assert.ok(installerFor(r.release, "windows"));
  assert.equal(installerFor(r.release, "macos"), undefined);
  assert.match(r.errors.join(), /macos: sha256/);
  assert.match(r.errors.join(), /linux: not a supported/);
});

test("release-level gaps refuse everything", () => {
  for (const [change, why] of [
    [(r: Record<string, unknown>) => { delete r.founderApproval; }, /founder approval/],
    [(r: Record<string, unknown>) => { r.sourceSha = "main"; }, /source SHA/],
    [(r: Record<string, unknown>) => { r.notes = []; }, /release notes/],
    [(r: Record<string, unknown>) => { r.version = "9.9.9"; }, /version differs/],
    [(r: Record<string, unknown>) => { r.tag = "latest"; }, /tag is malformed/],
    [(r: Record<string, unknown>) => { r.tag = "v0.0.0"; r.version = "0.0.0"; }, /tag is malformed/],
  ] as const) {
    const m = fixture();
    change(m.release);
    const r = validate(m);
    assert.equal(r.release, undefined);
    assert.match(r.errors.join(), why);
  }
  const m = fixture();
  m.release.platforms = {};
  assert.equal(validate(m).release, undefined);
});

// A published release as the network would serve it.
function served(over: { bytes?: Buffer; sums?: string; source?: string; redirect?: string; missing?: string; pkg?: Record<string, unknown>; sbom?: string; hugeNotice?: boolean } = {}) {
  const bytes = Buffer.from("installer bytes");
  const sha256 = createHash("sha256").update(bytes).digest("hex");
  const m = fixture().release;
  const sourceSha = "1".repeat(40);
  const w = { ...m.platforms.windows, sha256, size: bytes.length };
  const release: PublicRelease = { ...m, sourceSha, platforms: { windows: w } };
  const base = `https://github.com/AbdulazizShehri/MedScale/releases/download/${TAG}/`;
  const files: Record<string, BodyInit> = {
    [w.fileName]: new Uint8Array(over.bytes ?? bytes),
    [w.companions.checksums]: over.sums ?? `${sha256}  windows/${w.fileName}\n`,
    [w.companions.packageManifest]: "\uFEFF" + JSON.stringify({
      source_sha: over.source ?? sourceSha, installer: { sha256 }, platform: "windows", version: "0.0.0", signed: false, notarized: false,
      sbom_sha256: createHash("sha256").update("{}").digest("hex"), notice_sha256: createHash("sha256").update("notice").digest("hex"), ...over.pkg,
    }),
    [w.companions.sbom]: over.sbom ?? "{}",
    [w.companions.notices]: over.hugeNotice ? new Uint8Array(65 * 1024 * 1024) : "notice",
  };
  const seen: RequestInit[] = [];
  const fetcher = async (url: string, init: RequestInit) => {
    seen.push(init);
    if (over.redirect && url.endsWith(w.fileName)) return new Response(null, { status: 302, headers: { location: over.redirect } });
    if (url.startsWith("https://release-assets.githubusercontent.com/")) return new Response(new Uint8Array(bytes));
    const name = url.startsWith(base) ? url.slice(base.length) : "";
    if (name === over.missing || !(name in files)) return new Response("gone", { status: 404 });
    return new Response(files[name]);
  };
  return { release, fetcher, seen };
}

test("published bytes must match the manifest", async () => {
  const good = served();
  assert.deepEqual(await verifyPlatformAssets(good.release, "windows", good.fetcher), []);
  assert.ok(good.seen.every((i) => i.credentials === "omit" && !i.headers), "downloads use no login");

  const tampered = served({ bytes: Buffer.from("different bytes") });
  assert.match((await verifyPlatformAssets(tampered.release, "windows", tampered.fetcher)).join(), /installer SHA-256 is/);

  const short = served({ bytes: Buffer.from("x") });
  assert.match((await verifyPlatformAssets(short.release, "windows", short.fetcher)).join(), /bytes, manifest says/);

  const sums = served({ sums: `${"c".repeat(64)}  windows/MedScale_0.0.0_x64-setup.exe\n` });
  assert.match((await verifyPlatformAssets(sums.release, "windows", sums.fetcher)).join(), /checksums file disagrees/);

  const noEntry = served({ sums: "nothing here\n" });
  assert.match((await verifyPlatformAssets(noEntry.release, "windows", noEntry.fetcher)).join(), /no entry/);

  const source = served({ source: "2".repeat(40) });
  assert.match((await verifyPlatformAssets(source.release, "windows", source.fetcher)).join(), /package manifest source/);

  const noSbom = served({ missing: "windows-SBOM.cdx.json" });
  assert.match((await verifyPlatformAssets(noSbom.release, "windows", noSbom.fetcher)).join(), /SBOM: HTTP 404/);

  const wrongSbom = served({ sbom: "another file" });
  assert.match((await verifyPlatformAssets(wrongSbom.release, "windows", wrongSbom.fetcher)).join(), /SBOM digest disagrees/);

  const huge = served({ hugeNotice: true });
  assert.match((await verifyPlatformAssets(huge.release, "windows", huge.fetcher)).join(), /larger than a companion/);

  const lookalike = served({ sums: `${"c".repeat(64)}  x-MedScale_0.0.0_x64-setup.exe\n` });
  assert.match((await verifyPlatformAssets(lookalike.release, "windows", lookalike.fetcher)).join(), /no entry/, "a longer name isn't this file");

  const same = served();
  same.release.platforms.windows!.companions.notices = same.release.platforms.windows!.companions.sbom;
  assert.match((await verifyPlatformAssets(same.release, "windows", same.fetcher)).join(), /must be distinct/);

  assert.match((await verifyPlatformAssets(good.release, "macos", good.fetcher)).join(), /not in the release/);
});

test("redirects are followed only to GitHub's download hosts", async () => {
  const ok = served({ redirect: "https://release-assets.githubusercontent.com/x" });
  assert.deepEqual(await verifyPlatformAssets(ok.release, "windows", ok.fetcher), []);
  const evil = served({ redirect: "https://evil.example/x" });
  assert.match((await verifyPlatformAssets(evil.release, "windows", evil.fetcher)).join(), /refused host evil\.example/);
  const plain = served({ redirect: "http://release-assets.githubusercontent.com/x" });
  assert.match((await verifyPlatformAssets(plain.release, "windows", plain.fetcher)).join(), /refused host/);
});

test("the signing claim must match what was packaged", async () => {
  const claimed = served();
  claimed.release.platforms.windows!.signing = "authenticode";
  assert.match((await verifyPlatformAssets(claimed.release, "windows", claimed.fetcher)).join(), /signed=false, release says authenticode/);

  const notarized = served({ pkg: { notarized: true } });
  assert.match((await verifyPlatformAssets(notarized.release, "windows", notarized.fetcher)).join(), /notarized=true/);

  const platform = served({ pkg: { platform: "macos" } });
  assert.match((await verifyPlatformAssets(platform.release, "windows", platform.fetcher)).join(), /package manifest is for macos/);

  const version = served({ pkg: { version: "9.9.9" } });
  assert.match((await verifyPlatformAssets(version.release, "windows", version.fetcher)).join(), /package manifest version/);
});

// The GitHub records behind a release.
function records(over: { release?: object; tagSha?: string; approvalBy?: string; approvalId?: number; approvalBody?: string; edited?: boolean; issue?: string; decisionBody?: string; run?: object } = {}) {
  const { release } = served();
  const w = release.platforms.windows!;
  const API = "https://api.github.com/repos/AbdulazizShehri/MedScale/";
  const routes: Record<string, unknown> = {
    [`releases/tags/${TAG}`]: { draft: false, immutable: true, ...over.release },
    [`commits/${TAG}`]: { sha: over.tagSha ?? release.sourceSha },
    "issues/comments/1": {
      user: { login: over.approvalBy ?? "AbdulazizShehri", id: over.approvalId ?? 222180173 },
      body: over.approvalBody ?? `GO.\nAPPROVE ${w.sha256}\nSIGNING windows unsigned`,
      created_at: "2026-11-08T10:00:00Z",
      updated_at: over.edited ? "2026-11-08T11:00:00Z" : "2026-11-08T10:00:00Z",
      issue_url: `https://api.github.com/repos/AbdulazizShehri/MedScale/issues/${over.issue ?? "436"}`,
    },
    "actions/runs/1": {
      conclusion: "success", name: QUALIFICATION_WORKFLOW, path: ".github/workflows/release-installed-journey.yml",
      event: "push", head_repository: { full_name: "AbdulazizShehri/MedScale" }, ...over.run,
    },
  };
  if (over.decisionBody !== undefined) {
    release.platforms.windows!.signingDecision = "https://github.com/AbdulazizShehri/MedScale/issues/436#issuecomment-2";
    routes["issues/comments/2"] = {
      user: { login: "AbdulazizShehri", id: 222180173 }, body: over.decisionBody,
      created_at: "2026-11-08T10:00:00Z", updated_at: "2026-11-08T10:00:00Z",
      issue_url: "https://api.github.com/repos/AbdulazizShehri/MedScale/issues/436",
    };
  }
  const fetcher = async (url: string) => {
    const route = url.startsWith(API) ? url.slice(API.length) : "";
    return route in routes ? Response.json(routes[route]) : new Response("gone", { status: 404 });
  };
  return { release, fetcher };
}

test("approval and qualification are checked against GitHub, not by URL shape", async () => {
  const check = async (over: Parameters<typeof records>[0]) => {
    const r = records(over);
    return (await verifyReleaseRecords(r.release, r.fetcher)).join();
  };
  assert.equal(await check({}), "");
  assert.match(await check({ release: { draft: true } }), /still a draft/);
  assert.match(await check({ release: { immutable: false } }), /not immutable/);
  assert.match(await check({ tagSha: "2".repeat(40) }), /the tag is at/);
  assert.match(await check({ approvalBy: "someone-else", approvalId: 1 }), /founder approval: written by someone-else/);
  assert.match(await check({ approvalId: 7 }), /founder approval: written by AbdulazizShehri/, "a reused login is not the founder");
  assert.match(await check({ edited: true }), /edited after it was posted/);
  assert.match(await check({ issue: "999" }), /different issue/);
  assert.match(await check({ approvalBody: "GO for something else" }), /APPROVE <windows installer SHA-256>/);
  const w = records().release.platforms.windows!;
  assert.match(await check({ approvalBody: `do NOT ship ${w.sha256}\nSIGNING windows unsigned` }), /APPROVE </, "naming the bytes is not approving them");
  assert.match(await check({ decisionBody: "signed" }), /doesn't name the state unsigned/);
  assert.match(await check({ run: { conclusion: "failure" } }), /concluded failure/);
  assert.match(await check({ run: { name: "other" } }), /run is from other/);
  assert.match(await check({ run: { path: ".github/workflows/fork.yml" } }), /run used \.github\/workflows\/fork\.yml/);
  assert.match(await check({ run: { event: "pull_request" } }), /triggered by pull_request/);
  assert.match(await check({ run: { head_repository: { full_name: "someone/fork" } } }), /came from someone\/fork/);
});
