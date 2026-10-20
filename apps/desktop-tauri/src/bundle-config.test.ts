// Spec 101 T101-01: the bundle configuration stays inside its authorized scope.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";

const conf = JSON.parse(readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));

test("bundling is explicit: no implicit 'all' targets that would fetch WiX or AppImage tooling", () => {
  assert.equal(conf.bundle.active, true);
  assert.deepEqual(conf.bundle.targets, []);
});

test("Windows installer is per-user and never downloads WebView2 at install time", () => {
  assert.deepEqual(conf.bundle.windows.webviewInstallMode, { type: "skip" });
  assert.equal(conf.bundle.windows.nsis.installMode, "currentUser");
});

test("the Windows installer carries the Logo 12 application icon, not the NSIS default", () => {
  assert.equal(conf.bundle.windows.nsis.installerIcon, "icons/icon.ico");
  assert.ok(conf.bundle.icon.includes("icons/icon.ico"));
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

test("the test-only e2e-webdriver build is never used for packaging or release", () => {
  // That feature opens a WebView2 debugging port and turns off InPrivate.
  // Only the desktop E2E workflow may name it.
  const root = new URL("../../../", import.meta.url);
  const workflows = new URL(".github/workflows/", root);
  const scripts = new URL("scripts/", root);
  const offenders: string[] = [];
  for (const [dir, files] of [
    [workflows, readdirSync(workflows)],
    [scripts, readdirSync(scripts).filter((f) => /\.(ps1|sh|mjs|js)$/.test(f))],
  ] as const) {
    for (const file of files) {
      if (file === "desktop-e2e.yml") continue;
      if (readFileSync(new URL(file, dir), "utf8").includes("e2e-webdriver")) offenders.push(file);
    }
  }
  assert.deepEqual(offenders, []);
  assert.doesNotMatch(JSON.stringify(conf), /e2e-webdriver/);
  const cargo = readFileSync(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
  // Not a default feature: only an explicit `--features e2e-webdriver` turns it on.
  assert.doesNotMatch(cargo.match(/^default\s*=\s*\[[^\]]*\]/m)?.[0] ?? "", /e2e-webdriver/);
});
