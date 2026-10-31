/**
 * UI-only first-session smoke test. Tauri IPC is simulated here; the real-Core
 * synthetic ingest and installer smoke are separately covered by Rust and CI.
 * Never use this to claim native install, OS Gatekeeper, or PHI qualification.
 */
import assert from "node:assert/strict";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const dist = path.join(appRoot, "dist");
const allowed = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".woff2": "font/woff2", ".png": "image/png", ".ico": "image/x-icon" };
const server = http.createServer((req, res) => {
  const url = new URL(req.url ?? "/", "http://localhost");
  const file = path.resolve(dist, "." + (url.pathname === "/" ? "/index.html" : url.pathname));
  if (!(file === path.join(dist, "index.html") || file.startsWith(dist + path.sep))) {
    res.writeHead(403); res.end(); return;
  }
  fs.readFile(file, (err, bytes) => {
    res.writeHead(err ? 404 : 200, { "Content-Type": allowed[path.extname(file)] ?? "application/octet-stream" });
    res.end(err ? "Not found" : bytes);
  });
});

const model = () => {
  let open = false;
  const calls = [];
  const subjects = [
    { subject_ref: "synthetic-subject-ada", display_name: "Ada (synthetic)", condition_summary: "Synthetic example", coverage_slots: [], latest_event: null },
    { subject_ref: "synthetic-subject-coverage", display_name: "Coverage example (synthetic)", condition_summary: "Synthetic example", coverage_slots: [], latest_event: null },
  ];
  const initial = { open: false, version: "0.1.0", network_default_deny: true, encrypted_vault_exists: false, real_phi_authorized: false, workspace: null };
  const impl = {
    calls,
    invoke: async (cmd) => {
      calls.push(cmd);
      if (cmd === "workspace_status") return open ? { ...initial, open: true, workspace: { subjects: 2, sources: 12 } } : initial;
      if (cmd === "workspace_open_synthetic") { open = true; return { subjects: 2, sources: 12 }; }
      if (cmd === "patients_list") return { subjects };
      if (cmd === "evidence_corpus") return { source_identity: "synthetic-qa", documents: [] };
      if (cmd === "models_overview") return { models: [] };
      if (cmd === "workflow_overview") return { tasks: [], outbox_entries: 0, steps: [] };
      if (cmd === "projects_list") return [];
      throw { kind: "unavailable", message: "UI test fixture omits " + cmd };
    }
  };
  window.__firstRunCommands = impl.calls;
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main", windowLabel: "main" } },
    transformCallback: () => 0,
    invoke: impl.invoke,
  };
};
async function run() {
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const addr = server.address();
  const origin = "http://127.0.0.1:" + addr.port;
  const browser = await chromium.launch({
    executablePath: process.env.MEDSCALE_EDGE_PATH || "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
    headless: true,
  });
  const checks = [];
  try {
    for (const width of [375, 768, 1280]) {
      const page = await browser.newPage({ viewport: { width, height: 820 }, reducedMotion: "reduce" });
      const exceptions = [], external = [];
      page.on("pageerror", (err) => exceptions.push(err.message));
      page.on("request", (r) => { if (!r.url().startsWith(origin)) external.push(r.url()); });
      await page.addInitScript(model);
      await page.addInitScript(() => window.localStorage.setItem("medscale.project", "synthetic-legacy-project-marker"));
      const res = await page.goto(origin, { waitUntil: "networkidle" });
      assert.equal(res.status(), 200, "served desktop bundle");
      await page.getByRole("heading", { name: /your local ai workspace/i }).waitFor();
      const primary = page.getByRole("button", { name: "Try MedScale with sample data" });
      const advanced = page.getByRole("button", { name: "Set up or unlock encrypted vault" });
      assert.equal(await primary.isVisible(), true);
      assert.equal(await advanced.isVisible(), true);
      assert.equal(await page.getByText("Beta · synthetic data only", { exact: false }).count() > 0, true);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true, "no horizontal overflow");
      await primary.click();
      await page.getByRole("heading", { name: "Workspace", exact: true }).waitFor({ timeout: 12000 });
      assert.equal(await page.evaluate(() => localStorage.getItem("medscale.project")), null,
        "legacy last-project ID is cleared from browser storage on startup");
      const cmds = await page.evaluate(() => window.__firstRunCommands);
      assert.equal(cmds.filter((cmd) => cmd === "workspace_open_synthetic").length, 1, "one-click synthetic open");
      assert.equal(cmds.includes("workspace_create_encrypted"), false, "no vault creation on primary CTA");
      assert.equal(cmds.includes("workspace_unlock"), false, "no passphrase on primary CTA");
      assert.equal(exceptions.length, 0, "no uncaught browser exceptions: " + exceptions.join("; "));
      assert.deepEqual(external, [], "no remote requests in UI smoke");
      // Keyboard/screen-reader smoke on the built frontend (simulated IPC).
      // This does not substitute for a full native VoiceOver/Narrator audit.
      if (width === 1280) {
        const opener = page.locator("button.sb-search");
        await opener.focus();
        await opener.click();
        const dialog = page.getByRole("dialog", { name: "Search or run" });
        await dialog.waitFor();
        const combobox = page.getByRole("combobox", { name: "Search or run" });
        assert.equal(await combobox.evaluate((el) => document.activeElement === el), true, "palette focuses its search field");
        assert.equal(await combobox.getAttribute("aria-expanded"), "true", "palette exposes expanded combobox");
        assert.equal(await combobox.getAttribute("aria-controls"), "palette-list", "combobox owns its listbox");
        await combobox.press("ArrowDown");
        assert.equal(await combobox.getAttribute("aria-activedescendant") !== null, true, "keyboard selection remains exposed");
        await combobox.press("Escape");
        await dialog.waitFor({ state: "hidden" });
        assert.equal(await opener.evaluate((el) => document.activeElement === el), true, "closing palette restores keyboard focus");
        checks.push({ viewport: width, route: "palette combobox keyboard focus and Escape restore", passed: true });
      }
      checks.push({ viewport: width, route: "welcome -> synthetic -> workspace", passed: true, commands: cmds });
      await page.close();
    }
    const advanced = await browser.newPage({ viewport: { width: 1280, height: 820 } });
    await advanced.addInitScript(model);
    await advanced.goto(origin, { waitUntil: "networkidle" });
    await advanced.getByRole("button", { name: "Set up or unlock encrypted vault" }).click();
    await advanced.getByRole("heading", { name: "Create encrypted vault" }).waitFor({ timeout: 10000 });
    const advancedCmds = await advanced.evaluate(() => window.__firstRunCommands);
    assert.equal(advancedCmds.includes("workspace_open_synthetic"), false);
    checks.push({ viewport: 1280, route: "welcome -> encrypted vault (opt-in)", passed: true });
    await advanced.close();
  } finally {
    await browser.close();
    await new Promise((resolve) => server.close(resolve));
  }
  const out = process.env.MEDSCALE_FIRST_RUN_QA_REPORT;
  if (out) { fs.mkdirSync(path.dirname(path.resolve(out)), { recursive: true }); fs.writeFileSync(out, JSON.stringify({ simulatedIPC: true, checks }, null, 2)); }
  console.log("First-run browser smoke: PASS " + checks.length + "/" + checks.length + " (simulated IPC; native qualification separate)");
}
run().catch((error) => { console.error(error); server.close(); process.exitCode = 1; });
