// Accessibility audit for the static export. Run after `next build`. On every page and
// width it tabs through the page and fails if a stop has no visible focus ring, text
// misses WCAG AA contrast, a control has no name, the heading or skip-link structure
// breaks, the page scrolls sideways, or motion runs under prefers-reduced-motion.
// Uses installed Microsoft Edge by default; AUDIT_BROWSER=chrome selects Chrome.
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { chromium } from "playwright-core";

const out = path.resolve("out");
const pages = ["/", "/product/", "/models/", "/download/", "/docs/", "/community/", "/404.html"];
const widths = [1440, 1024, 768, 375, 320];
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".png": "image/png", ".webp": "image/webp", ".woff2": "font/woff2", ".json": "application/json", ".txt": "text/plain" };

function structure() {
  const issues = [];
  const say = (text) => issues.push(text);
  const visible = (el) => {
    const r = el.getBoundingClientRect(), s = getComputedStyle(el);
    return r.width > 0 && r.height > 0 && s.visibility !== "hidden" && s.display !== "none";
  };
  const name = (el) => (el.getAttribute("aria-label") || el.innerText || el.getAttribute("title") || el.querySelector("img[alt]")?.alt || "").trim();
  if (document.querySelectorAll("h1").length !== 1) say(`${document.querySelectorAll("h1").length} h1 elements`);
  if (!document.querySelector("main#main")) say("no main#main");
  const skip = document.querySelector('a[href="#main"]');
  if (!skip) say("no skip link");
  let last = 0;
  for (const h of document.querySelectorAll("h1,h2,h3,h4")) {
    if (!visible(h)) continue;
    const level = Number(h.tagName[1]);
    if (last && level > last + 1) say(`heading jumps h${last} to h${level}: "${h.innerText.slice(0, 40)}"`);
    last = level;
  }
  for (const el of document.querySelectorAll("a[href],button,summary")) if (visible(el) && !name(el)) say(`unnamed ${el.tagName.toLowerCase()}.${el.className}`);
  for (const img of document.querySelectorAll("img")) if (!img.hasAttribute("alt")) say(`image without alt ${img.src.slice(-40)}`);
  if (document.documentElement.scrollWidth > innerWidth + 1) say(`sideways scroll ${document.documentElement.scrollWidth - innerWidth}px`);

  const parse = (c) => { const m = c.match(/[\d.]+/g)?.map(Number) ?? [0, 0, 0, 0]; return { r: m[0], g: m[1], b: m[2], a: m[3] ?? 1 }; };
  const lum = ({ r, g, b }) => { const f = (v) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }; return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b); };
  const over = (top, under) => ({ r: top.r * top.a + under.r * (1 - top.a), g: top.g * top.a + under.g * (1 - top.a), b: top.b * top.a + under.b * (1 - top.a), a: 1 });
  const background = (el) => {
    const stack = [];
    for (let n = el; n; n = n.parentElement) {
      const s = getComputedStyle(n);
      if (s.backgroundImage !== "none") return null; // Over an image or gradient: not measurable here.
      const c = parse(s.backgroundColor);
      if (c.a > 0) { stack.push(c); if (c.a === 1) break; }
    }
    return stack.reverse().reduce((under, top) => over(top, under), { r: 255, g: 255, b: 255, a: 1 });
  };
  const seen = new Set();
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const text = walker.currentNode.textContent.trim(), el = walker.currentNode.parentElement;
    if (!text || !el || !visible(el) || ["SCRIPT", "STYLE", "NOSCRIPT"].includes(el.tagName) || el.closest(".sr-only")) continue;
    const s = getComputedStyle(el);
    const bg = background(el);
    if (!bg) continue;
    const size = parseFloat(s.fontSize), large = size >= 24 || (size >= 18.66 && Number(s.fontWeight) >= 700);
    const ink = over(parse(s.color), bg);
    const a = lum(ink), b = lum(bg), ratio = (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
    const need = large ? 3 : 4.5;
    const key = `${el.className}|${s.color}`;
    if (ratio < need && !seen.has(key)) { seen.add(key); say(`contrast ${ratio.toFixed(2)} under ${need}: ${el.tagName.toLowerCase()}.${el.className} "${text.slice(0, 32)}"`); }
  }
  return issues;
}

function motion() {
  const moving = [];
  for (const el of document.querySelectorAll("*")) {
    const s = getComputedStyle(el);
    const seconds = (v) => Math.max(...v.split(",").map((x) => parseFloat(x) * (x.includes("ms") ? 0.001 : 1)));
    if (s.animationName !== "none" && seconds(s.animationDuration) > 0.05) moving.push(`animation ${el.tagName.toLowerCase()}.${el.className}`);
    if (seconds(s.transitionDuration) > 0.05) moving.push(`transition ${el.tagName.toLowerCase()}.${el.className}`);
  }
  return [...new Set(moving)].slice(0, 12);
}

const ring = () => {
  const el = document.activeElement;
  if (!el || el === document.body) return null;
  const s = getComputedStyle(el), r = el.getBoundingClientRect();
  return {
    label: `${el.tagName.toLowerCase()}.${el.className} "${(el.innerText || el.getAttribute("aria-label") || "").trim().slice(0, 28)}"`,
    shown: (s.outlineStyle !== "none" && parseFloat(s.outlineWidth) > 0) || s.boxShadow !== "none",
    onScreen: r.width > 0 && r.height > 0 && r.right > 0 && r.left < innerWidth,
    isSkip: el.getAttribute("href") === "#main",
  };
};

const server = http.createServer((req, res) => {
  let p = path.join(out, decodeURIComponent(req.url.split("?")[0]));
  if (fs.existsSync(p) && fs.statSync(p).isDirectory()) p = path.join(p, "index.html");
  if (!fs.existsSync(p)) { res.writeHead(404); return res.end(); }
  res.writeHead(200, { "content-type": types[path.extname(p)] ?? "application/octet-stream" });
  fs.createReadStream(p).pipe(res);
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch(process.env.AUDIT_BROWSER === "chrome" ? { channel: "chrome" } : { channel: "msedge" });
const found = [];
let stops = 0;
for (const route of pages) {
  for (const width of widths) {
    const page = await browser.newPage({ viewport: { width, height: 900 }, reducedMotion: "reduce" });
    await page.goto(origin + route, { waitUntil: "networkidle" });
    const note = (text) => found.push(`${route} @${width}: ${text}`);
    for (const issue of await page.evaluate(structure)) note(issue);
    for (const issue of await page.evaluate(motion)) note(`moves under reduced motion: ${issue}`);
    const visited = new Set();
    for (let i = 0; i < 120; i++) {
      await page.keyboard.press("Tab");
      const stop = await page.evaluate(ring);
      if (!stop) break;
      if (i === 0 && !stop.isSkip) note(`first Tab stop isn't the skip link: ${stop.label}`);
      if (visited.has(stop.label)) break;
      visited.add(stop.label);
      stops += 1;
      if (!stop.shown) note(`no focus ring: ${stop.label}`);
      if (!stop.onScreen) note(`focus off screen: ${stop.label}`);
    }
    await page.close();
  }
}
await browser.close();
server.close();
const unique = [...new Set(found)];
for (const line of unique) console.log(line);
console.log(`${unique.length ? "FAIL" : "PASS"}: ${pages.length} pages at ${widths.length} widths, ${stops} focus stops, ${unique.length} issues`);
process.exit(unique.length ? 1 : 0);
