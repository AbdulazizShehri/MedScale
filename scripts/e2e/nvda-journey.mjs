// Runs real NVDA against the installed release build on a disposable Windows runner
// and reads what it says from NVDA's own speech log. Synthetic data only. This is
// an automated pass over named screens: it shows what NVDA announces, and it doesn't
// replace a person using the app with a screen reader.
import { execFileSync, spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import readline from "node:readline";

const [installer, outDir] = process.argv.slice(2);
if (!installer || !outDir) throw new Error("usage: nvda-journey.mjs <installer> <output folder>");
if (process.env.GITHUB_ACTIONS !== "true" || process.env.RUNNER_ENVIRONMENT !== "github-hosted") throw new Error("disposable GitHub-hosted runner required");

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const pwsh = (script) => execFileSync("pwsh", ["-NoProfile", "-Command", script], { encoding: "utf8" }).trim();
fs.mkdirSync(outDir, { recursive: true });
const logPath = path.resolve(outDir, "nvda.log");

// NVDA's first-run dialogs would otherwise take focus from the app.
const configDir = path.join(process.env.APPDATA, "nvda");
fs.mkdirSync(configDir, { recursive: true });
fs.writeFileSync(path.join(configDir, "nvda.ini"), [
  "schemaVersion = 11", "[general]", "showWelcomeDialogAtStartup = False",
  "[update]", "autoCheck = False", "allowUsageStats = False", "askedAllowUsageStats = True", "startupNotification = False", "",
].join("\r\n"));

const nvdaExe = ["C:\\Program Files (x86)\\NVDA\\nvda.exe", "C:\\Program Files\\NVDA\\nvda.exe"].find((p) => fs.existsSync(p));
if (!nvdaExe) throw new Error("NVDA isn't installed");

const run = spawnSync(installer, ["/S"], { timeout: 300000, stdio: "ignore" });
if (run.error || run.status !== 0) throw new Error("the MedScale installer failed");
const key = JSON.parse(pwsh(`Get-ItemProperty 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*' -ErrorAction SilentlyContinue | Where-Object DisplayName -eq 'MedScale' | Select-Object -First 1 InstallLocation,UninstallString | ConvertTo-Json -Compress`));
const dir = (key.InstallLocation || path.dirname(key.UninstallString.replaceAll('"', ""))).replaceAll('"', "");
const exe = path.join(dir, fs.readdirSync(dir).find((f) => f.endsWith(".exe") && !f.toLowerCase().startsWith("uninstall")));

// nvda.exe asks for UI access in its manifest, so it must start through the shell.
pwsh(`Start-Process -FilePath '${nvdaExe}' -ArgumentList '-m','-l','12','-f','${logPath}'`);
await sleep(15000);

const app = spawn(exe, [], { stdio: "ignore" });
const driverProc = spawn("powershell.exe", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", path.resolve("scripts/e2e/uia-driver.ps1")], { stdio: ["pipe", "pipe", "ignore"] });
const waiting = [];
readline.createInterface({ input: driverProc.stdout }).on("line", (line) => waiting.shift()?.(JSON.parse(line)));
const send = (command) => new Promise((resolve) => { waiting.push(resolve); driverProc.stdin.write(`${JSON.stringify(command)}\n`); });

const button = (name) => ({ kind: "Button", name });
async function find(c, timeout = 60000) {
  const end = Date.now() + timeout;
  while (Date.now() < end) {
    const r = await send({ op: "exists", ...c });
    if (r.ok && r.found && r.enabled) return;
    await sleep(400);
  }
  throw new Error(`not found: ${c.name ?? c.id}`);
}
async function press(name, timeout) {
  await find(button(name), timeout);
  const r = await send({ op: "invoke", ...button(name) });
  if (!r.ok) throw new Error(`could not press ${name}`);
}
/** Real keystrokes to the foreground window, which is what NVDA reacts to. */
function keys(sequence) {
  pwsh(`$w = New-Object -ComObject WScript.Shell; [void]$w.AppActivate(${app.pid}); Start-Sleep -Milliseconds 400; $w.SendKeys('${sequence}')`);
}
async function tabThrough(count) {
  for (let i = 0; i < count; i++) { keys("{TAB}"); await sleep(900); }
}
/** What NVDA has spoken since the given log offset. */
function spokenSince(offset) {
  const text = fs.existsSync(logPath) ? fs.readFileSync(logPath, "utf8").slice(offset) : "";
  const lines = [...text.matchAll(/Speaking \[(.*)\]/g)].map((m) => m[1].replace(/LangChangeCommand\s*\([^)]*\),?|CancellableSpeech\s*\([^)]*\),?|EndUtteranceCommand\s*\([^)]*\),?|CharacterModeCommand\s*\([^)]*\),?/g, "").replace(/['"]/g, "").trim());
  return lines.filter(Boolean);
}
const logSize = () => (fs.existsSync(logPath) ? fs.readFileSync(logPath, "utf8").length : 0);

const report = { nvda_started: false, screens: {}, checks: {} };
let failed = false;
try {
  for (let i = 0; i < 240 && !(await send({ op: "attach", pid: app.pid })).found; i++) await sleep(500);
  const welcomeButton = "Try MedScale with sample data";
  await find(button(welcomeButton));
  await sleep(1500);

  let mark = logSize();
  await tabThrough(4);
  report.screens.welcome = spokenSince(mark);

  await press(welcomeButton);
  await find(button("Patients"), 180000);
  await sleep(4000);

  for (const [page, tabs, expected] of [
    ["Home", 40, /Open patient /],
    ["Patients", 24, /Select patient /],
    ["Documents", 24, /Select source /],
    ["Audio", 14, null],
    ["Settings", 14, null],
  ]) {
    await press(page);
    await sleep(3500);
    mark = logSize();
    await tabThrough(tabs);
    const spoken = spokenSince(mark);
    report.screens[page] = spoken;
    if (expected) report.checks[`${page}: rows announced by name`] = spoken.some((s) => expected.test(s));
  }
  report.nvda_started = fs.existsSync(logPath) && logSize() > 0;
  report.checks["NVDA spoke at all"] = Object.values(report.screens).some((s) => s.length > 0);
  report.checks["welcome action announced"] = report.screens.welcome.some((s) => s.includes(welcomeButton));
  const blanks = Object.values(report.screens).flat().filter((s) => /^(row|table row|blank)\b/i.test(s));
  report.blank_rows_announced = blanks.length;
  report.checks["no row announced without a name"] = blanks.length === 0;
  for (const [name, ok] of Object.entries(report.checks)) { console.log(`NVDA_CHECK ${ok ? "PASS" : "FAIL"} ${name}`); if (!ok) failed = true; }
} catch (error) {
  failed = true;
  console.log(`NVDA_ERROR ${String(error.message).slice(0, 300)}`);
} finally {
  fs.writeFileSync(path.join(outDir, "nvda-report.json"), JSON.stringify(report, null, 2));
  for (const [screen, lines] of Object.entries(report.screens)) console.log(`NVDA_SPOKE ${screen} ${lines.length}: ${lines.slice(0, 8).join(" | ").slice(0, 500)}`);
  console.log(`NVDA_RESULT ${failed ? "FAIL" : "PASS"} log_bytes=${logSize()}`);
  driverProc.kill();
  if (app.exitCode === null) app.kill();
  try { pwsh(`Start-Process -FilePath '${nvdaExe}' -ArgumentList '-q'`); } catch {}
}
process.exit(failed ? 1 : 0);
