// After the founder publishes a release: writes src/lib/release-manifest.json
// from the candidate's artifact-inventory.json and adds the tag to
// QUALIFIED_RELEASE_TAGS, so the download page lists exactly those files.
// It changes only these two files and fetches nothing; `npm run
// verify-release` then checks every listed file against the published bytes.
//
//   node --experimental-strip-types scripts/manifest-from-candidate.mjs \
//     <artifact-inventory.json> <tag> <approval-comment-url> [signing-comment-url]
import fs from "node:fs";
import path from "node:path";
import { checkedManifest } from "../src/lib/release-manifest-gen.ts";

const [inventoryPath, tag, founderApproval, signingDecision] = process.argv.slice(2);
if (!inventoryPath || !tag || !founderApproval) {
  console.error("usage: manifest-from-candidate.mjs <artifact-inventory.json> <tag> <approval-comment-url> [signing-comment-url]");
  process.exit(2);
}
const inventory = JSON.parse(fs.readFileSync(inventoryPath, "utf8"));
const manifest = checkedManifest(inventory, { tag, founderApproval, signingDecision });

const root = process.cwd();
fs.writeFileSync(path.join(root, "src/lib/release-manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
const releaseTs = path.join(root, "src/lib/release.ts");
const source = fs.readFileSync(releaseTs, "utf8");
const gate = /export const QUALIFIED_RELEASE_TAGS: readonly string\[\] = \[([^\]]*)\];/;
const match = gate.exec(source);
if (!match) throw new Error("QUALIFIED_RELEASE_TAGS not found in release.ts");
const tags = match[1].split(",").map((t) => t.trim()).filter(Boolean);
if (!tags.includes(JSON.stringify(tag))) tags.push(JSON.stringify(tag));
fs.writeFileSync(releaseTs, source.replace(gate, `export const QUALIFIED_RELEASE_TAGS: readonly string[] = [${tags.join(", ")}];`));
console.log(`Wrote the ${tag} manifest (${Object.keys(manifest.release.platforms).length} downloads) and qualified the tag.`);
console.log("Next: npm run verify-release, then npm run build && npm run check.");
