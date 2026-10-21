// Product-identity guard (issue #182). Scans product-facing surfaces for stale
// or third-party-centred wording and checks that required license/provenance
// records exist. Historical specs, evidence and planning records are exempt:
// they document source history and must not be rewritten.
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const repo = new URL("../../../", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const read = (p: string) => readFileSync(join(repo, p), "utf8");

function walk(dir: string): string[] {
  return readdirSync(join(repo, dir)).flatMap((name) => {
    const rel = join(dir, name);
    return statSync(join(repo, rel)).isDirectory() ? walk(rel) : [rel];
  });
}

// Product-facing surfaces: the README intro (before the engineering section),
// the desktop UI source, the HTML shell and the bundle metadata.
function productSurfaces(): Array<[string, string]> {
  const readme = read("README.md");
  const intro = readme.slice(0, readme.indexOf("## Working on MedScale"));
  const ui = walk("apps/desktop-tauri/src")
    .filter((p) => /\.(tsx?|css|html)$/.test(p) && !p.endsWith(".test.ts"))
    .map((p): [string, string] => [p, read(p)]);
  return [
    ["README.md (product intro)", intro],
    ["apps/desktop-tauri/index.html", read("apps/desktop-tauri/index.html")],
    ["apps/desktop-tauri/src-tauri/tauri.conf.json", read("apps/desktop-tauri/src-tauri/tauri.conf.json")],
    ...ui,
  ];
}

const STALE: Array<[RegExp, string]> = [
  [/UI_VISUAL_SOURCE\s*=\s*v0/i, "stale v0 visual-source marker"],
  [/strategic donor|capability floor/i, "third-party-centred positioning"],
  [/\bTauri App\b|vite\.svg|Vite \+ React|create-vite/i, "generator boilerplate"],
  [/ScaleFold/i, "superseded identity (#125)"],
  [/paired[- ]M\b|M0 76 50 0 100 76Z|M0 86\.60254/, "superseded paired-M logo (Logo 12, 2026-10-19)"],
  [/lorem ipsum/i, "placeholder text"],
  [/\b\d[\d,]* (?:local |)models (?:run|available) locally\b/i, "bare model-count claim (Spec 103 R11)"],
];

test("product-facing surfaces carry no stale or third-party-centred identity", () => {
  const hits: string[] = [];
  for (const [file, text] of productSurfaces()) {
    for (const [pattern, why] of STALE) if (pattern.test(text)) hits.push(`${file}: ${why}`);
  }
  assert.deepEqual(hits, []);
});

test("product name and window title are MedScale", () => {
  const conf = JSON.parse(read("apps/desktop-tauri/src-tauri/tauri.conf.json"));
  assert.equal(conf.productName, "MedScale");
  assert.ok(conf.app.windows.every((w: { title: string }) => w.title === "MedScale"));
  assert.match(read("apps/desktop-tauri/index.html"), /<title>MedScale[^<]*<\/title>/);
});

test("MedScale catalog and model evaluation do not brand product features as competitors", () => {
  const models = read("apps/desktop-tauri/src/pages/Intelligence.tsx");
  assert.match(models, />MedScale Catalog<\/button>/);
  assert.doesNotMatch(models, />Catalog\s*<span[^>]*>OpenMed/i);
  assert.doesNotMatch(models, /MedScale vs OpenMed · static ledger/);
  assert.doesNotMatch(models, /OpenMed-compatible \(ONNX decoder\)/);
  // Preserve exact repository IDs in the Core data contract; display only a friendly alias.
  assert.match(models, /catalogDisplayName\(r\.repo_id\)/);
  assert.match(models, /p\.commit\.slice\(0, 12\)/);
  assert.doesNotMatch(models, />\{p\.repository\} @/);
});

test("legacy reference presentation uses MedScale labels for the evidence workspace", () => {
  const slint = read("crates/medscale-desktop/ui/app.slint");
  assert.match(slint, /MedScale Evidence Center/);
  assert.match(slint, /MedScale evidence ledger/);
  assert.doesNotMatch(slint, /MedScale × OpenMed/);
  assert.doesNotMatch(slint, /Pinned OpenMed evidence/);
});
test("license, provenance and release notice sources are present", () => {
  assert.ok(existsSync(join(repo, "LICENSE")));
  assert.match(read("LICENSE"), /Apache License/);
  assert.ok(walk("third_party/provenance").length > 0);
  assert.ok(existsSync(join(repo, "assets/brand/fonts/Inter-OFL.txt")));
  assert.ok(existsSync(join(repo, "assets/brand/fonts/JetBrainsMono-OFL.txt")));
  // NOTICE.md and the SBOM are generated into every release set and checked by
  // scripts/verify-tauri-release.ps1; their generators must exist.
  assert.ok(existsSync(join(repo, "scripts/package-tauri-preview.ps1")));
  assert.ok(existsSync(join(repo, "scripts/package-tauri-release.ps1")));
});
