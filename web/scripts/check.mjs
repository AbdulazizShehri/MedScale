// Static-export verification for the MedScale website. Run after `next build`.
// Checks routes, brand and font assets, disclosures, the release gate, and that
// every internal link and asset resolves inside the export for the configured
// base path (NEXT_PUBLIC_SITE_BASE_PATH, "" for a root domain).
import fs from "node:fs";
import path from "node:path";
import { QUALIFIED_RELEASE_TAGS, formatSize, releaseAssetUrl, validateRelease } from "../src/lib/release.ts";

const root = process.cwd();
const out = path.join(root, "out");
const base = process.env.NEXT_PUBLIC_SITE_BASE_PATH || "";
const failures = [];
const check = (ok, msg) => { if (!ok) failures.push(msg); };
const read = (p) => fs.readFileSync(p, "utf8");

check(fs.existsSync(out), "next export output missing (run next build first)");
const pages = ["index.html", "product/index.html", "models/index.html", "download/index.html", "docs/index.html", "community/index.html", "404.html"];
for (const p of pages) check(fs.existsSync(path.join(out, p)), `missing page ${p}`);
for (const a of ["brand/medscale-horizontal-white.svg", "brand/mark-white.svg", "brand/medscale-favicon.svg", "brand/apple-touch-icon.png", "og.png", "fonts/Inter-OFL.txt", "fonts/JetBrainsMono-OFL.txt"])
  check(fs.existsSync(path.join(out, a)), `missing asset ${a}`);

// The website logo must be the brand master, byte for byte.
const master = path.join(root, "..", "assets", "brand", "medscale-horizontal-white.svg");
if (fs.existsSync(master)) check(read(master) === read(path.join(root, "public/brand/medscale-horizontal-white.svg")), "web logo differs from assets/brand master");

// Release gate and download honesty.
const fixture = process.argv.includes("--fixture");
const raw = JSON.parse(read(path.join(root, "src/lib/release-manifest.json")));
const verdict = validateRelease(raw, fixture ? ["v0.0.0-beta.0"] : QUALIFIED_RELEASE_TAGS);
check(verdict.errors.length === 0, `release manifest refused: ${verdict.errors.join("; ")}`);
const offered = Object.values(verdict.release?.platforms ?? {});
if (!fixture) {
  check(raw.release === null || !String(raw.release.tag).startsWith("v0.0.0"), "the fixture release must never ship");
  check((raw.release === null) === (QUALIFIED_RELEASE_TAGS.length === 0), "release gate and manifest must change together");
}
const allowedAssets = new Set(offered.flatMap((p) => [p.url, ...Object.values(p.companions).map((f) => releaseAssetUrl(verdict.release.tag, f))]));

// Exact host and repository: a prefix match would accept medscale.si.evil.example.
const INSTALLER = /\.(exe|msi|dmg|pkg|appimage|deb|rpm)$/i;
function checkExternal(rel, href) {
  let url;
  try { url = new URL(href); } catch { check(false, `${rel}: unparsable link ${href}`); return; }
  const repo = /^\/AbdulazizShehri\/MedScale(\/|$)/.test(url.pathname);
  const allowed = url.protocol === "https:" && (
    (url.hostname === "github.com" && (repo || /^\/sponsors\/AbdulazizShehri(\/|$)/.test(url.pathname))) ||
    (url.hostname === "abdulazizshehri.github.io" && /^\/MedScale(\/|$)/.test(url.pathname)) ||
    url.hostname === "medscale.si");
  check(allowed, `${rel}: unexpected external link ${href}`);
  const isAsset = url.pathname.includes("/releases/download/") || url.pathname.includes("/releases/latest") || INSTALLER.test(url.pathname);
  if (isAsset && !allowedAssets.has(href)) check(false, `${rel}: links an unapproved release asset ${href}`);
}

const files = [];
(function walk(d) { for (const e of fs.readdirSync(d, { withFileTypes: true })) { const f = path.join(d, e.name); e.isDirectory() ? walk(f) : f.endsWith(".html") && files.push(f); } })(out);

const resolve = (href) => {
  let p = href.split("#")[0].split("?")[0];
  if (base && !p.startsWith(base + "/") && p !== base) return null;
  p = p.slice(base.length) || "/";
  const target = path.join(out, decodeURIComponent(p));
  return [target, path.join(target, "index.html"), target + ".html"].some((t) => fs.existsSync(t) && fs.statSync(t).isFile());
};

for (const f of files) {
  const rel = path.relative(out, f);
  const html = read(f);
  check(html.includes('<html lang="en"'), `${rel}: missing lang`);
  check(html.includes('id="main"'), `${rel}: missing skip-link target`);
  check(!/<script[^>]+src="https?:/i.test(html), `${rel}: loads a remote script`);
  check(![...html.matchAll(/<link[^>]+>/gi)].some(([tag]) => /rel="(stylesheet|preload|modulepreload|preconnect)"/i.test(tag) && /href="https?:/i.test(tag)), `${rel}: loads a remote stylesheet or preload`);
  check(!/fonts\.googleapis|fonts\.gstatic/i.test(html), `${rel}: uses a remote font service`);
  for (const [, href] of html.matchAll(/(?:href|src)="([^"]+)"/g)) {
    if (/^(https?:|mailto:|#|data:)/.test(href)) {
      if (/^https?:/.test(href)) checkExternal(rel, href.replaceAll("&amp;", "&"));
      continue;
    }
    if (!href.startsWith("/")) continue;
    check(resolve(href) === true, `${rel}: broken internal link ${href}${base && !href.startsWith(base) ? " (missing base path)" : ""}`);
  }
}

// Client code can link too, and URLs also sit in the bundles.
const scripts = [];
(function walk(d) { for (const e of fs.readdirSync(d, { withFileTypes: true })) { const f = path.join(d, e.name); e.isDirectory() ? walk(f) : /\.(js|txt)$/.test(f) && scripts.push(f); } })(out);
for (const f of scripts) {
  const rel = path.relative(out, f);
  const body = read(f);
  check(!/api\.github\.com|releases\/latest/i.test(body), `${rel}: depends on a mutable release lookup`);
  for (const [href] of body.matchAll(/https:\/\/[A-Za-z0-9.-]+\/[^\s"'`\\<>]*\/releases\/download\/[^\s"'`\\<>]*/g)) {
    // The compiled releaseAssetUrl template is code, not a link.
    if (!href.includes("${")) check(allowedAssets.has(href), `${rel}: names an unapproved release asset ${href}`);
  }
}

const home = read(path.join(out, "index.html"));
const product = read(path.join(out, "product/index.html"));
// React separates adjacent text with comment markers in the export.
const dl = read(path.join(out, "download/index.html")).replaceAll("<!-- -->", "");
check(home.includes("synthetic workspaces + licensed nonpatient audio"), "home: accurate preview data scope missing");
check(home.includes("real patient records, clinical decisions and production deployments are not authorized"), "home: patient/clinical restriction missing");
check(product.includes("Synthetic workspaces and user-approved licensed nonpatient audio"), "product: supported nonpatient audio disclosure missing");
check(product.includes("Real patient data and clinical decisions are not authorized"), "product: patient/clinical restriction missing");
check(!/synthetic data only/i.test(home + product + dl), "site: inaccurate synthetic-only product claim reintroduced");
check(home.includes("Real screenshot"), "home: screenshot provenance label missing");
check(home.includes("Interactive example"), "home: interactive example label missing");
if (offered.length === 0) {
  check(dl.includes("There is no qualified public installer yet"), "download: release status missing");
  check(!/Download for (Windows|Mac)/.test(dl), "download: offers an installer with no release");
} else {
  for (const p of offered) {
    check(dl.includes(p.url), `download: missing link to ${p.fileName}`);
    check(dl.includes(p.sha256), `download: missing SHA-256 for ${p.fileName}`);
    check(dl.includes(formatSize(p.size)), `download: missing size for ${p.fileName}`);
  }
  check(dl.includes(`MedScale ${verdict.release.version} Beta`), "download: version label missing");
  check(dl.includes(verdict.release.sourceSha), "download: source commit missing");
  check(dl.includes("Report a problem"), "download: support link missing");
  check(dl.includes("Real patient records and clinical decisions aren"), "download: patient data restriction missing");
  check(!dl.includes("Signed with Apple Developer ID") || offered.some((p) => p.signing === "developer-id-notarized"), "download: claims notarization no installer has");
}
check(dl.includes("Not a production release"), "download: production disclaimer missing");

if (failures.length) { console.error("FAIL\n- " + failures.join("\n- ")); process.exit(1); }
console.log(`PASS: ${files.length} pages, base path "${base || "/"}", internal links and assets resolve, release gate ${offered.length ? "open for " + verdict.release.tag : "closed"}, disclosures present`);
