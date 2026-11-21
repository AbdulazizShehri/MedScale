import test from "node:test";
import assert from "node:assert/strict";
import {evaluateCareLoop} from "../src/care-loop.mjs";
import {makeSyntheticBundle} from "../fixtures/synthetic.mjs";

const key = (r) => r.resourceType + "/" + r.id;
const stageKeys = [
  "Observation/screen-001",
  "Task/review-001",
  "ServiceRequest/request-001",
  "Appointment/appointment-001",
  "Encounter/encounter-001",
  "Observation/result-001",
  "Task/close-001",
];
const get = (bundle, reference) =>
  bundle.entry.find((e) => key(e.resource) === reference)?.resource;
const remove = (bundle, reference) =>
  bundle.entry = bundle.entry.filter((e) => key(e.resource) !== reference);
const proof = (reference) =>
  "Provenance/evidence-" + reference.split("/")[1];
const asSynthetic = () => structuredClone(makeSyntheticBundle());

test("each stage and each reviewer provenance is independently necessary", () => {
  for (const reference of stageKeys) {
    const b = asSynthetic();
    remove(b, reference);
    const result = evaluateCareLoop(b);
    assert.equal(result.canClose, false, reference + " deletion allowed false closure");
    assert.notEqual(result.status, "closed");
  }
  for (const reference of stageKeys) {
    const b = asSynthetic();
    remove(b, proof(reference));
    const result = evaluateCareLoop(b);
    assert.equal(result.canClose, false, reference + " reviewer proof deletion allowed false closure");
    assert.notEqual(result.status, "closed");
  }
});

test("300 seeded mutations of required synthetic evidence never falsely close", () => {
  let seed = 0x6d656473;
  const rand = () => ((seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0) / 0x100000000);
  const choose = (items) => items[Math.floor(rand() * items.length)];
  for (let trial = 0; trial < 300; trial++) {
    const b = asSynthetic();
    const critical = choose(stageKeys);
    const mode = trial % 6;
    if (mode === 0) {
      remove(b, critical);
    } else if (mode === 1) {
      get(b, proof(critical)).target = [{reference:"Patient/wrong-evidence-target"}];
    } else if (mode === 2) {
      get(b, proof(critical)).agent = [{who:{reference:"Practitioner/unverified"}}];
    } else if (mode === 3) {
      get(b, proof(critical)).recorded = "bad-time-format";
    } else if (mode === 4) {
      get(b, critical).id += "-tampered";
    } else {
      get(b, "Patient/demo-001").identifier = [{system:"urn:untrusted", value:"identifying"}];
    }
    const result = evaluateCareLoop(b);
    assert.equal(result.canClose, false, "trial " + trial + " mode " + mode + " " + critical);
    assert.notEqual(result.status, "closed");
  }
});
