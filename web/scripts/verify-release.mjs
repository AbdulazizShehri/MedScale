// Confirms the release built into the site matches the published bytes, downloading
// without credentials. Run before enabling or changing a download link.
// With no release in the manifest there is nothing to fetch, so it makes no request.
import fs from "node:fs";
import path from "node:path";
import { validateRelease } from "../src/lib/release.ts";
import { verifyPlatformAssets, verifyReleaseRecords } from "../src/lib/release-verify.ts";

const raw = JSON.parse(fs.readFileSync(path.join(process.cwd(), "src/lib/release-manifest.json"), "utf8"));
const { release, errors } = validateRelease(raw);
const failures = [...errors];
if (release) {
  failures.push(...await verifyReleaseRecords(release));
  for (const key of Object.keys(release.platforms)) failures.push(...await verifyPlatformAssets(release, key));
}
if (failures.length) { console.error("FAIL\n- " + failures.join("\n- ")); process.exit(1); }
console.log(release ? `PASS: ${release.tag} assets match for ${Object.keys(release.platforms).join(", ")}` : "PASS: no release in the manifest, nothing to verify");
