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

test("packaging keeps updater and config-managed plugins absent", () => {
  assert.equal(conf.plugins, undefined);
  assert.equal(conf.bundle.createUpdaterArtifacts, undefined);
  assert.deepEqual(conf.app.security.capabilities, ["default"]);
});

test("native Audio picker grants OS open only, never broad file access", () => {
  const capability = JSON.parse(readFileSync(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"));
  assert.ok(capability.permissions.includes("dialog:allow-open"));
  assert.deepEqual(capability.permissions.filter((name: string) => name.startsWith("dialog:")), ["dialog:allow-open"]);
  assert.ok(!capability.permissions.some((name: string) => /^(?:fs|shell|http):/.test(name)));
  const cargo = readFileSync(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
  const native = readFileSync(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
  assert.match(cargo, /tauri-plugin-dialog = "=2\.6\.0"/);
  assert.match(native, /\.plugin\(tauri_plugin_dialog::init\(\)\)/);
});

test("production CSP still allows no remote origin", () => {
  const csp: string = conf.app.security.csp;
  assert.doesNotMatch(csp, /https?:\/\/(?!ipc\.localhost)/);
  assert.match(csp, /connect-src ipc: http:\/\/ipc\.localhost;/);
});

test("the test-only e2e-webdriver build is never used for packaging or release", () => {
  // That feature opens a WebView2 debugging port and turns off InPrivate.
  // Only dedicated, isolated Windows desktop E2E workflows may name it.
  // Their feature-gated binaries must never enter installer/release workflows.
  const root = new URL("../../../", import.meta.url);
  const workflows = new URL(".github/workflows/", root);
  const scripts = new URL("scripts/", root);
  const realVoiceE2e = readFileSync(new URL("audio-real-desktop-e2e.yml", workflows), "utf8");
  assert.match(realVoiceE2e, /name: audio-real-desktop-e2e/);
  assert.match(realVoiceE2e, /runs-on: windows-latest/);
  assert.match(realVoiceE2e, /feature\/audio-real-ui-e2e-/);
  assert.match(realVoiceE2e, /--features custom-protocol,e2e-webdriver/);
  assert.doesNotMatch(realVoiceE2e, /^  pull_request:/m);
  assert.doesNotMatch(realVoiceE2e, /\b(?:tauri\s+build|cargo\s+tauri\s+build|gh\s+release\s+create)\b/);
  const offenders: string[] = [];
  for (const [dir, files] of [
    [workflows, readdirSync(workflows)],
    [scripts, readdirSync(scripts).filter((f) => /\.(ps1|sh|mjs|js)$/.test(f))],
  ] as const) {
    for (const file of files) {
      if (file === "desktop-e2e.yml" || file === "audio-real-desktop-e2e.yml") continue;
      if (readFileSync(new URL(file, dir), "utf8").includes("e2e-webdriver")) offenders.push(file);
    }
  }
  assert.deepEqual(offenders, []);
  assert.doesNotMatch(JSON.stringify(conf), /e2e-webdriver/);
  const guard = readFileSync(new URL("../src-tauri/build.rs", import.meta.url), "utf8");
  const desktopE2e = readFileSync(new URL("desktop-e2e.yml", workflows), "utf8");
  // Accidental release builds must not activate the exposed WebView2 debug port:
  // the cargo build script requires both a specific opt-in and a Windows Actions runner.
  assert.match(guard, /CARGO_FEATURE_E2E_WEBDRIVER/);
  assert.match(guard, /MEDSCALE_E2E_WEBDRIVER_CI/);
  assert.match(guard, /GITHUB_ACTIONS/);
  assert.match(guard, /RUNNER_OS/);
  assert.match(desktopE2e, /MEDSCALE_E2E_WEBDRIVER_CI: "1"/);
  assert.match(realVoiceE2e, /MEDSCALE_E2E_WEBDRIVER_CI: "1"/);
  for (const file of readdirSync(workflows)) {
    if (file === "desktop-e2e.yml" || file === "audio-real-desktop-e2e.yml") continue;
    assert.doesNotMatch(
      readFileSync(new URL(file, workflows), "utf8"),
      /MEDSCALE_E2E_WEBDRIVER_CI/,
      "unrelated workflow " + file + " must never opt in to WebView2 debug builds",
    );
  }
  const cargo = readFileSync(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
  // Not a default feature: only an explicit `--features e2e-webdriver` turns it on.
  assert.doesNotMatch(cargo.match(/^default\s*=\s*\[[^\]]*\]/m)?.[0] ?? "", /e2e-webdriver/);
});
