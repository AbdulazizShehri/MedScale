import assert from "node:assert/strict";
import test from "node:test";
import { formatStorage } from "./model-storage.ts";

test("model preflight formats binary units consistently", () => {
  assert.equal(formatStorage(200 * 1024 * 1024), "200 MiB");
  assert.equal(formatStorage(512 * 1024 * 1024), "512 MiB");
});
test("invalid disk metrics are never displayed as capacity", () => {
  assert.equal(formatStorage(-1), "Unknown");
  assert.equal(formatStorage(Number.NaN), "Unknown");
  assert.equal(formatStorage(1.2), "Unknown");
});

test("catalog decimal megabytes switch to GB at 1000 MB and reject invalid sizes", async () => {
  const { formatDecimalMegabytes } = await import("./model-storage.ts");
  assert.equal(formatDecimalMegabytes(11_444), "11.4 GB");
  assert.equal(formatDecimalMegabytes(999.6), "1000 MB");
  assert.equal(formatDecimalMegabytes(151), "151 MB");
  assert.equal(formatDecimalMegabytes(null), "—");
  assert.equal(formatDecimalMegabytes(-1), "—");
  assert.equal(formatDecimalMegabytes(Number.NaN), "—");
});
