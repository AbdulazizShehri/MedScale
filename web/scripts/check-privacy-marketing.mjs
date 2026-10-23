import fs from "node:fs";
import path from "node:path";

const root = path.resolve(process.cwd(), "..");
const read = (file) => fs.readFileSync(path.join(root, file), "utf8");
const page = (route) => read("web/out/" + (route ? route + "/" : "") + "index.html");
const home = page("");
const product = page("product");
const download = page("download");
const site = [home, product, download].join("\n");
const failures = [];
const verify = (pass, description) => { if (!pass) failures.push(description); };

for (const [name, html] of [["home", home], ["product", product], ["download", download]]) {
  verify(/free to use/i.test(html), name + ": missing free-to-use promise");
  verify(/locall?y|on your computer|on your desktop|local processing/i.test(html), name + ": missing local-processing promise");
  verify(/no automatic (?:workspace |data )?upload/i.test(html), name + ": missing bounded no-upload promise");
}
verify(home.includes('id="privacy"'), "home: privacy proof section absent");
verify(home.includes("No automatic cloud handoff"), "home: default-local message absent");
verify(home.includes("approved browsing, exports, and external integrations can transmit data"), "home: exceptions to no-upload promise absent");
verify(/synthetic data only/i.test(site), "synthetic Beta boundary missing");
verify(download.includes("not a qualified production or medical release"), "download: release caveat absent");
verify(/Apache-2.0/.test(home), "home: missing software licence source");
verify(home.includes("TRUST_AND_PRIVACY_MODEL.md"), "home: missing privacy-model source link");
verify(/does not automatically send workspace data, product telemetry, or crash reports/.test(home), "home: bounded telemetry disclosure missing");
verify(!/patient data never leaves (?:the )?(?:device|computer)|100% private|HIPAA.compliant/i.test(site), "unqualified privacy or compliance claim exposed");

const license = read("LICENSE");
const readme = read("README.md");
const trust = read("docs/security/TRUST_AND_PRIVACY_MODEL.md");
verify(license.includes("Apache License") && license.includes("Version 2.0"), "source: Apache-2.0 licence absent");
verify(readme.includes("network actions are explicitly governed and default-denied"), "source: network policy no longer substantiated");
verify(trust.includes("No product telemetry or remote crash upload by default."), "source: telemetry guardrail no longer substantiated");

if (failures.length) {
  console.error("Privacy marketing integrity FAIL:\n" + failures.map((f) => " - " + f).join("\n"));
  process.exit(1);
}
console.log("PASS: free/local/no-automatic-upload message on three pages, guarded network caveats, Beta disclosure, license and policy evidence");
