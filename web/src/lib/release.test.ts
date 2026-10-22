// node --test --experimental-strip-types src/lib/release.test.ts
// Cases follow TheHalfMoon/CommunityFinance-CoFi tests/site-release-selector.test.cjs, plus MedScale's gate.
import { test } from "node:test";
import assert from "node:assert/strict";
import { assetArch, detectPlatform, formatSize, pickInstaller, platformLabel, releaseQualified, scoreAsset, type ReleaseAsset } from "./release.ts";

const a = (name: string): ReleaseAsset => ({ name, browser_download_url: `https://example.test/${name}`, size: 50 * 1024 * 1024 });

test("no release is qualified for public download", () => {
  assert.equal(releaseQualified, false);
});

test("platform detection", () => {
  assert.equal(detectPlatform("Win32"), "windows");
  assert.equal(detectPlatform("Windows"), "windows");
  assert.equal(detectPlatform("MacIntel"), "macos");
  assert.equal(detectPlatform("macOS"), "macos");
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
