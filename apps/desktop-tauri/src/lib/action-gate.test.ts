import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { createSingleFlightGate } from "./action-gate.ts";

test("two actions cannot start before the UI's pending state renders", () => {
  const gate = createSingleFlightGate();
  assert.equal(gate.tryEnter(), true);
  assert.equal(gate.isOccupied(), true);
  assert.equal(gate.tryEnter(), false);
  assert.equal(gate.isOccupied(), true);
  gate.leave();
  assert.equal(gate.isOccupied(), false);
  assert.equal(gate.tryEnter(), true);
});

test("each component gets an independent action gate", () => {
  const a = createSingleFlightGate();
  const b = createSingleFlightGate();
  assert.equal(a.tryEnter(), true);
  assert.equal(b.tryEnter(), true);
  a.leave();
  assert.equal(b.isOccupied(), true);
  assert.equal(b.tryEnter(), false);
});

test("rejected second action never clears the active action", async () => {
  const gate = createSingleFlightGate();
  let releaseFirst!: () => void;
  const firstMayFinish = new Promise<void>((resolve) => { releaseFirst = resolve; });
  const first = (async () => {
    assert.equal(gate.tryEnter(), true);
    try { await firstMayFinish; } finally { gate.leave(); }
  })();
  assert.equal(gate.tryEnter(), false);
  assert.equal(gate.isOccupied(), true);
  releaseFirst();
  await first;
  assert.equal(gate.tryEnter(), true);
});

test("the production hook acquires the gate before native invoke", () => {
  const source = readFileSync(new URL("./ipc.ts", import.meta.url), "utf8");
  assert.match(source, /const gate = useRef\(createSingleFlightGate\(\)\)/);
  assert.match(source, /if \(!gate\.current\.tryEnter\(\)\)/);
  assert.match(source, /gate\.current\.leave\(\)/);
});
