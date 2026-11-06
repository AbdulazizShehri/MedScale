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
import os from "node:os";
import path from "node:path";
import { execFileSync, spawn } from "node:child_process";
import { auditProfiles, auditProfilesWithTransientRetry, hasObservedProfileBytes, hasCompleteAuditCoverage } from "./webview-disk-audit.mjs";

const [app, shots = "target/e2e-shots"] = process.argv.slice(2);
if (!app) throw new Error("usage: desktop-journey.mjs <app> [shots]");
const WD = process.env.WEBDRIVER_URL ?? "http://127.0.0.1:9515";
const DEBUG_PORT = Number(process.env.WEBVIEW2_DEBUG_PORT ?? 39517);
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

const ELEMENT = "element-6066-11e4-a52e-4f735466cecf";
let sid;
let child;
let step = 0;
const results = [];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function launch(extraEnv = {}) {
  child = spawn(path.resolve(app), [], {
    env: { ...process.env, ...extraEnv, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${DEBUG_PORT}` },
    stdio: "ignore",
  });
  // A fresh runner profile can take minutes to start WebView2 the first time.
  const started = Date.now();
  const end = started + 180000;
  let lastLog = 0;
  for (;;) {
    if (Date.now() - lastLog > 15000) {
      lastLog = Date.now();
      let webviews = "?";
      try {
        webviews = execFileSync("tasklist", ["/FI", "IMAGENAME eq msedgewebview2.exe", "/NH"], { encoding: "utf8" })
          .split(String.fromCharCode(10)).filter((l) => l.includes("msedgewebview2")).length;
      } catch {}
      console.log(`waiting for WebView2: ${Math.round((Date.now() - started) / 1000)} s, app ${child.exitCode === null ? "running" : `exited ${child.exitCode}`}, msedgewebview2 processes: ${webviews}`);
    }
    try {
      const r = await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/version`);
      if (r.ok) break;
    } catch {}
    if (Date.now() > end) {
      try {
        const ps = [
          "Get-CimInstance Win32_Process -Filter \"Name='msedgewebview2.exe'\" | Where-Object { $_.CommandLine -match 'medscale-desktop-tauri' } | Select-Object -First 2 ProcessId, CommandLine | Format-List | Out-String -Width 400",
          "Get-NetTCPConnection -State Listen | Where-Object { (Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue).ProcessName -match 'msedgewebview2|medscale' } | Format-Table LocalAddress, LocalPort, OwningProcess | Out-String",
        ].join("; ");
        console.log(execFileSync("pwsh", ["-NoProfile", "-Command", ps], { encoding: "utf8" }));
      } catch (err) {
        console.log(`diagnostics failed: ${err.message}`);
      }
      const state = child.exitCode === null ? "still running" : `exited with code ${child.exitCode}`;
      throw new Error(`the app opened no WebView2 debugging port (app ${state})`);
    }
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
async function visible(id) {
  // Layout-based check: WebDriver's "displayed" reports false for this
  // WebView2 window on the CI desktop even when it is rendered.
  return wd("POST", `/session/${sid}/execute/sync`, {
    script: "const e = arguments[0]; const r = e.getBoundingClientRect(); const s = getComputedStyle(e); return !e.disabled && r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none';",
    args: [{ [ELEMENT]: id }],
  }).catch(() => false);
}
let loggedFindError = false;
async function find(xpath, timeout = 30000) {
  const end = Date.now() + timeout;
  let last;
  while (Date.now() < end) {
    try {
      const els = await wd("POST", `/session/${sid}/elements`, { using: "xpath", value: xpath });
      for (const e of els) {
        const id = e[ELEMENT];
        if (id && (await visible(id))) return id;
      }
    } catch (err) {
      last = err;
      if (!loggedFindError) {
        loggedFindError = true;
        console.log(`find error: ${err.message}`);
      }
    }
    await sleep(250);
  }
  try {
    const els = await wd("POST", `/session/${sid}/elements`, { using: "xpath", value: xpath });
    let probe = "none";
    if (els[0]) {
      probe = await wd("POST", `/session/${sid}/execute/sync`, {
        script: "const e = arguments[0]; const r = e.getBoundingClientRect(); return JSON.stringify({ w: r.width, h: r.height, disabled: e.disabled, vis: getComputedStyle(e).visibility });",
        args: [{ [ELEMENT]: els[0][ELEMENT] }],
      }).catch((e) => `error ${e.message}`);
    }
    console.log(`DEBUG find matches=${els.length} keys=${JSON.stringify(Object.keys(els[0] ?? {}))} probe=${probe}`);
  } catch (e) {
    console.log(`DEBUG find probe failed: ${e.message}`);
  }
  throw new Error(`not found within ${timeout} ms: ${xpath}${last ? ` (${last.message})` : ""}`);
}
const text = (t) => `contains(normalize-space(.), ${JSON.stringify(t)})`;
// Unlike find(), disabledGuard deliberately accepts a disabled DOM element.
// WebDriver must not click a model-download control during synthetic E2E.
async function assertDisabled(xpath) {
  const disabled = await wd("POST", `/session/${sid}/execute/sync`, {
    script: "const e=document.evaluate(arguments[0],document,null,XPathResult.FIRST_ORDERED_NODE_TYPE,null).singleNodeValue; const r=e?.getBoundingClientRect(); return !!e && e.disabled === true && r.width > 0 && r.height > 0;",
    args: [xpath],
  });
  if (!disabled) throw new Error(`missing or enabled model-download control: ${xpath}`);
}
// Native WebDriver input first; if WebView2 reports the element as not
// interactable on the CI desktop, fall back to DOM events on the same element.
async function click(xpath, timeout) {
  const id = await find(xpath, timeout);
  try {
    await wd("POST", `/session/${sid}/element/${id}/click`, {});
  } catch {
    await wd("POST", `/session/${sid}/execute/sync`, { script: "arguments[0].click();", args: [{ [ELEMENT]: id }] });
  }
}
async function type(xpath, value) {
  const id = await find(xpath);
  try {
    await wd("POST", `/session/${sid}/element/${id}/clear`, {});
    await wd("POST", `/session/${sid}/element/${id}/value`, { text: value });
  } catch {
    // React tracks the value setter; use the prototype setter, then notify.
    await wd("POST", `/session/${sid}/execute/sync`, {
      script:
        "const e = arguments[0]; const proto = e.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;" +
        " Object.getOwnPropertyDescriptor(proto, 'value').set.call(e, arguments[1]); e.dispatchEvent(new Event('input', { bubbles: true }));",
      args: [{ [ELEMENT]: id }, value],
    });
  }
  // Under runner load the next click can land before the field holds the
  // text (a stale value then runs instead). Wait until it does; if native
  // typing left another value, set it through React's setter once more.
  const read = () => wd("POST", `/session/${sid}/execute/sync`, { script: "return arguments[0].value;", args: [{ [ELEMENT]: id }] });
  const end = Date.now() + 5000;
  let forced = false;
  while ((await read()) !== value) {
    if (Date.now() > end) throw new Error(`field ${xpath} did not take the typed value`);
    if (!forced) {
      forced = true;
      await wd("POST", `/session/${sid}/execute/sync`, {
        script:
          "const e = arguments[0]; const proto = e.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;" +
          " Object.getOwnPropertyDescriptor(proto, 'value').set.call(e, arguments[1]); e.dispatchEvent(new Event('input', { bubbles: true }));",
        args: [{ [ELEMENT]: id }, value],
      });
    }
    await sleep(100);
  }
  await sleep(300);
}
async function see(t, timeout) {
  await find(`//*[${text(t)} and not(.//*[${text(t)}])]`, timeout);
}
async function nav(label) {
  await click(`//button[contains(@class,'nav') and ${text(label)}]`);
}
async function pageText() {
  return wd("POST", `/session/${sid}/execute/sync`, { script: "return document.body.innerText;", args: [] });
}

/**
 * Synthetic-only WebView DOM storage canary. This is a narrow regression
 * assertion: it does not qualify WebView2 caches, crash dumps, logs, IndexedDB,
 * OS snapshots or real patient data. Never print stored keys or values.
 */
async function auditClientStorage(label) {
  const audit = await wd("POST", `/session/${sid}/execute/sync`, {
    script: `
      const riskyKey = /passphrase|password|credential|api[_-]?key|bearer|patient|subject|clinical|secret/i;
      const syntheticMarker = /synthetic-subject-|patient-golden\\.json|e2e synthetic interchange/i;
      function inspect(store) {
        let offending = 0;
        for (let i = 0; i < store.length; i++) {
          const k = store.key(i) ?? '';
          const v = store.getItem(k) ?? '';
          if (riskyKey.test(k) || syntheticMarker.test(v)) offending++;
        }
        return { entries: store.length, risky_entries: offending };
      }
      const local = inspect(window.localStorage);
      const session = inspect(window.sessionStorage);
      const cookies = document.cookie || '';
      return { local, session, project_pref_absent: window.localStorage.getItem("medscale.project") === null,
        cookie_risky: riskyKey.test(cookies) || syntheticMarker.test(cookies) };
    `,
    args: [],
  });
  // Query only aggregate metadata for origin IndexedDB and CacheStorage.
  // The E2E WebView disables InPrivate for WebDriver; this is not a
  // production WebView disk, logs, crash or snapshot privacy qualification.
  const originStores = await wd("POST", `/session/${sid}/execute/async`, {
    script: `
      const callback = arguments[arguments.length - 1];
      (async () => {
        if (typeof indexedDB === 'undefined' ||
            typeof indexedDB.databases !== 'function' ||
            typeof caches === 'undefined' ||
            typeof caches.keys !== 'function') {
          return { inspection_available: false, indexed_db_count: null, cache_storage_count: null };
        }
        // Do not open DBs, fetch caches or inspect private names/values.
        const [databases, cacheNames] = await Promise.all([
          indexedDB.databases(), caches.keys(),
        ]);
        return {
          inspection_available: true,
          indexed_db_count: databases.length,
          cache_storage_count: cacheNames.length,
        };
      })().then(callback, () => callback({
        inspection_available: false, indexed_db_count: null, cache_storage_count: null,
      }));
    `,
    args: [],
  });
  // Persist aggregate counts and booleans only, never keys, names or content.
  fs.writeFileSync(path.join(shots, `storage-audit-${label}.json`), JSON.stringify({
    scope: "synthetic-only origin DOM storage plus IndexedDB/CacheStorage COUNTS; not WebView disk/privacy qualification",
    ...audit,
    origin_stores: originStores,
  }, null, 2));
  if (!audit.project_pref_absent) {
    throw new Error(`WebView last-project selection was persisted in localStorage (${label})`);
  }
  if (audit.local.risky_entries || audit.session.risky_entries || audit.cookie_risky) {
    throw new Error(`WebView client-storage canary detected a sensitive key or synthetic patient marker (${label})`);
  }
  if (!originStores?.inspection_available ||
      !Number.isInteger(originStores.indexed_db_count) ||
      !Number.isInteger(originStores.cache_storage_count)) {
    throw new Error(`WebView IndexedDB/CacheStorage origin inspection unavailable (${label})`);
  }
  if (originStores.indexed_db_count !== 0 || originStores.cache_storage_count !== 0) {
    throw new Error(`WebView unexpectedly stored synthetic journey data in IndexedDB or CacheStorage (${label})`);
  }
}

async function waitForText(pattern, timeout = 60000) {
  const end = Date.now() + timeout;
  while (Date.now() < end) {
    const m = (await pageText()).match(pattern);
    if (m) return m[0];
    await sleep(500);
  }
  throw new Error(`text ${pattern} not shown within ${timeout} ms`);
}
async function shot(name) {
  const png = await wd("GET", `/session/${sid}/screenshot`);
  fs.writeFileSync(path.join(shots, `${String(++step).padStart(2, "0")}-${name}.png`), Buffer.from(png, "base64"));
}
async function journey(name, fn, { screenshot = true } = {}) {
  const started = Date.now();
  try {
    await fn();
    if (screenshot) await shot(name);
    results.push({ journey: name, ok: true, ms: Date.now() - started });
    console.log(`E2E PASS ${name}`);
  } catch (err) {
    if (screenshot) await shot(`${name}-FAILED`).catch(() => {});
    try {
      const handles = await wd("GET", `/session/${sid}/window/handles`);
      const title = await wd("GET", `/session/${sid}/title`);
      const url = await wd("GET", `/session/${sid}/url`);
      const buttons = await wd("POST", `/session/${sid}/elements`, { using: "xpath", value: "//button" });
      const js = await wd("POST", `/session/${sid}/execute/sync`, {
        script: "return Array.from(document.querySelectorAll('button')).slice(0, 8).map(b => b.innerText.trim());",
        args: [],
      });
      console.log(`DEBUG handles=${JSON.stringify(handles)} title=${title} url=${url} xpathButtons=${buttons.length} jsButtons=${JSON.stringify(js)}`);
    } catch (e) {
      console.log(`DEBUG failed: ${e.message}`);
    }
    results.push({ journey: name, ok: false, ms: Date.now() - started, error: String(err.message) });
    console.log(`E2E FAIL ${name}: ${err.message}`);
    throw err;
  }
}

try {
  await launch();
  if (process.env.MEDSCALE_CI_WEBVIEW_BASELINE === "1") {
    // Synthetic disposable Windows CI runner only; never scan a user workstation.
    // Run a pristine welcome-screen boot with no workspace opened, close the
    // WebView2 process, and take an aggregate baseline *before* any journey.
    if (process.env.GITHUB_ACTIONS !== "true" || process.platform !== "win32" ||
        !process.env.RUNNER_TEMP || !process.env.LOCALAPPDATA) {
      throw new Error("WebView profile baseline requires disposable Windows GitHub Actions");
    }
    await quit();
    // A transient file-sharing race immediately after WebView2 exit may
    // leave one unreadable entry. Retry only that case, with full coverage
    // required on a fresh scan. No file or category is silently excluded.
    const baselineAttempt = await auditProfilesWithTransientRetry([
      { label: "tauri_identifier_profile", root: path.join(process.env.LOCALAPPDATA, "org.medscale.desktop.preview") },
      { label: "legacy_medscale_profile", root: path.join(process.env.LOCALAPPDATA, "MedScale") },
    ]);
    const baseline = baselineAttempt.report;
    // No private paths, file names, values, audio or synthetic subject IDs:
    // save the same category counters as the post-journey scanner only.
    fs.writeFileSync(path.join(shots, "webview-disk-profile-before-workspace.json"),
      JSON.stringify({
        phase: "before_synthetic_workspace",
        observed_app_profile_bytes: hasObservedProfileBytes(baseline),
        complete_coverage: hasCompleteAuditCoverage(baseline),
        ...baseline,
      }, null, 2) + "\n", "utf8");
    console.log("WEBVIEW_SYNTHETIC_BASELINE=" + JSON.stringify({
      observed_bytes: hasObservedProfileBytes(baseline),
      coverage_complete: hasCompleteAuditCoverage(baseline),
      scan_attempts: baselineAttempt.attempts,
      profiles: baseline.profiles.map(({ label, exists, files_examined, bytes_examined,
        bounded, unreadable_entries, skipped_large_files, skipped_symlinks }) =>
        ({ label, exists, files_examined, bytes_examined, bounded, unreadable_entries,
          skipped_large_files, skipped_symlinks })),
    }));
    if (!baselineAttempt.qualified) {
      throw new Error("Disposable Windows baseline remained incomplete; refusing synthetic journeys");
    }
    // Reopen the very same app/profile; existing 26 journeys stay unchanged.
    await launch();
  }
  // Spec 108 step 4 (3b): kill the app during its first sample seed, then
  // the next launch must rebuild it from the bundled sample. The
  // e2e-webdriver build blocks the seed after fixture 3 and writes a marker.
  const seedMarker = path.join(os.tmpdir(), `medscale-e2e-seed-${process.pid}`);
  await quit();
  await launch({ MEDSCALE_E2E_BLOCK_SEED_AFTER: "3", MEDSCALE_E2E_SEED_MARKER: seedMarker });
  await journey("interrupted-first-seed-killed", async () => {
    await click(`//button[${text("Try MedScale with sample data")}]`, 60000);
    const end = Date.now() + 180000;
    while (!fs.existsSync(seedMarker)) {
      if (Date.now() > end) throw new Error("first seed never reached the blocking boundary");
      await sleep(250);
    }
  }, { screenshot: false });
  await quit();
  fs.rmSync(seedMarker, { force: true });
  await launch();
  await journey("launch-and-open-synthetic-workspace", async () => {
    await click(`//button[${text("Try MedScale with sample data")}]`, 60000);
    // The interrupted seed above was proven fixture-only and rebuilt.
    await see("Sample workspace rebuilt", 120000);
    await find(`//button[contains(@class,'nav') and ${text("Patients")}]`, 60000);
  });
  // Spec 108 step 3: a second copy against the same data directory must exit
  // with code 3 inside setup, before any window or workspace access, while
  // the first copy keeps working. Same env as the first copy so a regressed
  // guard would show up as a second page target, not as a WebView2 failure.
  await journey("second-instance-refused", async () => {
    const second = spawn(path.resolve(app), [], {
      env: {
        ...process.env,
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${DEBUG_PORT}`,
        MEDSCALE_E2E_HEADLESS_SECOND_INSTANCE: "1",
      },
      stdio: "ignore",
    });
    const code = await Promise.race([
      new Promise((r) => second.once("exit", (c) => r(c))),
      sleep(30000).then(() => "timeout"),
    ]);
    if (code === "timeout") {
      second.kill();
      throw new Error("second MedScale copy did not exit within 30 s");
    }
    if (code !== 3) throw new Error(`second MedScale copy exited ${code}, expected 3 (already running)`);
    const targets = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json();
    const pages = targets.filter((t) => t.type === "page");
    if (pages.length !== 1) throw new Error(`expected one MedScale page target, found ${pages.length}`);
    await find(`//button[contains(@class,'nav') and ${text("Patients")}]`, 30000);
  }, { screenshot: false });
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
  await journey("data-snapshot-and-analytics-query", async () => {
    await nav("Data");
    await click(`//button[${text("Add synthetic sample source")}]`);
    await click(`//button[contains(@class,'lr') and ${text("Synthetic fixture table")}]`, 60000);
    await click(`//button[${text("Import snapshot")}]`);
    const snap = await waitForText(/snap-[A-Za-z0-9_-]+/);
    await nav("Analytics");
    await type(`//input[@aria-label='Bindings']`, `t=${snap}`);
    await click(`//button[normalize-space(.)='Run']`);
    await see("4 rows", 60000);
    // The success toast/result can precede the asynchronous receipt reload.
    // Require the visible Core-backed receipt count after the UI settles.
    await find("//section[.//*[contains(@class,'sec-t') and normalize-space(.)='Receipts']]//span[contains(@class,'sec-c') and number(normalize-space(.)) > 0]", 30000);
  });
  await journey("analytics-explains-invalid-bindings", async () => {
    await type(`//input[@aria-label='Bindings']`, "not-a-binding");
    await click(`//button[normalize-space(.)='Run']`);
    await see("bindings are alias=snapshot_id pairs", 30000);
  });
  await journey("knowledge-index-and-search", async () => {
    await nav("Knowledge");
    await click(`//button[${text("Build index")}]`);
    await see("Index built", 60000);
    await type(`//input[@aria-label='Search indexed sources']`, "synthetic");
    await click(`//button[@type='submit' and ${text("Search")}]`);
    await find(`//pre[contains(@class,'pre')]`, 60000);
  });
  await journey("model-fleet-lanes", async () => {
    await nav("Model Fleet");
    await click(`//button[${text("Add two lanes")}]`);
    await see("Two reviewer lanes created", 60000);
  });
  await journey("collaboration-room", async () => {
    await nav("Collaboration");
    await type(`//input[@aria-label='Room name']`, "E2E tumor board");
    await click(`//button[@type='submit' and ${text("Add")}]`);
    await see("E2E tumor board", 30000);
  });
  await journey("browse-default-deny", async () => {
    await nav("Browse");
    await type(`//input[@aria-label='URL']`, "https://example.org/");
    await click(`//button[@type='submit' and ${text("Fetch")}]`);
    await see("empty_allowlist", 30000);
  });
  await journey("insights-grounded-answer", async () => {
    await nav("Insights");
    await type(`//input[@aria-label='Question for the evidence corpus']`, "When is an asthma rescue inhaler used?");
    await click(`//button[normalize-space(.)='Ask']`);
    await see("doc-asthma", 30000);
  });
  await journey("audio-synthetic-tone-import", async () => {
    await nav("Audio");
    // Desktop Core preflight checks actual available disk space without downloading.
    await find("//select[@aria-label='Available speech models']");
    await click("//button[normalize-space(.)='Check disk budget']");
    await see("Storage checked", 30000);
    await find("//*[@data-testid='audio-model-preflight' and contains(normalize-space(.),'No download has started')]");
    // Headless E2E must never approve or start network/model downloads.
    await find("//input[@aria-label='Approve Whisper model download' and not(@checked)]");
    await assertDisabled("//button[normalize-space(.)='Download, verify and install']");
    await assertDisabled("//button[normalize-space(.)='Activate saved model']");
    // Assert production's native chooser buttons are visible; do not open an
    // OS modal in unattended E2E. Real WAV/Pack inputs still use explicit paths.
    await find("//form[.//input[@id='ap1']]//button[@type='button' and contains(normalize-space(.),'Browse folder')]");
    await find("//form[.//input[@id='wp1']]//button[@type='button' and contains(normalize-space(.),'Browse WAV')]");
    await click(`//button[${text("Import synthetic tone sample")}]`);
    await see("Synthetic tone imported", 60000);
    // An action toast alone is insufficient; imported synthetic lineage must
    // also become visible in the active project after overview reload.
    await find("//section[.//*[contains(@class,'sec-t') and normalize-space(.)='Sources']]//span[contains(@class,'sec-c') and number(normalize-space(.)) > 0]", 30000);
  });
  await journey("audio-source-retention-readonly", async () => {
    // Probe the named Core-through-Tauri read command from the *real* Windows
    // WebView2 surface. No model or data is downloaded and no record is deleted.
    await nav("Audio");
    const sourceRow = "//section[.//*[contains(@class,'sec-t') and normalize-space(.)='Sources']]" +
      "//div[contains(@class,'lr') and .//button[normalize-space(.)='Stored data']]";
    await find(sourceRow, 30000);
    await click("(" + sourceRow + ")[1]//button[normalize-space(.)='Stored data']");
    await see("Stored recording inspected", 30000);
    await find("//p[@role='status' and contains(.,'Stored audio:') and " +
      "contains(.,'transcript revisions: 0') and contains(.,'audit receipts: 0') and " +
      "contains(.,'no deletion has been performed') and " +
      "contains(.,'Older backups may retain this recording')]", 30000);
    // The source is merely inspected. A destructive command cannot run
    // unless a user enters the exact source-specific confirmation phrase.
    const armed = await wd("POST", `/session/${sid}/execute/sync`, {
      script: "return [...document.querySelectorAll('button')].filter(b => /Remove this recording from active vault/.test(b.innerText)).every(b => b.disabled);",
      args: [],
    });
    if (armed !== true) throw new Error("unconfirmed recording removal is enabled");
  });
  await journey("audio-source-confirmed-removal-synthetic", async () => {
    // The test data belongs to this disposable CI vault. An explicit typed
    // phrase is required; the previous journey already proved the button
    // remains disabled before confirmation.
    const phrase = await wd("POST", `/session/${sid}/execute/sync`, {
      script: "return document.querySelector('#audio-removal-confirmation')?.parentElement?.querySelector('code')?.textContent || '';",
      args: [],
    });
    if (!/^REMOVE .+ FROM ACTIVE VAULT$/.test(phrase)) {
      throw new Error("source-specific recording removal phrase is absent");
    }
    await type("//input[@id='audio-removal-confirmation']", phrase);
    await find("//button[normalize-space(.)='Remove this recording from active vault' and not(@disabled)]");
    await click("//button[normalize-space(.)='Remove this recording from active vault']");
    // Capture a bounded, synthetic-only diagnostic while the native toast
    // is still visible. This distinguishes a Core refusal from an E2E
    // timing-only failure without printing audio or any user data.
    await sleep(700);
    const removalDiagnostic = await wd("POST", `/session/${sid}/execute/sync`, {
      script: "return [...document.querySelectorAll('[role=alert],[role=status],[aria-live]')].map(e => e.textContent?.trim().slice(0,240)).filter(Boolean).slice(0,6);",
      args: [],
    });
    console.log("E2E synthetic removal feedback", JSON.stringify(removalDiagnostic));
    await see("Recording removed from active vault; receipt recorded", 30000);
  });
  // A transcript lives in the vault, so leaving the page must not lose it. The
  // segmentation route needs no model and always records a draft revision.
  await journey("audio-transcript-reopens-after-leaving-the-page", async () => {
    await click(`//button[${text("Import synthetic tone sample")}]`);
    await see("Synthetic tone imported", 60000);
    const row = "//section[.//*[contains(@class,'sec-t') and normalize-space(.)='Sources']]" +
      "//div[contains(@class,'lr') and .//button[normalize-space(.)='Open transcript']]";
    await find(row, 30000);
    const select = await find("//select[@aria-label='Transcription route']");
    await wd("POST", `/session/${sid}/execute/sync`, {
      script: "const s=arguments[0]; Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,'value').set.call(s,'segmentation_only'); s.dispatchEvent(new Event('change',{bubbles:true}));",
      args: [{ [ELEMENT]: select }],
    });
    await click(`(${row})[last()]//button[normalize-space(.)='Transcribe']`);
    await see("Transcription recorded", 60000);
    await nav("Settings");
    await see("Appearance");
    await nav("Audio");
    await find(row, 30000);
    const transcript = "//section[.//*[contains(@class,'sec-t') and normalize-space(.)='Transcript']]";
    const gone = await wd("POST", `/session/${sid}/execute/sync`, {
      script: "return !document.evaluate(arguments[0], document, null, 9, null).singleNodeValue;",
      args: [transcript],
    });
    if (!gone) throw new Error("expected the Transcript section to start closed after returning");
    await click(`(${row})[last()]//button[normalize-space(.)='Open transcript']`);
    await see("Latest transcript opened", 30000);
    await find(`${transcript}//*[contains(@class,'sec-c') and contains(normalize-space(.),'revision 1')]`, 30000);
  }, { screenshot: false });
  // The real audio journey runs ONLY on an isolated CI host with opt-in inputs.
  // Never screenshot or print genuine voice text, paths, speakers or models.
  const guiModelInstall = process.env.MEDSCALE_E2E_MODEL_GUI_DOWNLOAD === "1";
  if (process.env.MEDSCALE_E2E_REAL_WAV_PATH || process.env.MEDSCALE_E2E_REAL_ASR_PACK_PATH || guiModelInstall) {
    if (!process.env.MEDSCALE_E2E_REAL_WAV_PATH ||
        (!guiModelInstall && !process.env.MEDSCALE_E2E_REAL_ASR_PACK_PATH)) {
      throw new Error("real audio CI needs a licensed WAV and either pinned UI acquisition or an already verified Pack");
    }
    await journey("real-human-speech-local-whisper-ui", async () => {
      await nav("Audio");
      if (guiModelInstall) {
        // Opt-in only for the dedicated licensed-real-voice CI workflow. All
        // ordinary E2E remains strictly offline and never approves a download.
        await click("//button[normalize-space(.)='Check disk budget']");
        await see("Storage checked", 30000);
        await see("Enough local storage", 30000);
        await click("//input[@aria-label='Approve Whisper model download']");
        await click("//button[normalize-space(.)='Download, verify and install']");
        // Exercise the production HTTP download and Pack signature checks,
        // not preassembled weights or an injected test engine.
        await waitForText(/Model download: ready\./, 420000);
        await see("Local Whisper engine installed", 180000);
      } else {
        await type("//input[@id='ap1']", process.env.MEDSCALE_E2E_REAL_ASR_PACK_PATH);
        await click("//form[.//input[@id='ap1']]//button[@type='submit']");
        await see("Audio Pack admitted", 180000);
        await see("Local Whisper engine installed", 180000);
      }
      await type("//input[@id='wp1']", process.env.MEDSCALE_E2E_REAL_WAV_PATH);
      await click("//form[.//input[@id='wp1']]//button[@type='submit']");
      await see("Recording imported", 60000);
      const quotedFile = JSON.stringify(path.basename(process.env.MEDSCALE_E2E_REAL_WAV_PATH));
      const realSource = "//section[.//*[contains(@class,'sec-t') and normalize-space(.)='Sources']]" +
        "//div[contains(@class,'lr') and .//span[contains(normalize-space(.), " + quotedFile + ")]]";
      await find(realSource, 60000);
      await click(realSource + "//button[normalize-space(.)='Transcribe']", 60000);
      await see("Transcription recorded", 240000);
      await find(
        "//section[.//*[contains(@class,'sec-t') and normalize-space(.)='Transcript']]" +
          "//div[contains(@class,'lr')]/span[3]" +
          "[string-length(normalize-space(.)) > 8]" +
          "[not(contains(., '(untranscribed)'))]" +
          "[not(contains(., '[fixture'))]",
        90000,
      );
      const nonFixtureWhisper = await wd("POST", `/session/${sid}/execute/sync`, {
        script: "const s=[...document.querySelectorAll('section')].find(s => /Transcript/.test(s.innerText) && /engine/.test(s.innerText)); return !!s && /whisper/i.test(s.innerText) && !/\\(fixture\\)/i.test(s.innerText);",
        args: [],
      });
      if (!nonFixtureWhisper) throw new Error("verified nonfixture local Whisper receipt not visible");
    }, { screenshot: false });
    await journey("real-human-transcript-linked-retention-denies-removal", async () => {
      // Genuine CC-BY-4.0 human audio must remain protected after its real
      // transcript and audit receipt were created. The Core allows removal
      // only of unreferenced sources; never discard linked evidence here.
      // This checks the actual native WebView2 UI with no audio, path,
      // transcript, source identifier or screenshot in public CI output.
      await nav("Audio");
      const name = JSON.stringify(path.basename(process.env.MEDSCALE_E2E_REAL_WAV_PATH));
      const source = "//section[.//*[contains(@class,'sec-t') and normalize-space(.)='Sources']]" +
        "//div[contains(@class,'lr') and .//span[contains(normalize-space(.), " + name + ")]]";
      await find(source, 30000);
      await click(source + "//button[normalize-space(.)='Stored data']");
      await see("Stored recording inspected", 30000);
      await find("//p[@role='status' and contains(.,'Stored audio:') and " +
        "contains(.,'transcript revisions: 1') and contains(.,'audit receipts: 1') and " +
        "contains(.,'Linked evidence may prevent removal') and " +
        "contains(.,'Older backups may retain this recording')]", 30000);
      const denied = await wd("POST", `/session/${sid}/execute/sync`, {
        script: "return !document.querySelector('#audio-removal-confirmation') && ![...document.querySelectorAll('button')].some(b => b.textContent?.trim() === 'Remove this recording from active vault');",
        args: [],
      });
      if (denied !== true) {
        throw new Error("linked real human speech was offered destructive active-vault removal");
      }
    }, { screenshot: false });
  }
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
  // Catalog inventory is metadata, not downloaded or executable models.
  // Exercise the actual IPC pagination, not just the tab's initial shell.
  await journey("models-catalog-pagination", async () => {
    await see("A listing is not a download");
    await click(`//button[normalize-space(.)='Next']`);
    await waitForText(/rows 51[–-]100/, 60000);
    await click(`//button[normalize-space(.)='Previous']`);
    await waitForText(/rows 1[–-]50/, 60000);
  });
  // The Rust Core already tests real synthetic ONNX evaluation, but a user
  // must also be able to select the fixture and run it via packaged WebView2.
  // medagent_models exposes only admitted runnable Packs; no network fetch.
  await journey("models-evaluate-synthetic-onnx", async () => {
    await click(`//button[@role='tab' and ${text("Try a model")}]`);
    await find(`//select[@id='ep']`, 60000);
    const until = Date.now() + 60000;
    let packId = "";
    while (!packId && Date.now() < until) {
      packId = await wd("POST", `/session/${sid}/execute/sync`, {
        script: "return document.querySelector('#ep')?.value || '';",
        args: [],
      });
      if (!packId) await sleep(250);
    }
    if (!packId) throw new Error("no admitted runnable synthetic model Pack selected");
    await click(`//button[@type='submit' and normalize-space(.)='Evaluate']`);
    await see("Full proposal payload", 60000);
    await see("Prepared cache");
    const evidence = await wd("POST", `/session/${sid}/execute/sync`, {
      script: "const t = document.body.innerText; return { localRuntime: !!document.querySelector('.lk[title=runtime]') && !!document.querySelector('.lk[title=audit]'), authority: t.includes('Evaluation evidence, not a record.') };",
      args: [],
    });
    if (!evidence.localRuntime || !evidence.authority) {
      throw new Error("synthetic model evaluation omitted runtime/audit or no-promotion boundary");
    }
  });
  await journey("documents", async () => {
    await nav("Documents");
    await see("patient-golden.json");
  });
  // Import a byte-distinct fictional FHIR R4 Patient through the real
  // Documents UI and Core. The native file chooser is asserted present but
  // never opened unattended; the path field is the chooser's own target.
  const novelFhir = path.join(os.tmpdir(), `medscale-e2e-${process.pid}`, "e2e-novel-fictional-patient.json");
  await journey("documents-import-novel-fhir-ui", async () => {
    const bundled = JSON.parse(fs.readFileSync(path.resolve("fixtures/synthetic/fhir/r4/presentation/patient-golden.json"), "utf8"));
    fs.mkdirSync(path.dirname(novelFhir), { recursive: true });
    fs.writeFileSync(novelFhir, JSON.stringify({ ...bundled, id: "e2e-novel-fictional-patient" }));
    const before = fs.readFileSync(novelFhir);
    await nav("Documents");
    await click(`//button[${text("Import synthetic FHIR")}]`);
    await find("//form[.//input[@id='ip']]//button[@type='button' and contains(normalize-space(.),'Browse JSON')]");
    await type(`//input[@id='ip']`, novelFhir);
    await click(`//button[@type='submit' and ${text("Ingest through Core")}]`);
    await see("ingested and promoted as", 60000);
    await see("imported:e2e-novel-fictional-patient.json", 30000);
    // The user's source file must be read, never rewritten or moved.
    if (!fs.readFileSync(novelFhir).equals(before)) throw new Error("FHIR import modified the user's source file");
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
  // These routes refresh Core Doctor on Windows. They previously launched
  // vssadmin with a visible short-lived console child; keep the real IPC
  // navigation paths covered in native E2E alongside the silent-probe fix.
  await journey("settings-doctor", async () => {
    await nav("Settings");
    await see("Appearance");
    await see("Privacy & network", 60000);
    const handles = await wd("GET", `/session/${sid}/window/handles`);
    if (handles.length !== 1) throw new Error(`Settings unexpectedly opened ${handles.length} WebView windows`);
  });
  // Keyboard and screen-reader basics that unit tests cannot prove: focus moves
  // to the new page, real sub-headings exist, a search hit responds to a real
  // key press, and changing a setting does not drop focus to the document body.
  await journey("keyboard-focus-and-page-headings", async () => {
    const evalJs = (script, args = []) => wd("POST", `/session/${sid}/execute/sync`, { script, args });
    const poll = async (script, what) => {
      for (let i = 0; i < 40; i++) {
        if (await evalJs(script)) return;
        await new Promise((r) => setTimeout(r, 250));
      }
      throw new Error(`Keyboard journey: ${what}`);
    };
    await nav("Evidence");
    await poll("return document.activeElement && document.activeElement.id === 'main';", "focus did not move to main after navigating");
    await poll("return document.title === 'Evidence · MedScale';", "page title did not name the route");
    await poll("return document.querySelectorAll('main h1').length === 1 && document.querySelectorAll('main h2').length > 0;", "page has no h1 and h2 structure");
    await type(`//input[@aria-label='Search corpus text']`, "asthma");
    await click(`//button[@type='submit' and ${text("Search")}]`);
    const hit = await find(`//div[@role='button' and starts-with(@aria-label,'Open ')]`, 30000);
    await wd("POST", `/session/${sid}/element/${hit}/value`, { text: "" });
    await poll(`return !!document.querySelector("div[role='button'][aria-label^='Open '].sel");`, "Enter on a search hit did not select it");
    await nav("Settings");
    const radio = await find(`//div[@role='radiogroup' and @aria-label='Theme']/button[@aria-checked='false']`);
    const label = await evalJs("arguments[0].focus(); arguments[0].click(); return arguments[0].textContent;", [{ [ELEMENT]: radio }]);
    await poll(`const a = document.activeElement; return a && a.getAttribute('role') === 'radio' && a.textContent === ${JSON.stringify(label)} && a.getAttribute('aria-checked') === 'true';`, "changing the theme dropped keyboard focus");
    const back = await find(`//div[@role='radiogroup' and @aria-label='Theme']/button[@aria-checked='false']`);
    await evalJs("arguments[0].click();", [{ [ELEMENT]: back }]);
  });
  await journey("about-doctor", async () => {
    await nav("About");
    await see("Tauri 2", 60000);
    await see("Apache-2.0");
    // Spec 107 D2: copy-only support path with a fixed URL and the SUPPORT.md
    // data rule. Clipboard contents are never read back.
    await find("//code[@aria-label='Support address' and normalize-space(.)='https://github.com/AbdulazizShehri/MedScale/issues/new/choose']");
    await find(`//button[${text("Copy diagnostics")}]`);
    await click(`//button[${text("Copy link")}]`);
    await see("Never upload real patient records");
    const handles = await wd("GET", `/session/${sid}/window/handles`);
    if (handles.length !== 1) throw new Error(`About unexpectedly opened ${handles.length} WebView windows`);
  });
  await journey("webview-storage-canary", async () => {
    await auditClientStorage("after-synthetic-journeys");
  }, { screenshot: false });
  await quit();
  // Relaunch the binary: the vault must bring the run back.
  await launch();
  await journey("restart-persistence", async () => {
    await click(`//button[${text("Try MedScale with sample data")}]`, 60000);
    await nav("MedAgent");
    await see("summarize the bound sources", 60000);
    await see("completed");
  });
  // Reopening must attest the imported row against durable Core evidence
  // (fail closed otherwise) and show it exactly once, without replay.
  await journey("imported-fhir-survives-restart-once", async () => {
    await nav("Documents");
    await see("imported:e2e-novel-fictional-patient.json", 60000);
    const count = await wd("POST", `/session/${sid}/execute/sync`, {
      script: "return [...document.querySelectorAll('table[aria-label=\"Sources\"] tbody tr')].filter(r => r.textContent.includes('imported:e2e-novel-fictional-patient.json')).length;",
      args: [],
    });
    if (count !== 1) throw new Error(`imported FHIR row shown ${count} times after restart, expected exactly 1`);
    fs.rmSync(path.dirname(novelFhir), { recursive: true, force: true });
  });
  if (process.env.MEDSCALE_E2E_MODEL_GUI_DOWNLOAD === "1") {
    await journey("whisper-still-active-after-restart", async () => {
      await nav("Audio");
      await see("Local Whisper engine installed", 180000);
      const saved = await wd("POST", `/session/${sid}/execute/sync`, {
        script: "const e=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Activate saved model'); return !!e && e.disabled===false;",
        args: [],
      });
      if (!saved) throw new Error("verified downloaded model is not recoverable after restart");
    }, { screenshot: false });
  }
  await journey("webview-storage-canary-after-restart", async () => {
    await auditClientStorage("after-restart");
  }, { screenshot: false });
  // Switch to the dark theme through the command palette, as a user would,
  // then capture the main product areas in it.
  await journey("dark-theme-via-command-palette", async () => {
    await click(`//button[${text("Search or run")}]`);
    await type(`//input[@aria-label='Search or run']`, "dark");
    await click(`//*[@role='option' and ${text("Switch to dark theme")}]`);
    const theme = await wd("POST", `/session/${sid}/execute/sync`, { script: "return document.querySelector('.ms')?.getAttribute('data-theme');", args: [] });
    if (theme !== "dark") throw new Error(`theme is ${theme}, expected dark`);
    for (const [route, expect] of [["Patients", "Ada Synthetic"], ["Insights", "Present facts"], ["Knowledge", null], ["MedAgent", "summarize the bound sources"], ["Models", "MedScale Catalog"], ["Audit Trail", /synthetic-subject-ada|No disclosures recorded/]]) {
      await nav(route);
      if (route === "Knowledge") {
        // The restart starts a fresh session: rebuild the index and search again.
        await click(`//button[${text("Build index")}]`);
        await see("Index built", 60000);
        await type(`//input[@aria-label='Search indexed sources']`, "synthetic");
        await click(`//button[@type='submit' and ${text("Search")}]`);
        await find(`//pre[contains(@class,'pre')]`, 60000);
      } else if (expect instanceof RegExp) {
        await waitForText(expect, 30000);
      } else {
        await see(expect, 30000);
      }
      await sleep(900);
      await shot(`dark-${route.toLowerCase().replace(/\s+/g, "-")}`);
    }
  });
  // Spec 108 step 5: an un-openable sample (manifest removed after quit, on
  // a disposable Windows runner only) is set aside and replaced. The
  // set-aside copy holds the earlier user FHIR import.
  if (process.env.GITHUB_ACTIONS === "true" && process.platform === "win32" && process.env.LOCALAPPDATA) {
    // Spec 109 Part C through the real screens. A throwaway passphrase on a
    // disposable runner. The code is read from the page and never printed,
    // and this journey takes no screenshot because the page shows the codes.
    await journey("encrypted-vault-recovery-code-unlock", async () => {
      const passphrase = "e2e-disposable-runner-passphrase";
      const opened = `//button[contains(@class,'nav') and ${text("Patients")}]`;
      const lock = `//button[contains(@class,'nav') and ${text("Lock workspace")}]`;
      await click(lock);
      await see("Create encrypted vault", 30000);
      await type("//input[@id='pp']", passphrase);
      await type("//input[@id='pp2']", passphrase);
      await click("//button[@type='submit' and not(@disabled)]");
      await see("Save your recovery codes", 120000);
      const code = await wd("POST", `/session/${sid}/execute/sync`, {
        script: "return document.querySelector(\"[aria-label='Recovery codes'] span\").textContent;",
        args: [],
      });
      if (!code || code.length < 8) throw new Error("no recovery code was shown");
      await click(`//button[${text("I saved them")}]`);
      await find(opened, 120000);

      await click(lock);
      await see("Unlock workspace", 60000);
      await click(`//button[${text("Use a recovery code")}]`);
      await see("Recovery code", 10000);
      await type("//input[@id='pp']", "not-a-real-recovery-code");
      await click("//button[@type='submit' and not(@disabled)]");
      await see("did not open this vault", 120000);
      await type("//input[@id='pp']", code);
      await click("//button[@type='submit' and not(@disabled)]");
      await find(opened, 120000);

      await click(lock);
      await see("Unlock workspace", 60000);
      await type("//input[@id='pp']", passphrase);
      await click("//button[@type='submit' and not(@disabled)]");
      await find(opened, 120000);
    }, { screenshot: false });
    // Spec 109 Part B: a real kill while unlocked, with committed work that has
    // never been sealed. The next unlock must recover it and say so.
    await journey("encrypted-vault-recovers-after-a-kill", async () => {
      await nav("Projects");
      await type(`//input[@id='pn']`, "E2E work from before the kill");
      await click(`//button[@type='submit' and ${text("Create project")}]`);
      await see("E2E work from before the kill");
      execFileSync("taskkill", ["/F", "/T", "/IM", path.basename(app)], { stdio: "ignore" });
      await quit();
      await launch();
      // A closed encrypted vault opens straight on unlock, with no Welcome first.
      await see("Unlock workspace", 60000);
      await type("//input[@id='pp']", "e2e-disposable-runner-passphrase");
      await click("//button[@type='submit' and not(@disabled)]");
      await see("Recovered changes from your last session", 120000);
      await nav("Projects");
      await see("E2E work from before the kill", 60000);
    }, { screenshot: false });
    // Spec 109 PR 3: what recovery kept is visible, and restoring it is explicit.
    // The copy predates the project, so restoring removes it, and the state being
    // replaced stays listed so the restore can be undone.
    await journey("encrypted-vault-restore-a-kept-copy", async () => {
      await nav("Settings");
      await see("Crash recovery", 30000);
      await click(`//button[starts-with(@aria-label,'Restore Earlier saved copy')]`, 30000);
      await see("Restore this copy?", 10000);
      await see("shorten the audit trail", 10000);
      await click(`//button[${text("Restore and lock")}]`);
      await see("Unlock workspace", 120000);
      await type("//input[@id='pp']", "e2e-disposable-runner-passphrase");
      await click("//button[@type='submit' and not(@disabled)]");
      await find(`//button[contains(@class,'nav') and ${text("Patients")}]`, 120000);
      await nav("Projects");
      await find("//input[@id='pn']", 60000);
      await see("No projects", 60000);
      if ((await pageText()).includes("E2E work from before the kill")) {
        throw new Error("the restored vault still shows work newer than the copy");
      }
      await nav("Settings");
      await find(`//button[starts-with(@aria-label,'Restore Earlier saved copy')]`, 30000);
    }, { screenshot: false });
    await quit();
    const appData = path.join(process.env.LOCALAPPDATA, "org.medscale.desktop.preview");
    const manifest = path.join(appData, "synthetic-workspace.desktop-seed.json");
    if (!fs.existsSync(manifest)) throw new Error("expected the committed sample manifest before breaking it");
    fs.rmSync(manifest);
    await launch();
    await journey("unopenable-sample-set-aside", async () => {
      // An encrypted vault exists by now, so the sample is one explicit step away.
      await click(`//button[${text("Explore sample data instead")}]`, 60000);
      await click(`//button[${text("Try MedScale with sample data")}]`, 60000);
      await see("The sample workspace was not finished setting up.", 60000);
      await click(`//button[${text("Set the sample workspace aside")}]`);
      await see("not encrypted", 10000);
      await click(`//button[${text("Set aside and start a new sample")}]`);
      await find(`//button[contains(@class,'nav') and ${text("Patients")}]`, 180000);
      const slotRoot = path.join(appData, "synthetic-set-aside");
      if (!fs.existsSync(slotRoot) || fs.readdirSync(slotRoot).length !== 1) {
        throw new Error("expected exactly one completed set-aside slot");
      }
    });
  }
} finally {
  await quit();
  fs.writeFileSync(path.join(shots, "results.json"), JSON.stringify(results, null, 1));
  const failed = results.filter((r) => !r.ok).length;
  console.log(`E2E_RESULTS ${JSON.stringify({ journeys: results.length, failed })}`);
}
