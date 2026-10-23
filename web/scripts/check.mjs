// Static-export verification for the MedScale website. Run after `next build`.
// Checks routes, brand and font assets, disclosures, the release gate, and that
// every internal link and asset resolves inside the export for the configured
// base path (NEXT_PUBLIC_SITE_BASE_PATH, "" for a root domain).
import fs from "node:fs";
import path from "node:path";

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
const release = read(path.join(root, "src/lib/release.ts"));
check(/QUALIFIED_RELEASE_TAGS: readonly string\[\] = \[\];/.test(release), "release gate must stay empty until a signed, qualified release exists");

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
  // Explicitly published, pinned synthetic-only Beta assets are allowed here.
  // The qualified signed-release gate above remains empty; reject all other installer URLs.
  const betaPrefix = "https://github.com/AbdulazizShehri/MedScale/releases/download/engineering-preview-19d419dd7b15/";
  const allowedBetaInstallers = new Set([betaPrefix + "MedScale-engineering-preview-windows-x64.exe", betaPrefix + "MedScale-engineering-preview-macos.dmg"]);
  for (const [, installer] of html.matchAll(/href="([^"]+\.(?:exe|msi|dmg|pkg|appimage|deb|rpm))"/gi)) {
    check(allowedBetaInstallers.has(installer), `${rel}: unapproved installer link ${installer}`);
  }
  for (const [, href] of html.matchAll(/(?:href|src)="([^"]+)"/g)) {
    if (/^(https?:|mailto:|#|data:)/.test(href)) {
      if (/^https?:/.test(href)) check(/^https:\/\/(github\.com\/AbdulazizShehri\/MedScale|abdulazizshehri\.github\.io\/MedScale|medscale\.si)/.test(href), `${rel}: unexpected external link ${href}`);
      continue;
    }
    if (!href.startsWith("/")) continue;
    check(resolve(href) === true, `${rel}: broken internal link ${href}${base && !href.startsWith(base) ? " (missing base path)" : ""}`);
  }
}

const home = read(path.join(out, "index.html"));
const dl = read(path.join(out, "download/index.html"));
check(home.includes("synthetic data only"), "home: preview disclosure missing");
check(home.includes("Real screenshot"), "home: screenshot provenance label missing");
check(home.includes("Interactive example"), "home: interactive example label missing");
check(dl.includes("There is no signed public installer yet"), "download: release status missing");
check(dl.includes("Not a production release"), "download: production disclaimer missing");

if (failures.length) { console.error("FAIL\n- " + failures.join("\n- ")); process.exit(1); }
console.log(`PASS: ${files.length} pages, base path "${base || "/"}", internal links and assets resolve, release gate closed, disclosures present`);
