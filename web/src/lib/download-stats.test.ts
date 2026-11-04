import { test } from "node:test";
import assert from "node:assert/strict";
import { countInstallerDownloads } from "./download-stats.ts";

const tag = "engineering-preview-012345abcdef";
const base = `https://github.com/AbdulazizShehri/MedScale/releases/download/${tag}/`;
const file = (name: string, download_count: number) => ({
  name, download_count, browser_download_url: base + name, size: 1500,
});
const win = "MedScale-engineering-preview-windows-x64.exe";
const mac = "MedScale-engineering-preview-macos.dmg";
const release = {
  tag_name: tag, draft: false, prerelease: true,
  assets: [file(win, 2), file(mac, 1), file("SHA256SUMS", 200), file("MedScale-engineering-preview-windows-provenance.zip", 70)],
};

test("total counts Windows and macOS installers only", () => {
  assert.deepEqual(countInstallerDownloads([release]), { total: 3, windows: 2, macos: 1 });
});
test("counts across all published beta versions", () => {
  const next = { ...release, tag_name: "engineering-preview-000000000000", assets: [{
    ...file(win, 5), browser_download_url: "https://github.com/AbdulazizShehri/MedScale/releases/download/engineering-preview-000000000000/" + win,
  }] };
  assert.deepEqual(countInstallerDownloads([release, next]), { total: 8, windows: 7, macos: 1 });
});
test("excludes drafts, stable tags, unrelated projects and non-installer assets", () => {
  assert.deepEqual(countInstallerDownloads([{ ...release, draft: true }, { ...release, prerelease: false }, { ...release, tag_name: "v0.1.0" }]), { total: 0, windows: 0, macos: 0 });
  assert.deepEqual(countInstallerDownloads([{ ...release, assets: [file("random.exe", 123), file("SHA256SUMS", 50)] }]), { total: 0, windows: 0, macos: 0 });
});
test("malformed and untrusted API data cannot fabricate a count", () => {
  assert.equal(countInstallerDownloads({}), null);
  assert.equal(countInstallerDownloads([null]), null);
  assert.equal(countInstallerDownloads([{ ...release, assets: null }]), null);
  assert.equal(countInstallerDownloads([{ ...release, assets: [{ ...file(win, 4), browser_download_url: "https://evil.example/test.exe" }] }]), null);
  assert.equal(countInstallerDownloads([{ ...release, assets: [file(win, -1)] }]), null);
  assert.equal(countInstallerDownloads([{ ...release, assets: [file(win, 1.2)] }]), null);
  assert.equal(countInstallerDownloads([{ ...release, assets: [file(win, Number.MAX_SAFE_INTEGER + 1)] }]), null);
});
test("a real zero is shown as zero, not as a missing count", () => {
  assert.deepEqual(countInstallerDownloads([]), { total: 0, windows: 0, macos: 0 });
});
