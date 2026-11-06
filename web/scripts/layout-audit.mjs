// Layout audit for the static export. Run after `next build`. It opens every page at
// each width and fails if a checked edge misses the 12-column grid, a row's baselines
// or heights disagree, a target is under 44 px at touch widths, or the page overflows.
// Uses installed Microsoft Edge by default; AUDIT_BROWSER=chrome selects Chrome.
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { chromium } from "playwright-core";

const out = path.resolve("out");
const pages = ["/", "/product/", "/models/", "/download/", "/docs/", "/community/"];
const widths = [1440, 1024, 768, 375, 320];
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".webp": "image/webp", ".png": "image/png", ".woff2": "font/woff2" };

function audit() {
  const TOL = 0.75, misses = [];
  const counts = { edges: 0, rows: 0 };
  const visible = (el) => { const r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== "hidden" && !el.closest(".mobile-menu, .sr-only"); };
  const label = (el) => `${el.tagName.toLowerCase()}.${el.getAttribute("class") || ""} "${(el.innerText || "").trim().slice(0, 24).replace(/\s+/g, " ")}"`;
  const probe = document.createElement("div");
  probe.className = "shell"; probe.style.cssText = "position:absolute;top:0;left:0;right:0;height:0"; document.body.appendChild(probe);
  const box = probe.getBoundingClientRect(), css = getComputedStyle(probe);
  const L = box.left + parseFloat(css.paddingLeft), R = box.right - parseFloat(css.paddingRight); probe.remove();
  const gap = 24, col = (R - L - 11 * gap) / 12;
  const starts = Array.from({ length: 12 }, (_, i) => L + i * (col + gap)), ends = starts.map((s) => s + col);
  const near = (x, lines) => lines.reduce((best, v) => (Math.abs(x - v) < Math.abs(best) ? x - v : best), Infinity);

  const LEFT = ".brand-link img,.hero-copy > *,.hero-stage,.frame,.frame-caption,.strip-item,.proof-item,.section-head,.section-head > *,.eyebrow,.h2,.display,.lede,.story li > *,.tour > *,.tour-tab strong,.tour-points > div,.evidence,.pillar,.states,.state,.arch > *,.arch > .stack > *,.platforms,.platform,.dev > *,.dev > .stack > *,.cards,.card,.faq > *,.faq > .stack > *,.faq-list summary,.footer-grid > *,.footer-brand > *,.footer-col h2,.footer-col a,.footer-bottom > *,.closing-inner > *,.page-hero > *,.feature-row > *,.feature-copy > *,.notice,.table-wrap,.prose > *,.download-hero > *,.download-foot > *";
  const RIGHT = ".frame,.tour-panel,.tour-points,.evidence,.pillars,.states,.arch-diagram,.platforms,.terminal,.cards,.faq-list,.table-wrap,.notice,.footer-bottom,.built-on,.story";
  const INSIDE = ".pillar *, .card *, .platform *, .evidence *, .state *, .notice *, .arch-diagram *, .terminal *, .frame *";
  const RIGHT_ALIGNED = ".footer-bottom > :last-child,.download-foot > :last-child,.frame-caption > :last-child";
  for (const el of document.querySelectorAll(LEFT)) {
    if (el.closest(".tour-tabs") && getComputedStyle(el.closest(".tour-tabs")).display === "flex") continue;
    if (!visible(el) || (el.matches(INSIDE) && !el.matches(".pillar,.card,.platform,.state,.notice,.frame"))) continue;
    counts.edges++;
    const r = el.getBoundingClientRect(), d = near(r.left, starts);
    if (Math.abs(d) > TOL && !(el.matches(RIGHT_ALIGNED) && Math.abs(r.right - R) <= TOL)) misses.push(`left ${d.toFixed(1)} ${label(el)}`);
  }
  for (const el of document.querySelectorAll(RIGHT)) {
    if (!visible(el)) continue;
    counts.edges++;
    const d = near(el.getBoundingClientRect().right, ends);
    if (Math.abs(d) > TOL) misses.push(`right ${d.toFixed(1)} ${label(el)}`);
  }

  const baseline = (el, last) => {
    const walk = document.createTreeWalker(el, NodeFilter.SHOW_TEXT, { acceptNode: (n) => (n.textContent.trim() ? 1 : 2) });
    let node = walk.nextNode(); if (!node) return null;
    if (last) for (let n = walk.nextNode(); n; n = walk.nextNode()) node = n;
    const wrap = document.createElement("span"); node.parentNode.insertBefore(wrap, node); wrap.appendChild(node);
    const mark = document.createElement("span"); mark.style.cssText = "display:inline-block;width:0;height:0";
    if (last) wrap.appendChild(mark); else wrap.insertBefore(mark, node);
    const y = mark.getBoundingClientRect().bottom;
    mark.remove(); wrap.parentNode.insertBefore(node, wrap); wrap.remove();
    return y;
  };
  const OWNER = ".strip-item,.proof-item,.tour-points > div,.pillar,.state,.card,.platform,.arch-node,.footer-col";
  for (const q of [".proof-item span",".proof-item small",".strip-item strong",".strip-item span",".tour-points strong",".tour-points span",".pillar h3",".pillar .body",".state small",".state strong",".state p",".card-top small",".card strong",".card p",".platform h3",".platform-sub",".platform-state",".platform-note",".footer-col h2",".footer-bottom > *",".download-foot > *",".frame-caption > *",".evidence-meta > *",".evidence-foot > *",".story-n, .story h3",".story h3, .story .body"]) {
    const rows = new Map();
    for (const el of document.querySelectorAll(q)) {
      if (!visible(el)) continue;
      const key = Math.round((el.closest(`${OWNER},.story li`) || el).getBoundingClientRect().top / 4);
      if (!rows.has(key)) rows.set(key, []);
      rows.get(key).push(el);
    }
    for (const row of rows.values()) {
      const xs = row.map((e) => e.getBoundingClientRect().left);
      if (row.length < 2 || Math.max(...xs) - Math.min(...xs) < 1) continue;
      const ys = row.map((e) => baseline(e)).filter((y) => y !== null);
      const tops = row.map((e) => e.getBoundingClientRect().top);
      if (Math.max(...tops) - Math.min(...tops) > 40) continue; // Stacked, not one row.
      counts.rows++;
      const spread = Math.max(...ys) - Math.min(...ys);
      if (spread > TOL) misses.push(`baseline ${spread.toFixed(1)} ${q}`);
    }
  }
  for (const head of document.querySelectorAll(".section-head.split")) {
    const a = head.querySelector(".h2"), b = head.children[1];
    if (b.getBoundingClientRect().left <= a.getBoundingClientRect().left + 1) continue;
    counts.rows++;
    const spread = Math.abs(baseline(a, true) - baseline(b, true));
    if (spread > TOL) misses.push(`head baseline ${spread.toFixed(1)}`);
  }
  for (const q of [".strip-item", ".pillar", ".state", ".card", ".platform"]) {
    const rows = new Map();
    for (const el of document.querySelectorAll(q)) {
      if (!visible(el)) continue;
      const r = el.getBoundingClientRect(), key = Math.round(r.top);
      if (!rows.has(key)) rows.set(key, []);
      rows.get(key).push(r.height);
    }
    for (const hs of rows.values()) if (hs.length > 1 && Math.max(...hs) - Math.min(...hs) > TOL) misses.push(`height ${hs.join("/")} ${q}`);
  }
  if (innerWidth < 860) {
    for (const el of document.querySelectorAll("a, button, summary")) {
      if (!visible(el) || el.matches(".claim, .notice .text-link, .prose .text-link, .body .text-link, .footer-bottom span a")) continue;
      const r = el.getBoundingClientRect();
      if (r.height < 43.5 && getComputedStyle(el).display !== "inline") misses.push(`target ${r.height.toFixed(0)}px ${label(el)}`);
    }
  }
  if (document.documentElement.scrollWidth > innerWidth) misses.push(`overflow ${document.documentElement.scrollWidth}px`);
  if (document.querySelectorAll("h1").length !== 1) misses.push("h1 count");
  return { misses, counts };
}

const server = http.createServer((req, res) => {
  let file = path.join(out, decodeURIComponent(req.url.split("?")[0]));
  if (fs.existsSync(file) && fs.statSync(file).isDirectory()) file = path.join(file, "index.html");
  if (!fs.existsSync(file)) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { "content-type": types[path.extname(file)] || "application/octet-stream" });
  fs.createReadStream(file).pipe(res);
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const base = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ channel: process.env.AUDIT_BROWSER || "msedge", headless: true });
let misses = 0, edges = 0, rows = 0;
for (const width of widths) {
  const context = await browser.newContext({ viewport: { width, height: 900 }, reducedMotion: "reduce" });
  const page = await context.newPage();
  for (const url of pages) {
    await page.goto(base + url, { waitUntil: "networkidle" });
    await page.evaluate(() => document.fonts.ready.then(() => 1));
    const result = await page.evaluate(audit);
    edges += result.counts.edges; rows += result.counts.rows; misses += result.misses.length;
    for (const miss of result.misses.slice(0, 8)) console.error(`${url} @${width}: ${miss}`);
  }
  await context.close();
}
await browser.close();
server.close();
console.log(`${misses ? "FAIL" : "PASS"}: ${edges} edges, ${rows} rows, ${pages.length} pages at ${widths.length} widths, ${misses} misses`);
process.exit(misses ? 1 : 0);
