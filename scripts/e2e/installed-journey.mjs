// Drives the exact installer a visitor would download, on a disposable Windows
// runner: install, first run, FHIR import, consented model download, real speech,
// restart, encrypted vault, uninstall and reinstall. The app is the normal release
// build with InPrivate on and no test feature; only an environment variable opens
// a debugging port, and input arrives through the browser's own input pipeline.
//
// Logs carry step names and fixed values only: never audio, transcript text,
// passphrases or recovery codes.
import { execFileSync, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const [installer, manifestPath, reportPath] = process.argv.slice(2);
const wavPath = process.env.MEDSCALE_E2E_REAL_WAV_PATH;
if (!installer || !manifestPath || !reportPath || !wavPath) throw new Error("usage: installed-journey.mjs <installer> <package-manifest> <report> with MEDSCALE_E2E_REAL_WAV_PATH");
if (process.env.GITHUB_ACTIONS !== "true" || process.env.RUNNER_ENVIRONMENT !== "github-hosted") throw new Error("disposable GitHub-hosted runner required");

const DEBUG_PORT = 39518;
const IDENTIFIER = "org.medscale.desktop.preview";
const appData = path.join(process.env.LOCALAPPDATA, IDENTIFIER);
const hostsFile = path.join(process.env.SystemRoot, "System32", "drivers", "etc", "hosts");
const originalHosts = fs.readFileSync(hostsFile);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const pwsh = (script) => execFileSync("pwsh", ["-NoProfile", "-Command", script], { encoding: "utf8" }).trim();

const report = { installer: path.basename(installer), steps: [], signing: "NOT_GRANTED" };
let child;
let cdp;
// Once real speech is on the page, a failure must not print the page.
let pageIsLoggable = true;

// ----- the app -----

class Cdp {
  constructor(socket) {
    this.socket = socket;
    this.next = 1;
    this.pending = new Map();
    socket.addEventListener("message", (event) => {
      const message = JSON.parse(event.data);
      const entry = this.pending.get(message.id);
      if (!entry) return;
      this.pending.delete(message.id);
      if (message.error) entry.reject(new Error(`CDP ${message.error.message}`));
      else entry.resolve(message.result);
    });
  }
  send(method, params = {}) {
    const id = this.next++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }
}

async function launch(exe) {
  child = spawn(exe, [], {
    stdio: "ignore",
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${DEBUG_PORT}` },
  });
  const end = Date.now() + 120000;
  let target;
  while (Date.now() < end && !target) {
    if (child.exitCode !== null) throw new Error(`the app exited with code ${child.exitCode}`);
    try {
      const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json();
      target = list.find((t) => t.type === "page" && t.webSocketDebuggerUrl);
    } catch {}
    if (!target) await sleep(500);
  }
  if (!target) throw new Error("the installed app opened no debugging port");
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", () => reject(new Error("debugging socket failed")), { once: true });
  });
  cdp = new Cdp(socket);
  await cdp.send("Runtime.enable");
  await cdp.send("Emulation.setFocusEmulationEnabled", { enabled: true });
}

async function evaluate(expression) {
  const { result, exceptionDetails } = await cdp.send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
  if (exceptionDetails) throw new Error("page script failed");
  return result.value;
}

const locate = (xpath) => `document.evaluate(${JSON.stringify(xpath)}, document, null, 9, null).singleNodeValue`;

/** Waits for a usable element and returns its centre, scrolled into view. */
async function find(xpath, timeout = 30000) {
  const end = Date.now() + timeout;
  while (Date.now() < end) {
    const point = await evaluate(`(() => {
      const e = ${locate(xpath)};
      if (!e || e.disabled) return null;
      e.scrollIntoView({ block: "center" });
      const r = e.getBoundingClientRect();
      return r.width > 0 && r.height > 0 ? { x: r.x + r.width / 2, y: r.y + r.height / 2 } : null;
    })()`).catch(() => null);
    if (point) return point;
    await sleep(250);
  }
  throw new Error(`not found: ${xpath}`);
}

async function click(xpath, timeout) {
  const { x, y } = await find(xpath, timeout);
  for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
    await cdp.send("Input.dispatchMouseEvent", { type, x, y, button: type === "mouseMoved" ? "none" : "left", clickCount: type === "mouseMoved" ? 0 : 1 });
  }
}

async function type(xpath, value) {
  await click(xpath);
  await evaluate(`${locate(xpath)}.select()`);
  await cdp.send("Input.insertText", { text: value });
}

const textIs = (t) => `normalize-space(.)=${JSON.stringify(t)}`;
const button = (t) => `//button[${textIs(t)}]`;
const nav = (label) => click(`//button[contains(@class,'nav') and ${textIs(label)}]`);
const pageText = () => evaluate("document.body.innerText");

async function waitForText(pattern, timeout = 60000) {
  const test = typeof pattern === "string" ? (s) => s.includes(pattern) : (s) => pattern.test(s);
  const end = Date.now() + timeout;
  while (Date.now() < end) {
    if (test(await pageText().catch(() => ""))) return;
    await sleep(250);
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
  try { cdp?.socket.close(); } catch {}
  cdp = undefined;
  // Let the WebView2 children release the profile and the debugging port.
  await sleep(4000);
}

// ----- the installer -----

const findInstall = () => pwsh(`
  $keys = 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*'
  $i = Get-ItemProperty $keys -ErrorAction SilentlyContinue | Where-Object DisplayName -eq 'MedScale' | Select-Object -First 1
  if ($i) { @{ location = $i.InstallLocation; uninstall = $i.UninstallString } | ConvertTo-Json -Compress }
  exit 0`);

function install() {
  const code = pwsh(`(Start-Process -FilePath '${installer}' -ArgumentList '/S' -Wait -PassThru).ExitCode`);
  if (code !== "0") throw new Error(`installer exited ${code}`);
  const found = findInstall();
  if (!found) throw new Error("the installer registered nothing");
  const { location, uninstall } = JSON.parse(found);
  const dir = (location || path.dirname(uninstall.replaceAll('"', ""))).replaceAll('"', "");
  const exe = fs.readdirSync(dir).find((f) => f.endsWith(".exe") && !f.toLowerCase().startsWith("uninstall"));
  if (!exe) throw new Error("no program was installed");
  return { exe: path.join(dir, exe), uninstall: uninstall.replaceAll('"', ""), dir };
}

async function uninstall(installed) {
  const code = pwsh(`(Start-Process -FilePath '${installed.uninstall}' -ArgumentList '/S' -Wait -PassThru).ExitCode`);
  if (code !== "0") throw new Error(`uninstaller exited ${code}`);
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
    if (pageIsLoggable && cdp) console.log(`PAGE: ${String(await pageText().catch(() => "")).slice(0, 1500)}`);
    throw error;
  }
}

async function checkAndApproveDownload() {
  await click(button("Check disk budget"));
  await waitForText("Enough local storage", 30000);
  const approve = "//input[@aria-label='Approve Whisper model download']";
  await find(approve);
  if (!(await evaluate(`${locate(approve)}.checked`))) await click(approve);
  await click(button("Download, verify and install"));
}

const PASSPHRASE = `disposable-runner-${process.pid}-${createHash("sha256").update(os.hostname()).digest("hex").slice(0, 12)}`;
const SAMPLE = button("Try MedScale with sample data");
const OPENED = `//button[contains(@class,'nav') and ${textIs("Patients")}]`;
const LOCK = `//button[contains(@class,'nav') and ${textIs("Lock workspace")}]`;
const VAULT_PROJECT = "Project kept across uninstall";
const wavName = path.basename(wavPath);
const source = `//section[.//*[contains(@class,'sec-t') and ${textIs("Sources")}]]//div[contains(@class,'lr') and .//span[contains(normalize-space(.), ${JSON.stringify(wavName)})]]`;

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
    // As a browser would mark a download. /S doesn't go through SmartScreen, so
    // this doesn't measure what SmartScreen shows a person.
    pwsh(`Set-Content -LiteralPath '${installer}' -Stream Zone.Identifier -Value "[ZoneTransfer]\`r\`nZoneId=3"`);
    if (findInstall()) throw new Error("already installed");
    if (fs.existsSync(appData)) throw new Error("app data already exists on this runner");
    installed = install();
    if (installed.dir.toLowerCase().includes("program files")) throw new Error("expected a per-user install");
  });
  await step("first-launch-needs-no-account", async () => {
    await launch(installed.exe);
    await find(SAMPLE, 60000);
    if (/sign in|log in|create an account/i.test(await pageText())) throw new Error("the first screen asks for an account");
  });
  await step("sample-workspace-opens", async () => {
    await click(SAMPLE);
    await find(OPENED, 180000);
    await nav("Patients");
    await waitForText(/subject/i, 30000);
  });
  await step("create-project", async () => {
    await nav("Projects");
    await type("//input[@id='pn']", "Installed build study");
    await click(`//button[@type='submit' and ${textIs("Create project")}]`);
    await waitForText("Installed build study", 30000);
  });
  await step("import-fhir", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "medscale-fhir-"));
    const file = path.join(dir, "installed-fictional-patient.json");
    const bundled = JSON.parse(fs.readFileSync(path.resolve("fixtures/synthetic/fhir/r4/presentation/patient-golden.json"), "utf8"));
    fs.writeFileSync(file, JSON.stringify({ ...bundled, id: "installed-fictional-patient" }));
    await nav("Documents");
    await click(`//button[contains(normalize-space(.),'Import synthetic FHIR')]`);
    await type("//input[@id='ip']", file);
    await click(`//button[@type='submit' and contains(normalize-space(.),'Ingest through Core')]`);
    await waitForText("ingested and promoted as", 60000);
  });
  await step("folder-admission-matches-this-build", async () => {
    await nav("Audio");
    await find(button("Check disk budget"), 30000);
    await sleep(1500);
    const present = await evaluate("!!document.getElementById('ap1')");
    report.folder_admission_offered = present;
    if (process.env.MEDSCALE_E2E_EXPECT_NO_FOLDER_ADMISSION === "1" && present) throw new Error("a release build offers folder admission");
  });
  await step("failed-download-says-so-and-recovers", async () => {
    fs.writeFileSync(hostsFile, Buffer.concat([originalHosts, Buffer.from("\r\n127.0.0.1 huggingface.co\r\n")]));
    pwsh("Clear-DnsClientCache");
    try {
      await checkAndApproveDownload();
      await waitForText(/Model download: (failed|error|cancelled)/i, 120000);
    } finally {
      fs.writeFileSync(hostsFile, originalHosts);
      pwsh("Clear-DnsClientCache");
    }
  });
  await step("kill-during-download-recovers", async () => {
    await checkAndApproveDownload();
    await waitForText(/Model download: downloading/, 60000);
    await sleep(1500);
    await kill();
    await launch(installed.exe);
    await click(SAMPLE, 60000);
    await find(OPENED, 180000);
    await nav("Audio");
    await find(button("Check disk budget"), 30000);
  });
  await step("consented-model-download", async () => {
    await checkAndApproveDownload();
    await waitForText(/Model download: ready\./, 420000);
    await waitForText("Local Whisper engine installed", 180000);
  });
  await step("real-speech-transcribed", async () => {
    pageIsLoggable = false;
    await type("//input[@id='wp1']", wavPath);
    await click("//form[.//input[@id='wp1']]//button[@type='submit']");
    await waitForText("Recording imported", 60000);
    await click(`${source}//button[${textIs("Transcribe")}]`, 60000);
    await waitForText("Transcription recorded", 240000);
    await find(`//section[.//*[contains(@class,'sec-t') and ${textIs("Transcript")}]]//div[contains(@class,'lr')]/span[3][string-length(normalize-space(.)) > 8][not(contains(., '(untranscribed)'))][not(contains(., '[fixture'))]`, 90000);
  });
  await step("close-reopen-keeps-transcript-and-model", async () => {
    await closeWindow();
    await launch(installed.exe);
    await click(SAMPLE, 60000);
    await find(OPENED, 180000);
    await nav("Audio");
    await waitForText("Local Whisper engine installed", 180000);
    await click(`${source}//button[${textIs("Open transcript")}]`, 30000);
    await waitForText("Latest transcript opened", 30000);
  });
  await step("encrypted-vault-create-and-close", async () => {
    pageIsLoggable = false;
    await click(LOCK);
    await waitForText("Create encrypted vault", 30000);
    await type("//input[@id='pp']", PASSPHRASE);
    await type("//input[@id='pp2']", PASSPHRASE);
    await click("//button[@type='submit' and not(@disabled)]");
    await waitForText("Save your recovery codes", 120000);
    await click(`//button[contains(normalize-space(.),'I saved them')]`);
    await find(OPENED, 120000);
    await nav("Projects");
    await type("//input[@id='pn']", VAULT_PROJECT);
    await click(`//button[@type='submit' and ${textIs("Create project")}]`);
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
  const access = `//button[contains(normalize-space(.),'Set up or unlock encrypted vault')]`;
  await click(access, 60000);
  await waitForText("Unlock workspace", 60000);
  await type("//input[@id='pp']", PASSPHRASE);
  await click("//button[@type='submit' and not(@disabled)]");
  await find(OPENED, 180000);
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
  report.result = failed ? "FAIL" : "PASS";
  fs.mkdirSync(path.dirname(reportPath), { recursive: true });
  fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
  console.log(`INSTALLED_JOURNEY ${report.result} ${report.steps.filter((s) => s.result === "PASS").length}/${report.steps.length}`);
}
process.exit(failed ? 1 : 0);
