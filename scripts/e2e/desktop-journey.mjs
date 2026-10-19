// End-to-end user journeys against the real MedScale desktop binary.
//
// Drives the release build through Microsoft Edge WebDriver in WebView2
// attach mode with no npm dependencies: the script launches the app with a
// fixed WebView2 remote-debugging port, and Edge WebDriver attaches through
// `ms:edgeOptions.debuggerAddress` (Microsoft's documented WebView2
// automation path). Plain HTTP to the WebDriver endpoint. Synthetic data only; the app opens its own synthetic
// workspace from a fresh data directory. Each step saves a screenshot for
// visual review. Usage:
//   node scripts/e2e/desktop-journey.mjs <path-to-app.exe> <screenshot-dir>
import fs from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";

const [app, shots = "target/e2e-shots"] = process.argv.slice(2);
if (!app) throw new Error("usage: desktop-journey.mjs <app> [shots]");
const WD = process.env.WEBDRIVER_URL ?? "http://127.0.0.1:9515";
const DEBUG_PORT = Number(process.env.WEBVIEW2_DEBUG_PORT ?? 9222);
fs.mkdirSync(shots, { recursive: true });

async function wd(method, url, body) {
  const res = await fetch(WD + url, {
    method,
    headers: { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const json = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(`${method} ${url}: ${res.status} ${JSON.stringify(json).slice(0, 400)}`);
  return json.value;
}

const ELEMENT = "element-6066-11e4-a52e-4f735466cecc";
let sid;
let child;
let step = 0;
const results = [];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function launch() {
  child = spawn(path.resolve(app), [], {
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${DEBUG_PORT}` },
    stdio: "ignore",
  });
  const end = Date.now() + 60000;
  for (;;) {
    try {
      const r = await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/version`);
      if (r.ok) break;
    } catch {}
    if (Date.now() > end) throw new Error("the app opened no WebView2 debugging port");
    await sleep(500);
  }
  const s = await wd("POST", "/session", {
    capabilities: {
      alwaysMatch: { browserName: "webview2", "ms:edgeOptions": { debuggerAddress: `127.0.0.1:${DEBUG_PORT}` } },
    },
  });
  sid = s.sessionId;
}
async function quit() {
  if (sid) await wd("DELETE", `/session/${sid}`).catch(() => {});
  sid = undefined;
  if (child) {
    child.kill();
    await new Promise((r) => (child.exitCode !== null ? r() : child.once("exit", r)));
    child = undefined;
    await sleep(2000);
  }
}
async function find(xpath, timeout = 30000) {
  const end = Date.now() + timeout;
  let last;
  while (Date.now() < end) {
    try {
      const els = await wd("POST", `/session/${sid}/elements`, { using: "xpath", value: xpath });
      for (const e of els) {
        const id = e[ELEMENT];
        const shown = await wd("GET", `/session/${sid}/element/${id}/displayed`).catch(() => false);
        const enabled = await wd("GET", `/session/${sid}/element/${id}/enabled`).catch(() => false);
        if (shown && enabled) return id;
      }
    } catch (err) {
      last = err;
    }
    await sleep(250);
  }
  throw new Error(`not found within ${timeout} ms: ${xpath}${last ? ` (${last.message})` : ""}`);
}
const text = (t) => `contains(normalize-space(.), ${JSON.stringify(t)})`;
async function click(xpath, timeout) {
  const id = await find(xpath, timeout);
  await wd("POST", `/session/${sid}/element/${id}/click`, {});
}
async function type(xpath, value) {
  const id = await find(xpath);
  await wd("POST", `/session/${sid}/element/${id}/clear`, {}).catch(() => {});
  await wd("POST", `/session/${sid}/element/${id}/value`, { text: value });
}
async function see(t, timeout) {
  await find(`//*[${text(t)} and not(.//*[${text(t)}])]`, timeout);
}
async function nav(label) {
  await click(`//button[contains(@class,'nav') and ${text(label)}]`);
}
async function shot(name) {
  const png = await wd("GET", `/session/${sid}/screenshot`);
  fs.writeFileSync(path.join(shots, `${String(++step).padStart(2, "0")}-${name}.png`), Buffer.from(png, "base64"));
}
async function journey(name, fn) {
  const started = Date.now();
  try {
    await fn();
    await shot(name);
    results.push({ journey: name, ok: true, ms: Date.now() - started });
    console.log(`E2E PASS ${name}`);
  } catch (err) {
    await shot(`${name}-FAILED`).catch(() => {});
    results.push({ journey: name, ok: false, ms: Date.now() - started, error: String(err.message) });
    console.log(`E2E FAIL ${name}: ${err.message}`);
    throw err;
  }
}

try {
  await launch();
  await journey("launch-and-open-synthetic-workspace", async () => {
    await click(`//button[${text("Open synthetic workspace")}]`, 60000);
    await find(`//button[contains(@class,'nav') and ${text("Patients")}]`, 60000);
  });
  await journey("patients", async () => {
    await nav("Patients");
    await see("Ada");
  });
  await journey("create-project", async () => {
    await nav("Projects");
    await type(`//input[@id='pn']`, "E2E study");
    await click(`//button[@type='submit' and ${text("Create project")}]`);
    await see("E2E study");
  });
  await journey("medagent-agent-run-evidence", async () => {
    await nav("MedAgent");
    await click(`//button[${text("New agent")}]`);
    await type(`//input[@id='an']`, "E2E reviewer");
    await click(`//button[@type='submit' and ${text("Register")}]`);
    await see("E2E reviewer");
    await type(`//textarea[@id='ap']`, "summarize the bound sources");
    await click(`//button[@type='submit' and ${text("Create run")}]`);
    await click(`//button[normalize-space(.)='Start']`);
    await click(`//button[${text("Execute on local runtime")}]`, 60000);
    await click(`//button[normalize-space(.)='Complete']`, 60000);
    await see("completed");
    await see("Evidence");
  });
  await journey("models-catalog", async () => {
    await nav("Models");
    await click(`//button[@role='tab' and ${text("Catalog")}]`);
    await see("of 2266", 60000);
  });
  await journey("documents", async () => {
    await nav("Documents");
    await see("patient-golden.json");
  });
  await journey("export-with-disclosure", async () => {
    await nav("Exports");
    await type(`//input[@id='xp']`, "e2e synthetic interchange");
    await click(`//button[@type='submit' and ${text("Export FHIR R4")}]`);
    await see("Full conformance is not claimed", 60000);
  });
  await journey("audit-trail", async () => {
    await nav("Audit Trail");
    await see("e2e synthetic interchange");
  });
  await quit();
  // Relaunch the binary: the vault must bring the run back.
  await launch();
  await journey("restart-persistence", async () => {
    await click(`//button[${text("Open synthetic workspace")}]`, 60000);
    await nav("MedAgent");
    await see("summarize the bound sources", 60000);
    await see("completed");
  });
} finally {
  await quit();
  fs.writeFileSync(path.join(shots, "results.json"), JSON.stringify(results, null, 1));
  const failed = results.filter((r) => !r.ok).length;
  console.log(`E2E_RESULTS ${JSON.stringify({ journeys: results.length, failed })}`);
}
