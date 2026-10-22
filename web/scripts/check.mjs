import fs from "node:fs";
import path from "node:path";
const root=process.cwd();
const src=fs.readFileSync(path.join(root,"src/app/page.tsx"),"utf8");
const downloads=fs.readFileSync(path.join(root,"src/components/DownloadCards.tsx"),"utf8");
const all=fs.readFileSync(path.join(root,"src/lib/site.ts"),"utf8");
const out=path.join(root,"out");
function invariant(ok,msg){if(!ok)throw new Error(msg)}
invariant(all.includes("released = false"),"Release gate must remain false");
invariant(downloads.includes("SIGNED RELEASE NOT AVAILABLE"),"Missing download honesty");
invariant(src.includes("CONCEPTUAL")===false || src.includes("PRODUCT"),"Home intro not found");
invariant(fs.existsSync(out),"Next export output missing");
for(const page of ["index.html","download/index.html","features/index.html","docs/index.html","community/index.html","404.html"]) invariant(fs.existsSync(path.join(out,page)),"Missing exported page "+page);
for(const asset of ["medscale-horizontal-white.svg","mark-white.svg","medscale-favicon.svg"]) invariant(fs.existsSync(path.join(out,"brand",asset)),"Missing brand master "+asset);
const home=fs.readFileSync(path.join(out,"index.html"),"utf8");
const dl=fs.readFileSync(path.join(out,"download/index.html"),"utf8");
invariant(home.includes("Not for clinical use."),"Mandatory clinical disclaimer missing");
invariant(home.includes("NOT A REAL CLINICAL RECORD"),"Hero synthetic disclosure missing");
invariant(dl.includes("SIGNED RELEASE NOT AVAILABLE"),"Download disclaimer missing");
invariant(!/href=["']https?:\/\/[^"']+\.(exe|dmg|appimage|msi)["']/i.test(dl),"Unverified installer URL in download page");
for(const file of ["index.html","download/index.html","features/index.html","docs/index.html","community/index.html"]){const html=fs.readFileSync(path.join(out,file),"utf8");invariant(html.includes('<html lang="en"'),file+" missing lang");invariant(html.includes('id="main"'),file+" missing skip-link target")}
console.log("PASS: Next static export, routes, approved assets, review disclosures and release integrity");
