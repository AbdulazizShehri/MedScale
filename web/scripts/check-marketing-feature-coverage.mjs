import fs from "node:fs";
import path from "node:path";

const project = path.resolve(process.cwd(), "..");
const read = (p) => fs.readFileSync(path.join(project, p), "utf8");
const source = read("apps/desktop-tauri/src/App.tsx");
const catalog = read("web/src/components/FeatureAtlas.tsx");
const home = read("web/out/index.html");
const product = read("web/out/product/index.html");

const routes = [...source.matchAll(/^  (?:"([^"]+)"|([A-Za-z]+)): \(\) =>/gm)]
  .map((m) => m[1] ?? m[2])
  .filter((name) => name !== "Home");
const promoted = [...catalog.matchAll(/\{ name: "([^"]+)", detail: "/g)]
  .map((m) => m[1]);
const missing = routes.filter((name) => !promoted.includes(name));
const extra = promoted.filter((name) => !routes.includes(name));
const duplicate = promoted.filter((name, i) => promoted.indexOf(name) !== i);
const failures = [];
const assert = (pass, label) => { if (!pass) failures.push(label); };

assert(routes.length === 24, `expected 24 implemented desktop areas, saw ${routes.length}`);
assert(promoted.length === 24, `expected 24 marketed desktop areas, saw ${promoted.length}`);
assert(!missing.length && !extra.length && !duplicate.length,
  `desktop routes and marketed areas differ: missing=${missing}, extra=${extra}, duplicate=${duplicate}`);
for (const [page, html] of [["homepage", home], ["product", product]]) {
  for (const name of routes) assert(html.includes(`<strong>${name}</strong>`), `${page} is missing ${name}`);
  assert(html.includes("brand/mark-white.svg"), `${page} missing approved symbol`);
  assert(html.includes("brand/medscale-wordmark-white.svg"), `${page} missing approved wordmark`);
  assert(html.includes("synthetic"), `${page} missing Beta synthetic boundary`);
}
assert(home.includes("id=\"features\""), "homepage missing feature overview anchor");
assert(product.includes("id=\"all-features\""), "product missing full feature guide anchor");

if (failures.length) {
  console.error("Marketing integrity FAIL:\n" + failures.map((f) => "- " + f).join("\n"));
  process.exit(1);
}
console.log(`PASS: ${routes.length} marketed desktop areas exactly match implemented routes across both pages; approved symbol + wordmark and Beta disclosure preserved`);
