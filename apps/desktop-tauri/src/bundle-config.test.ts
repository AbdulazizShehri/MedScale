// Spec 101 T101-01: the bundle configuration stays inside its authorized scope.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const conf = JSON.parse(readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));

test("bundling is explicit: no implicit 'all' targets that would fetch WiX or AppImage tooling", () => {
  assert.equal(conf.bundle.active, true);
  assert.deepEqual(conf.bundle.targets, []);
});

test("Windows installer is per-user and never downloads WebView2 at install time", () => {
  assert.deepEqual(conf.bundle.windows.webviewInstallMode, { type: "skip" });
  assert.equal(conf.bundle.windows.nsis.installMode, "currentUser");
});

test("product name changes do not move app data: identifier is unchanged", () => {
  assert.equal(conf.productName, "MedScale");
  assert.equal(conf.identifier, "org.medscale.desktop.preview");
});

test("packaging adds no updater, plugin or capability", () => {
  assert.equal(conf.plugins, undefined);
  assert.equal(conf.bundle.createUpdaterArtifacts, undefined);
  assert.deepEqual(conf.app.security.capabilities, ["default"]);
});

test("production CSP still allows no remote origin", () => {
  const csp: string = conf.app.security.csp;
  assert.doesNotMatch(csp, /https?:\/\/(?!ipc\.localhost)/);
  assert.match(csp, /connect-src ipc: http:\/\/ipc\.localhost;/);
});
