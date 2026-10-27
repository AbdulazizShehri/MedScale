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
