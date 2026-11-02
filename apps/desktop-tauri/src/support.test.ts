import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { SUPPORT_DATA_RULE, SUPPORT_DIAGNOSTIC_KEYS, SUPPORT_ISSUES_URL, supportDiagnostics } from "./support.ts";

const supportMd = fs.readFileSync(new URL("../../../SUPPORT.md", import.meta.url), "utf8");

test("support URL and data rule stay in sync with SUPPORT.md", () => {
  assert.ok(supportMd.includes(`(${SUPPORT_ISSUES_URL})`), "SUPPORT.md no longer links the issue chooser");
  assert.ok(supportMd.includes(`**${SUPPORT_DATA_RULE}**`), "SUPPORT.md data rule changed; update support.ts");
});

test("diagnostics expose exactly the allowlisted keys and no path-like values", () => {
  const text = supportDiagnostics({
    version: "0.1.0", build_profile: "release", target: "windows-x86_64",
    shell: "Tauri 2", local_only: true, network_default_deny: true,
  });
  const keys = text.trimEnd().split("\n").map((line) => line.slice(0, line.indexOf(":")));
  assert.deepEqual(keys, [...SUPPORT_DIAGNOSTIC_KEYS]);
  for (const forbidden of ["/", "\\", "@", ":\\"]) assert.ok(!text.includes(forbidden), `diagnostics contain ${forbidden}`);
  assert.match(text, /Network: default deny \(consented model downloads excepted\)/);
});

test("diagnostics values cannot inject extra lines", () => {
  const text = supportDiagnostics({
    version: "1\nExtra: injected", build_profile: "release", target: "t", shell: "s", local_only: false, network_default_deny: false,
  });
  assert.equal(text.trimEnd().split("\n").length, SUPPORT_DIAGNOSTIC_KEYS.length);
});
