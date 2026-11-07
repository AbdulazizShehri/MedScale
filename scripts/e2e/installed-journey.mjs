// Drives the exact installer a visitor would download, on a disposable Windows
// runner: install, first run, FHIR import, consented model download, real speech,
// restart, encrypted vault, uninstall and reinstall. The app is the normal release
// build with InPrivate on and no test feature, unchanged in any way: it's driven
// through Windows UI Automation (uia-driver.ps1), which assistive technology uses.
//
// Logs carry step names and fixed values only: never audio, transcript text,
// passphrases or recovery codes.
import { execFileSync, spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import readline from "node:readline";

const [installer, manifestPath, reportPath] = process.argv.slice(2);
const wavPath = process.env.MEDSCALE_E2E_REAL_WAV_PATH;
const referencePath = process.env.MEDSCALE_E2E_REAL_WAV_REFERENCE;
if (!installer || !manifestPath || !reportPath || !wavPath || !referencePath) throw new Error("usage: installed-journey.mjs <installer> <package-manifest> <report> with MEDSCALE_E2E_REAL_WAV_PATH and MEDSCALE_E2E_REAL_WAV_REFERENCE");
if (process.env.GITHUB_ACTIONS !== "true" || process.env.RUNNER_ENVIRONMENT !== "github-hosted") throw new Error("disposable GitHub-hosted runner required");

const IDENTIFIER = "org.medscale.desktop.preview";
const appData = path.join(process.env.LOCALAPPDATA, IDENTIFIER);
const hostsFile = path.join(process.env.SystemRoot, "System32", "drivers", "etc", "hosts");
const originalHosts = fs.readFileSync(hostsFile);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const pwsh = (script) => execFileSync("pwsh", ["-NoProfile", "-Command", script], { encoding: "utf8" }).trim();

const report = { installer: path.basename(installer), steps: [], signing: "NOT_GRANTED", inprivate_by_launch: [] };
let child;
let driver;
// Once real speech is on the page, a failure must not print the page.
let pageIsLoggable = true;

// ----- the app -----

function startDriver() {
  const proc = spawn("powershell.exe", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", path.resolve("scripts/e2e/uia-driver.ps1")], { stdio: ["pipe", "pipe", "ignore"] });
  const lines = readline.createInterface({ input: proc.stdout });
  const waiting = [];
  lines.on("line", (line) => waiting.shift()?.(line));
  return {
    proc,
    send(command) {
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error(`driver timed out on ${command.op}`)), 120000);
        waiting.push((line) => { clearTimeout(timer); resolve(JSON.parse(line)); });
        proc.stdin.write(`${JSON.stringify(command)}\n`);
      });
    },
  };
}

async function launch(exe) {
  child = spawn(exe, [], { stdio: "ignore" });
  driver ??= startDriver();
  const end = Date.now() + 120000;
  while (Date.now() < end) {
    if (child.exitCode !== null) throw new Error(`the app exited with code ${child.exitCode}`);
    if ((await driver.send({ op: "attach", pid: child.pid })).found) return;
    await sleep(500);
  }
  throw new Error("the installed app opened no window");
}

const describe = (c) => c.id ? `#${c.id}` : `${c.kind ?? "any"} "${c.name}"`;
const control = (name) => ({ kind: "Button", name });

/** Waits for a control that can be used. */
async function find(c, timeout = 30000) {
  const end = Date.now() + timeout;
  let tries = 0;
  while (Date.now() < end) {
    const r = await driver.send({ op: "exists", ...c });
    if (r.ok && r.found && r.enabled) return;
    if (++tries % 15 === 0) await driver.send({ op: "wake", pid: child.pid });
    await sleep(300);
  }
  throw new Error(`not found: ${describe(c)}`);
}

async function click(c, timeout) {
  await find(c, timeout);
  const r = await driver.send({ op: "invoke", ...c });
  if (!r.ok) throw new Error(`could not press ${describe(c)}: ${r.error}`);
}

async function type(id, value) {
  await find({ id });
  const r = await driver.send({ op: "set", id, value });
  if (!r.ok) throw new Error(`could not fill #${id}: ${r.error}`);
}

const button = control;
const nav = (label) => click(control(label));
async function pageNames() {
  const r = await driver.send({ op: "text" });
  if (!r.ok) throw new Error(`could not read the window: ${r.error}`);
  return r.names ?? [];
}
const pageText = async () => (await pageNames()).join("\n");

// The accessibility tree splits adjacent text nodes, and pageText puts a line
// break between them, so a pattern spanning nodes needs \\s rather than a space.
async function waitForText(pattern, timeout = 60000) {
  const test = typeof pattern === "string" ? (s) => s.includes(pattern) : (s) => pattern.test(s);
  const end = Date.now() + timeout;
  while (Date.now() < end) {
    if (test(await pageText().catch(() => ""))) return;
    await sleep(500);
  }
  throw new Error(`text not seen: ${pattern}`);
}

/** Closes the window as a person would, and waits for the app to finish. */
async function closeWindow() {
  const { pid } = child;
  pwsh(`(Get-Process -Id ${pid}).CloseMainWindow() | Out-Null`);
  await waitForExit(180000);
}

async function kill() {
  pwsh(`Stop-Process -Id ${child.pid} -Force`);
  await waitForExit(30000);
}

async function waitForExit(timeout) {
  const end = Date.now() + timeout;
  while (child.exitCode === null && Date.now() < end) await sleep(250);
  if (child.exitCode === null) throw new Error("the app did not exit");
  // Let the WebView2 children release the profile.
  await sleep(4000);
}

// ----- the installer -----

const findInstall = () => pwsh(`
  $keys = 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*'
  $i = Get-ItemProperty $keys -ErrorAction SilentlyContinue | Where-Object DisplayName -eq 'MedScale' | Select-Object -First 1
  if ($i) { @{ location = $i.InstallLocation; uninstall = $i.UninstallString } | ConvertTo-Json -Compress }
  exit 0`);

function install() {
  // Launched directly: going through the shell, a marked file can stop at a
  // security prompt that nobody can answer here. Bounded either way.
  const run = spawnSync(installer, ["/S"], { timeout: 300000, stdio: "ignore" });
  if (run.error) throw new Error(`installer: ${run.error.code ?? run.error.message}`);
  if (run.status !== 0) throw new Error(`installer exited ${run.status}`);
  const found = findInstall();
  if (!found) throw new Error("the installer registered nothing");
  const { location, uninstall } = JSON.parse(found);
  const dir = (location || path.dirname(uninstall.replaceAll('"', ""))).replaceAll('"', "");
  const exe = fs.readdirSync(dir).find((f) => f.endsWith(".exe") && !f.toLowerCase().startsWith("uninstall"));
  if (!exe) throw new Error("no program was installed");
  return { exe: path.join(dir, exe), uninstall: uninstall.replaceAll('"', ""), dir };
}

async function uninstall(installed) {
  const run = spawnSync(installed.uninstall, ["/S"], { timeout: 300000, stdio: "ignore" });
  if (run.error) throw new Error(`uninstaller: ${run.error.code ?? run.error.message}`);
  if (run.status !== 0) throw new Error(`uninstaller exited ${run.status}`);
  await sleep(6000); // NSIS uninstallers relaunch from a temp copy.
  if (findInstall()) throw new Error("still registered after uninstall");
  if (fs.existsSync(installed.exe)) throw new Error("program left behind after uninstall");
}

// ----- steps -----

async function step(name, fn) {
  const started = Date.now();
  try {
    await fn();
    report.steps.push({ name, result: "PASS", seconds: Math.round((Date.now() - started) / 1000) });
    console.log(`STEP ${name} PASS`);
  } catch (error) {
    report.steps.push({ name, result: "FAIL", seconds: Math.round((Date.now() - started) / 1000) });
    console.log(`STEP ${name} FAIL: ${String(error.message).slice(0, 300)}`);
    if (driver) console.log(`SHAPE: ${JSON.stringify((await driver.send({ op: "shape" }).catch(() => ({}))).shape ?? {})}`);
    if (pageIsLoggable && driver) console.log(`PAGE: ${String(await pageText().catch(() => "")).replaceAll("\n", " | ").slice(0, 2500)}`);
    throw error;
  }
}

async function checkAndApproveDownload() {
  await click(button("Check disk budget"));
  await waitForText("Enough local storage", 30000);
  const approve = { name: "Approve Whisper model download", kind: "CheckBox" };
  await find(approve);
  if (!(await driver.send({ op: "checked", ...approve })).checked) await click(approve);
  await click(button("Download, verify and install"));
}

const PASSPHRASE = `disposable-runner-${process.pid}-${createHash("sha256").update(os.hostname()).digest("hex").slice(0, 12)}`;
const SAMPLE = button("Try MedScale with sample data");
const OPENED = control("Patients");
const LOCK = control("Lock workspace");
const VAULT_PROJECT = "Project kept across uninstall";

async function main() {
  let installed;
  await step("installer-digest-matches-manifest", async () => {
    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8").replace(/^﻿/, ""));
    const digest = createHash("sha256").update(fs.readFileSync(installer)).digest("hex");
    if (digest !== manifest.installer.sha256) throw new Error("digest mismatch");
    report.installer_sha256 = digest;
    report.source_sha = manifest.source_sha;
    report.version = manifest.version;
  });
  await step("install-with-mark-of-the-web", async () => {
    // As a browser would mark a download. This doesn't measure what SmartScreen
    // shows a person: a direct, silent launch doesn't pass through it.
    pwsh(`Set-Content -LiteralPath '${installer}' -Stream Zone.Identifier -Value "[ZoneTransfer]\`r\`nZoneId=3"`);
    if (findInstall()) throw new Error("already installed");
    if (fs.existsSync(appData)) throw new Error("app data already exists on this runner");
    installed = install();
    if (installed.dir.toLowerCase().includes("program files")) throw new Error("expected a per-user install");
  });
  await step("first-launch-needs-no-account", async () => {
    await launch(installed.exe);
    await find(SAMPLE, 60000);
    await recordInPrivate("first");
    if (/sign in|log in|create an account/i.test(await pageText())) throw new Error("the first screen asks for an account");
  });
  await step("sample-workspace-opens", async () => {
    await click(SAMPLE);
    await find(OPENED, 180000);
    await nav("Patients");
    await waitForText(/subject/i, 30000);
  });
  await step("inprivate-storage-probe", async () => {
    // InPrivate keeps no browser storage on disk. A saved preference is the
    // direct test: does it reach the profile, on the first launch and the next?
    const setTheme = async (name) => {
      await nav("Settings");
      await click({ kind: "RadioButton", name }, 30000);
      await sleep(1500);
    };
    await setTheme("Light");
    await closeWindow();
    report.pref_on_disk_after_first_launch = webViewResidue(["medscale.theme"]);
    await launch(installed.exe);
    await find(SAMPLE, 60000);
    await recordInPrivate("second");
    await click(SAMPLE, 60000);
    await find(OPENED, 180000);
    await setTheme("Dark");
    await closeWindow();
    report.pref_on_disk_after_second_launch = webViewResidue(["medscale.theme"]);
    await launch(installed.exe);
    await find(SAMPLE, 60000);
    await recordInPrivate("third");
    await click(SAMPLE, 60000);
    await find(OPENED, 180000);
    // The second launch chose Dark, and the first chose Light; compare with Light
    // here too, because Dark may simply be the default.
    await nav("Settings");
    await find({ kind: "RadioButton", name: "Light" }, 30000);
    report.theme_selected_after_restart = {
      dark: (await driver.send({ op: "selected", kind: "RadioButton", name: "Dark" })).selected ?? null,
      light: (await driver.send({ op: "selected", kind: "RadioButton", name: "Light" })).selected ?? null,
    };
    console.log(`THEME_AFTER_RESTART ${JSON.stringify(report.theme_selected_after_restart)}`);
    await click({ kind: "RadioButton", name: "Light" }, 30000);
    await sleep(1500);
    await closeWindow();
    await launch(installed.exe);
    await find(SAMPLE, 60000);
    await click(SAMPLE, 60000);
    await find(OPENED, 180000);
    await nav("Settings");
    await find({ kind: "RadioButton", name: "Light" }, 30000);
    report.light_kept_after_restart = (await driver.send({ op: "selected", kind: "RadioButton", name: "Light" })).selected ?? null;
    console.log(`LIGHT_KEPT_AFTER_RESTART ${report.light_kept_after_restart}`);
    console.log(`PREF_ON_DISK first=${report.pref_on_disk_after_first_launch.hits} second=${report.pref_on_disk_after_second_launch.hits}`);
  });
  await step("focusable-controls-are-named", async () => {
    // Web content takes focus as a whole, so the document itself is excluded.
    const report_ = {};
    for (const page of ["Home", "Patients", "Documents", "Projects", "Models"]) {
      await nav(page);
      await sleep(1500);
      const { unnamed = {} } = await driver.send({ op: "unnamed" });
      const { "ControlType.Document": _document, "ControlType.Pane": _pane, ...controls } = unnamed;
      report_[page] = controls;
    }
    report.unnamed_focusable_by_page = report_;
    console.log(`UNNAMED_FOCUSABLE ${JSON.stringify(report_)}`);
    const pagesWithGaps = Object.entries(report_).filter(([, c]) => Object.keys(c).length > 0).map(([p]) => p);
    if (pagesWithGaps.length) throw new Error(`unnamed focusable controls on ${pagesWithGaps.join(", ")}`);
  });
  await step("layout-measured-at-window-sizes", async () => {
    // Synthetic workspace only, so names are safe to print. Measured, not asserted yet.
    const result = {};
    for (const [width, height] of [[1194, 938], [1180, 720], [1440, 900]]) {
      const applied = await driver.send({ op: "resize", width, height });
      for (const page of ["Home", "Patients", "Documents", "Insights", "Audio", "Settings"]) {
        await nav(page);
        await sleep(1200);
        const { names = [] } = await driver.send({ op: "clipped" });
        if (names.length) result[`${width}x${height}:${page}`] = names.length;
        if (names.length) console.log(`CLIPPED ${applied.width ?? width}x${applied.height ?? height} ${page} ${names.length}: ${names.slice(0, 6).join(" | ").slice(0, 220)}`);
      }
    }
    report.clipped_by_size_and_page = result;
    console.log(`CLIPPED_TOTAL ${JSON.stringify(result)}`);
  });
  await step("create-project", async () => {
    await nav("Projects");
    await type("pn", "Installed build study");
    await click(button("Create project"));
    await waitForText("Installed build study", 30000);
  });
  await step("import-fhir", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "medscale-fhir-"));
    const file = path.join(dir, "installed-fictional-patient.json");
    const bundled = JSON.parse(fs.readFileSync(path.resolve("fixtures/synthetic/fhir/r4/presentation/patient-golden.json"), "utf8"));
    fs.writeFileSync(file, JSON.stringify({ ...bundled, id: "installed-fictional-patient" }));
    await nav("Documents");
    await click(button("Import synthetic FHIR"));
    await type("ip", file);
    await click(button("Ingest through Core"));
    await waitForText("ingested and promoted as", 60000);
  });
  await step("folder-admission-matches-this-build", async () => {
    await nav("Audio");
    await find(button("Check disk budget"), 30000);
    await sleep(1500);
    const present = (await driver.send({ op: "exists", id: "ap1" })).found === true;
    report.folder_admission_offered = present;
    if (process.env.MEDSCALE_E2E_EXPECT_NO_FOLDER_ADMISSION === "1" && present) throw new Error("a release build offers folder admission");
  });
  await step("failed-download-says-so-and-recovers", async () => {
    fs.writeFileSync(hostsFile, Buffer.concat([originalHosts, Buffer.from("\r\n127.0.0.1 huggingface.co\r\n")]));
    pwsh("Clear-DnsClientCache");
    try {
      await checkAndApproveDownload();
      await waitForText(/Model download:\s*(failed|error|cancelled)/i, 120000);
    } finally {
      fs.writeFileSync(hostsFile, originalHosts);
      pwsh("Clear-DnsClientCache");
    }
  });
  await step("kill-during-download-recovers", async () => {
    await checkAndApproveDownload();
    await waitForText(/Model download:\s*downloading/, 60000);
    await sleep(1500);
    await kill();
    await launch(installed.exe);
    await find(SAMPLE, 60000);
    await recordInPrivate("after-kill");
    await click(SAMPLE, 60000);
    await find(OPENED, 180000);
    await nav("Audio");
    await find(button("Check disk budget"), 30000);
  });
  await step("consented-model-download", async () => {
    await checkAndApproveDownload();
    await waitForText(/Model download:\s*ready/, 420000);
    await waitForText("Local Whisper engine installed", 180000);
    // The download really happened: the weights are on disk at their real size.
    // A killed download can leave a partial copy, which is reported separately.
    const copies = findFiles(path.join(appData, "audio-models"), "model.safetensors").map((f) => fs.statSync(f).size).sort((a, b) => b - a);
    if (copies.length === 0) throw new Error("no model weights on disk");
    report.model_weights_bytes = copies[0];
    report.leftover_partial_weights_bytes = copies.slice(1);
    if (report.model_weights_bytes < 50 * 1024 * 1024) throw new Error("the model weights are too small to be real");
  });
  await step("real-speech-transcribed", async () => {
    pageIsLoggable = false;
    await type("wp1", wavPath);
    await click(button("Import"));
    await waitForText("Recording imported", 60000);
    const before = new Set(await pageNames());
    await click(button("Transcribe"), 60000);
    await waitForText("Transcription recorded", 240000);
    await sleep(2000);
    // Real speech yields new sentences on the page. Counted, never printed.
    const after = await pageNames();
    if (after.some((n) => n.includes("(untranscribed)") || n.includes("[fixture"))) throw new Error("the transcript is a placeholder");
    const added = after.filter((n) => !before.has(n));
    // Against the public reference: a score goes in the report, never the words.
    const heard = new Set(words(added.join(" ")));
    const reference = words(fs.readFileSync(referencePath, "utf8"));
    const recall = reference.filter((w) => heard.has(w)).length / reference.length;
    report.reference_word_count = reference.length;
    report.reference_word_recall = Number(recall.toFixed(3));
    if (recall < 0.6) throw new Error(`transcript matches only ${report.reference_word_recall} of the reference words`);
  });
  await step("close-reopen-keeps-transcript-and-model", async () => {
    await closeWindow();
    await launch(installed.exe);
    await find(SAMPLE, 60000);
    await recordInPrivate("after-clean-close");
    await click(SAMPLE, 60000);
    await find(OPENED, 180000);
    await nav("Audio");
    await waitForText("Local Whisper engine installed", 180000);
    await click(button("Open transcript"), 30000);
    await waitForText("Latest transcript opened", 30000);
  });
  await step("encrypted-vault-create-and-close", async () => {
    pageIsLoggable = false;
    await click(LOCK);
    await waitForText("Create encrypted vault", 30000);
    await type("pp", PASSPHRASE);
    await type("pp2", PASSPHRASE);
    await click(button("Create vault"));
    await waitForText("Save your recovery codes", 120000);
    await click(button("I saved them · continue"));
    await find(OPENED, 120000);
    await nav("Projects");
    await type("pn", VAULT_PROJECT);
    await click(button("Create project"));
    await waitForText(VAULT_PROJECT, 30000);
    await closeWindow();
    if (!fs.existsSync(path.join(appData, "vault", "meta.sealed")) && !findFile(appData, "meta.sealed")) throw new Error("closing left no seal");
    if (findFile(appData, "meta.work.sqlite3")) throw new Error("closing left the work database behind");
  });
  await step("encrypted-vault-unlocks-after-restart", async () => {
    await launch(installed.exe);
    await unlock();
    await nav("Projects");
    await waitForText(VAULT_PROJECT, 30000);
    await closeWindow();
  });
  await step("webview-residue-measured", async () => {
    // Reported, not asserted yet: this journey is the first to look after real use.
    report.webview_residue = webViewResidue(["Installed build study", VAULT_PROJECT, "installed-fictional-patient"]);
    console.log(`WEBVIEW_RESIDUE ${JSON.stringify(report.webview_residue)}`);
  });
  await step("uninstall-keeps-user-data", async () => {
    await uninstall(installed);
    if (!findFile(appData, "meta.sealed")) throw new Error("uninstall removed the encrypted vault");
    report.user_data_kept_after_uninstall = true;
  });
  await step("reinstall-opens-the-same-vault", async () => {
    installed = install();
    await launch(installed.exe);
    await unlock();
    await nav("Projects");
    await waitForText(VAULT_PROJECT, 30000);
    await closeWindow();
  });
  await step("final-uninstall", () => uninstall(installed));
}

async function unlock() {
  // Older builds open on Welcome. Newer ones go straight to unlock when a vault exists.
  const older = button("Set up or unlock encrypted vault");
  await waitForText(/Unlock workspace|Set up or unlock encrypted vault/, 60000);
  if ((await driver.send({ op: "exists", ...older })).found) await click(older);
  await waitForText("Unlock workspace", 60000);
  await type("pp", PASSPHRASE);
  await click(button("Unlock workspace"));
  await find(OPENED, 180000);
}

const words = (text) => text.toLowerCase().match(/[a-z']+/g) ?? [];

function findFiles(root, name) {
  const found = [];
  for (const entry of fs.existsSync(root) ? fs.readdirSync(root, { withFileTypes: true }) : []) {
    const full = path.join(root, entry.name);
    if (entry.isDirectory()) found.push(...findFiles(full, name));
    else if (entry.name === name) found.push(full);
  }
  return found;
}

/** Whether this launch's WebView says it's in InPrivate mode. */
async function recordInPrivate(label) {
  const names = await pageNames();
  const inPrivate = names.some((n) => /InPrivate/.test(n));
  report.inprivate_by_launch.push({ launch: label, inprivate: inPrivate });
  console.log(`INPRIVATE ${label} ${inPrivate}`);
}

/** Files under the WebView profile holding this run's synthetic markers. */
function webViewResidue(markers) {
  const root = path.join(appData, "EBWebView");
  const needles = markers.flatMap((m) => [Buffer.from(m, "utf8"), Buffer.from(m, "utf16le")]);
  let scanned = 0;
  let hits = 0;
  let unreadable = 0;
  const walk = (dir) => {
    for (const entry of fs.existsSync(dir) ? fs.readdirSync(dir, { withFileTypes: true }) : []) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) { walk(full); continue; }
      try {
        const bytes = fs.readFileSync(full);
        scanned += 1;
        if (needles.some((n) => bytes.includes(n))) hits += 1;
      } catch { unreadable += 1; }
    }
  };
  walk(root);
  return { scanned, hits, unreadable };
}

function findFile(root, name) {
  for (const entry of fs.existsSync(root) ? fs.readdirSync(root, { withFileTypes: true }) : []) {
    const full = path.join(root, entry.name);
    if (entry.isDirectory() && entry.name !== "EBWebView") {
      const nested = findFile(full, name);
      if (nested) return nested;
    } else if (entry.name === name) return full;
  }
  return null;
}

let failed = false;
try {
  await main();
} catch {
  failed = true;
} finally {
  fs.writeFileSync(hostsFile, originalHosts);
  if (child && child.exitCode === null) child.kill();
  driver?.proc.kill();
  report.result = failed ? "FAIL" : "PASS";
  fs.mkdirSync(path.dirname(reportPath), { recursive: true });
  fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
  console.log(`INSTALLED_JOURNEY ${report.result} ${report.steps.filter((s) => s.result === "PASS").length}/${report.steps.length}`);
}
process.exit(failed ? 1 : 0);
