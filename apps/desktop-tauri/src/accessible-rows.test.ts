// A row that takes keyboard focus has no name of its own unless it's given one:
// Windows UI Automation and macOS Accessibility reported these as blank.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";

const pages = new URL("./pages/", import.meta.url);

test("every focusable row names itself", () => {
  const unnamed: string[] = [];
  let rows = 0;
  for (const file of readdirSync(pages).filter((f) => f.endsWith(".tsx"))) {
    readFileSync(new URL(file, pages), "utf8").split("\n").forEach((line, i) => {
      if (!/<(tr|div)\b/.test(line) || !line.includes("tabIndex={0}")) return;
      rows += 1;
      if (!line.includes("aria-label=")) unnamed.push(`${file}:${i + 1}`);
    });
  }
  assert.ok(rows >= 8, `the scan found only ${rows} rows, so it may have stopped matching`);
  assert.deepEqual(unnamed, []);
});
