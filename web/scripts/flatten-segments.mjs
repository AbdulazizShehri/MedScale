// Next.js 16 static export writes per-segment prefetch payloads as nested files
// (product/__next.product/__PAGE__.txt), while the client router requests the
// flat name (product/__next.product.__PAGE__.txt). Static hosts such as GitHub
// Pages then answer every prefetch with 404. This writes the flat-named copy
// next to each nested payload. It runs after `next build`.
import fs from "node:fs";
import path from "node:path";

const out = path.join(process.cwd(), "out");
let written = 0;

function walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (!entry.isDirectory()) continue;
    if (entry.name.startsWith("__next.")) flatten(full, path.dirname(full), entry.name);
    else walk(full);
  }
}

function flatten(dir, parent, prefix) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    const name = `${prefix}.${entry.name}`;
    if (entry.isDirectory()) flatten(full, parent, name);
    else if (!fs.existsSync(path.join(parent, name))) {
      fs.copyFileSync(full, path.join(parent, name));
      written += 1;
    }
  }
}

if (!fs.existsSync(out)) throw new Error("out/ missing: run next build first");
walk(out);
console.log(`flatten-segments: wrote ${written} flat prefetch payload(s)`);
