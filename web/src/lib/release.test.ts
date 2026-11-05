// node --test --experimental-strip-types src/lib/release.test.ts
// Cases follow TheHalfMoon/CommunityFinance-CoFi tests/site-release-selector.test.cjs, plus MedScale's gate.
import { test } from "node:test";
import assert from "node:assert/strict";
import { QUALIFIED_RELEASE_TAGS, assetArch, detectPlatform, formatSize, installerFor, pickInstaller, platformLabel, publicRelease, scoreAsset, signingNote, validateRelease, type PublicRelease, type ReleaseAsset } from "./release.ts";
import { verifyPlatformAssets } from "./release-verify.ts";
import { createHash } from "node:crypto";
import fs from "node:fs";

const a = (name: string): ReleaseAsset => ({ name, browser_download_url: `https://example.test/${name}`, size: 50 * 1024 * 1024 });

test("no release is qualified for public download", () => {
  assert.deepEqual(QUALIFIED_RELEASE_TAGS, []);
  assert.equal(publicRelease, undefined);
  assert.equal(installerFor(publicRelease, "windows"), undefined);
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
function served(over: { bytes?: Buffer; sums?: string; source?: string; redirect?: string; missing?: string } = {}) {
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
    [w.companions.packageManifest]: "\uFEFF" + JSON.stringify({ source_sha: over.source ?? sourceSha, installer: { sha256 } }),
    [w.companions.sbom]: "{}",
    [w.companions.notices]: "notice",
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
