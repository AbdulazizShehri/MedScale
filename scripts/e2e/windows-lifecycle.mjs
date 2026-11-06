// Installs, launches, upgrades and removes a real Windows installer on a disposable
// runner. The app is the normal release build with InPrivate on and no test feature,
// driven through Windows UI Automation (uia-driver.ps1).
//
//   clean:   fresh install, first launch, return to the sample, encrypted vault,
//            wrong passphrase, kill while open, damaged vault, uninstall, reinstall.
//   upgrade: a published engineering preview holding a sample and a vault, then this
//            build installed over it, then the same data opened and removed.
//
// A pass here is lifecycle evidence for one unsigned build. It qualifies no release.
// Logs and the report carry step names and fixed values only: never passphrases or
// recovery codes.
import { execFileSync, spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import readline from "node:readline";

const [phase, installer, reportPath, previousInstaller] = process.argv.slice(2);
if (!["clean", "upgrade"].includes(phase) || !installer || !reportPath || (phase === "upgrade" && !previousInstaller)) {
  throw new Error("usage: windows-lifecycle.mjs <clean|upgrade> <installer> <report> [previous installer]");
}
if (process.env.GITHUB_ACTIONS !== "true" || process.env.RUNNER_ENVIRONMENT !== "github-hosted") {
  throw new Error("disposable GitHub-hosted runner required: this script changes app data");
}

const IDENTIFIER = "org.medscale.desktop.preview";
const appData = path.join(process.env.LOCALAPPDATA, IDENTIFIER);
const shots = path.join(path.dirname(path.resolve(reportPath)), `${phase}-screens`);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const pwsh = (script) => execFileSync("pwsh", ["-NoProfile", "-Command", script], { encoding: "utf8" }).trim();
const sha256 = (file) => createHash("sha256").update(fs.readFileSync(file)).digest("hex");

const report = { phase, source_sha: process.env.GITHUB_SHA ?? null, signing_decision: "NOT_GRANTED", qualifies_a_release: false, steps: [] };
let child;
let driver;

// ----- the app -----

function startDriver() {
  const proc = spawn("powershell.exe", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", path.resolve("scripts/e2e/uia-driver.ps1")], { stdio: ["pipe", "pipe", "inherit"] });
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

const button = (name) => ({ kind: "Button", name });
const describe = (c) => (c.id ? `#${c.id}` : `${c.kind ?? "any"} "${c.name}"`);

async function exists(c) {
  const r = await driver.send({ op: "exists", ...c });
  return !!(r.ok && r.found && r.enabled);
}

async function find(c, timeout = 30000) {
  const end = Date.now() + timeout;
  let tries = 0;
  while (Date.now() < end) {
    if (await exists(c)) return;
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

async function pageText() {
  const r = await driver.send({ op: "text" });
  if (!r.ok) throw new Error(`could not read the window: ${r.error}`);
  return (r.names ?? []).join("\n");
}

async function waitForText(pattern, timeout = 60000) {
  const test = typeof pattern === "string" ? (s) => s.includes(pattern) : (s) => pattern.test(s);
  const end = Date.now() + timeout;
  while (Date.now() < end) {
    if (test(await pageText().catch(() => ""))) return;
    await sleep(500);
  }
  throw new Error(`text not seen: ${pattern}`);
}

async function waitForExit(timeout) {
  const end = Date.now() + timeout;
  while (child.exitCode === null && Date.now() < end) await sleep(250);
  if (child.exitCode === null) throw new Error("the app did not exit");
  await sleep(4000); // Let the WebView2 children release the profile.
}

async function closeWindow() {
  pwsh(`(Get-Process -Id ${child.pid}).CloseMainWindow() | Out-Null`);
  await waitForExit(180000);
}

async function kill() {
  pwsh(`Stop-Process -Id ${child.pid} -Force`);
  await waitForExit(30000);
}

/** The whole screen, at points where no passphrase or recovery code is showing. */
async function shot(name) {
  try {
    await driver.send({ op: "wake", pid: child.pid });
    await sleep(800);
    fs.mkdirSync(shots, { recursive: true });
    const file = path.join(shots, `${name}.png`).replaceAll("'", "''");
    pwsh(`Add-Type -AssemblyName System.Drawing,System.Windows.Forms
      $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
      $i = New-Object System.Drawing.Bitmap $b.Width, $b.Height
      $g = [System.Drawing.Graphics]::FromImage($i)
      $g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
      $i.Save('${file}'); $g.Dispose(); $i.Dispose()`);
  } catch (error) {
    console.log(`SHOT ${name} skipped: ${String(error.message).slice(0, 120)}`);
  }
}

// ----- the installer -----

const registered = () => pwsh(`
  $keys = 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*'
  @(Get-ItemProperty $keys -ErrorAction SilentlyContinue | Where-Object DisplayName -eq 'MedScale' | ForEach-Object { @{ location = $_.InstallLocation; uninstall = $_.UninstallString; version = $_.DisplayVersion } }) | ConvertTo-Json -Compress -AsArray`);

const entries = () => JSON.parse(registered() || "[]");

function markAsDownloaded(file) {
  // As a browser would mark a download. A direct, silent launch does not pass through
  // SmartScreen, so this says nothing about what a person sees there.
  pwsh(`Set-Content -LiteralPath '${file.replaceAll("'", "''")}' -Stream Zone.Identifier -Value "[ZoneTransfer]\`r\`nZoneId=3"`);
}

function signature(file) {
  return pwsh(`(Get-AuthenticodeSignature -LiteralPath '${file.replaceAll("'", "''")}').Status.ToString()`);
}

function install(file) {
  const run = spawnSync(file, ["/S"], { timeout: 300000, stdio: "ignore" });
  if (run.error) throw new Error(`installer: ${run.error.code ?? run.error.message}`);
  if (run.status !== 0) throw new Error(`installer exited ${run.status}`);
  const found = entries();
  if (found.length !== 1) throw new Error(`expected one registration, found ${found.length}`);
  const uninstaller = found[0].uninstall.replaceAll('"', "");
  const dir = (found[0].location || path.dirname(uninstaller)).replaceAll('"', "");
  const exe = fs.readdirSync(dir).find((f) => f.endsWith(".exe") && !f.toLowerCase().startsWith("uninstall"));
  if (!exe) throw new Error("no program was installed");
  if (dir.toLowerCase().includes("program files")) throw new Error("expected a per-user install");
  return { exe: path.join(dir, exe), uninstall: uninstaller, dir, version: found[0].version };
}

async function uninstall(installed) {
  const run = spawnSync(installed.uninstall, ["/S"], { timeout: 300000, stdio: "ignore" });
  if (run.error) throw new Error(`uninstaller: ${run.error.code ?? run.error.message}`);
  if (run.status !== 0) throw new Error(`uninstaller exited ${run.status}`);
  await sleep(6000); // NSIS uninstallers relaunch from a temp copy.
  if (entries().length) throw new Error("still registered after uninstall");
  if (fs.existsSync(installed.exe)) throw new Error("program left behind after uninstall");
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
    if (driver && child) await shot(`failed-${name}`);
    if (pageIsLoggable && driver) console.log(`PAGE: ${String(await pageText().catch(() => "")).replaceAll("\n", " | ").slice(0, 2500)}`);
    throw error;
  }
}

// Recovery codes can be on the page once a vault exists, so failures stop printing it.
let pageIsLoggable = true;
const PASSPHRASE = `disposable-runner-${process.pid}-${sha256(installer).slice(0, 12)}`;
const SAMPLE = button("Try MedScale with sample data");
const OPENED = button("Patients");
const LOCK = button("Lock workspace");
const VAULT_PROJECT = "Project kept across the lifecycle";

async function createVaultAndProject() {
  await type("pp", PASSPHRASE);
  await type("pp2", PASSPHRASE);
  await click(button("Create vault"));
  await waitForText("Save your recovery codes", 120000);
  await click(button("I saved them · continue"));
  await find(OPENED, 120000);
  await click(button("Projects"));
  await type("pn", VAULT_PROJECT);
  await click(button("Create project"));
  await waitForText(VAULT_PROJECT, 30000);
}

/** This build goes straight to unlock when a vault exists. */
async function unlockDirectly() {
  await waitForText("Unlock workspace", 60000);
  await type("pp", PASSPHRASE);
  await click(button("Unlock workspace"));
  await find(OPENED, 180000);
}

async function projectStillThere() {
  await click(button("Projects"));
  await waitForText(VAULT_PROJECT, 30000);
}

function recordInstaller(key, file) {
  report[key] = { sha256: sha256(file), authenticode: signature(file) };
  if (report[key].authenticode === "Valid") throw new Error("this installer is unexpectedly signed: check what was built");
}

async function clean() {
  let installed;
  await step("record-installer", () => recordInstaller("installer", installer));
  await step("clean-install-with-mark-of-the-web", () => {
    markAsDownloaded(installer);
    if (entries().length) throw new Error("already installed");
    if (fs.existsSync(appData)) throw new Error("app data already exists on this runner");
    installed = install(installer);
    report.installed_version = installed.version;
  });
  await step("first-launch-welcome-needs-no-account", async () => {
    await launch(installed.exe);
    await find(SAMPLE, 60000);
    await waitForText("Your local AI", 10000);
    if (/sign in to|log in|create an account/i.test(await pageText())) throw new Error("the first screen asks for an account");
    await shot("01-first-launch");
  });
  await step("sample-opens-on-an-explicit-click", async () => {
    await click(SAMPLE);
    await find(OPENED, 180000);
    await click(OPENED);
    await waitForText(/subject/i, 30000);
    await shot("02-sample-open");
  });
  await step("sidebar-group-folds-and-opens", async () => {
    const audit = button("Audit Trail");
    if (await exists(audit)) throw new Error("Governance should start folded");
    await click(button("Governance"));
    await find(audit, 10000);
    await click(button("Governance"));
    await sleep(500);
    if (await exists(audit)) throw new Error("Governance did not fold");
  });
  await step("redesigned-screens-open", async () => {
    await click(button("MedAgent"));
    await waitForText("New run", 30000);
    await shot("03-medagent");
    await click(button("Knowledge"));
    await waitForText("Build index", 30000);
    await shot("04-knowledge");
    await click(button("Settings"));
    await waitForText("Keyboard", 30000);
    await shot("05-settings");
  });
  await step("restart-words-a-return-without-opening", async () => {
    await closeWindow();
    await launch(installed.exe);
    await find(SAMPLE, 60000);
    await waitForText("Return to your", 10000);
    if (await exists(OPENED)) throw new Error("the sample opened by itself");
    await shot("06-return-to-sample");
  });
  await step("encrypted-vault-create", async () => {
    pageIsLoggable = false;
    await click(button("Create an encrypted workspace"));
    await waitForText("Create encrypted vault", 30000);
    await createVaultAndProject();
    await closeWindow();
    if (!findFile(appData, "meta.sealed")) throw new Error("closing left no seal");
    if (findFile(appData, "meta.work.sqlite3")) throw new Error("closing left the work database behind");
  });
  await step("existing-vault-goes-straight-to-unlock", async () => {
    await launch(installed.exe);
    await waitForText("Unlock workspace", 60000);
    await shot("07-unlock");
    if (await exists(OPENED)) throw new Error("the vault opened without a passphrase");
  });
  await step("wrong-passphrase-opens-nothing", async () => {
    await type("pp", `${PASSPHRASE}-wrong`);
    await click(button("Unlock workspace"));
    await sleep(8000);
    if (await exists(OPENED)) throw new Error("a wrong passphrase opened the vault");
    await find({ id: "pp" }, 30000);
  });
  await step("right-passphrase-opens-the-vault", async () => {
    await type("pp", PASSPHRASE);
    await click(button("Unlock workspace"));
    await find(OPENED, 180000);
    await projectStillThere();
  });
  await step("killed-while-open-recovers-on-next-unlock", async () => {
    await kill();
    await launch(installed.exe);
    await unlockDirectly();
    await projectStillThere();
    await closeWindow();
  });
  await step("damaged-vault-refuses-and-changes-nothing", async () => {
    const sealed = findFile(appData, "meta.sealed");
    if (!sealed) throw new Error("no seal to damage");
    const good = fs.readFileSync(sealed);
    const bad = Buffer.from(good);
    bad[bad.length - 1] ^= 0xff;
    fs.writeFileSync(sealed, bad);
    const damaged = sha256(sealed);
    await launch(installed.exe);
    await waitForText("Unlock workspace", 60000);
    await type("pp", PASSPHRASE);
    await click(button("Unlock workspace"));
    await sleep(15000);
    if (await exists(OPENED)) throw new Error("a damaged vault opened");
    await shot("08-damaged-vault");
    await closeWindow();
    if (sha256(sealed) !== damaged) throw new Error("the failed unlock changed the damaged file");
    fs.writeFileSync(sealed, good);
    await launch(installed.exe);
    await unlockDirectly();
    await projectStillThere();
    await closeWindow();
  });
  await step("uninstall-keeps-user-data", async () => {
    await uninstall(installed);
    if (!findFile(appData, "meta.sealed")) throw new Error("uninstall removed the encrypted vault");
    report.user_data_kept_after_uninstall = true;
  });
  await step("reinstall-opens-the-same-vault", async () => {
    installed = install(installer);
    await launch(installed.exe);
    await unlockDirectly();
    await projectStillThere();
    await closeWindow();
  });
  await step("final-uninstall", () => uninstall(installed));
}

async function upgrade() {
  let installed;
  await step("record-installers", () => {
    recordInstaller("previous_installer", previousInstaller);
    recordInstaller("installer", installer);
  });
  await step("install-the-published-preview", () => {
    markAsDownloaded(previousInstaller);
    if (entries().length) throw new Error("already installed");
    if (fs.existsSync(appData)) throw new Error("app data already exists on this runner");
    installed = install(previousInstaller);
    report.previous_version = installed.version;
    report.previous_exe_sha256 = sha256(installed.exe);
  });
  await step("preview-holds-a-sample-and-a-vault", async () => {
    await launch(installed.exe);
    await click(button("Open synthetic workspace"), 60000);
    await find(OPENED, 180000);
    pageIsLoggable = false;
    await click(LOCK);
    await waitForText("Create encrypted vault", 60000);
    await createVaultAndProject();
    await closeWindow();
    report.sealed_sha256_before = sha256(findFile(appData, "meta.sealed"));
    report.header_sha256_before = sha256(findFile(appData, "vault_header.json"));
  });
  await step("install-this-build-over-it", () => {
    markAsDownloaded(installer);
    installed = install(installer);
    report.installed_version = installed.version;
    report.installed_exe_sha256 = sha256(installed.exe);
    if (report.installed_exe_sha256 === report.previous_exe_sha256) throw new Error("the program was not replaced");
    if (sha256(findFile(appData, "meta.sealed")) !== report.sealed_sha256_before) throw new Error("the upgrade changed the sealed vault");
    if (sha256(findFile(appData, "vault_header.json")) !== report.header_sha256_before) throw new Error("the upgrade changed the vault header");
  });
  await step("upgraded-app-opens-the-same-vault", async () => {
    await launch(installed.exe);
    await waitForText("Unlock workspace", 60000);
    await shot("01-upgraded-unlock");
    if (await exists(OPENED)) throw new Error("the vault opened without a passphrase");
    await type("pp", PASSPHRASE);
    await click(button("Unlock workspace"));
    await find(OPENED, 180000);
    await projectStillThere();
    await shot("02-upgraded-project");
  });
  await step("upgraded-app-finds-the-sample", async () => {
    await click(LOCK);
    await waitForText("Unlock workspace", 60000);
    await click(button("Explore sample data instead"));
    await find(SAMPLE, 30000);
    await waitForText("Return to your", 10000);
    await shot("03-upgraded-return-to-sample");
    await click(SAMPLE);
    await find(OPENED, 180000);
    await click(OPENED);
    await waitForText(/subject/i, 30000);
    await closeWindow();
  });
  await step("uninstall-keeps-user-data", async () => {
    await uninstall(installed);
    if (!findFile(appData, "meta.sealed")) throw new Error("uninstall removed the encrypted vault");
    report.user_data_kept_after_uninstall = true;
  });
}

let failed = false;
try {
  await (phase === "clean" ? clean() : upgrade());
} catch {
  failed = true;
} finally {
  if (child && child.exitCode === null) child.kill();
  driver?.proc.kill();
  report.result = failed ? "FAIL" : "PASS";
  fs.mkdirSync(path.dirname(path.resolve(reportPath)), { recursive: true });
  fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
  console.log(`WINDOWS_LIFECYCLE ${phase} ${report.result} ${report.steps.filter((s) => s.result === "PASS").length}/${report.steps.length}`);
}
process.exit(failed ? 1 : 0);
