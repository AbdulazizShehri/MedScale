import test from "node:test";
import assert from "node:assert/strict";
import {evaluateCareLoop, DEMO_SYSTEM} from "../src/care-loop.mjs";
import {makeSyntheticBundle} from "../fixtures/synthetic.mjs";

const fresh = () => structuredClone(makeSyntheticBundle());
const get = (b, type, id) => b.entry.find(e => e.resource.resourceType === type && e.resource.id === id).resource;
const remove = (b, type, id) => b.entry = b.entry.filter(e =>
  !(e.resource.resourceType === type && e.resource.id === id));

test("complete proof-linked simulated loop is closed", () => {
  const r=evaluateCareLoop(fresh());
  assert.equal(r.status,"closed");
  assert.equal(r.canClose,true);
  assert.equal(r.evidence.length,7);
  assert.equal(r.syntheticOnly,true);
  assert.deepEqual(r.warnings,[]);
});
test("no real-patient metadata is accepted", () => {
  const b=fresh(); delete get(b,"Patient","demo-001").meta;
  assert.equal(evaluateCareLoop(b).status,"blocked");
});
test("no positive screen gives unknown, never closure", () => {
  const b=fresh(); get(b,"Observation","screen-001").valueBoolean=false;
  assert.equal(evaluateCareLoop(b).status,"unknown");
});
test("no review stops after screening", () => {
  const b=fresh(); remove(b,"Task","review-001");
  assert.equal(evaluateCareLoop(b).status,"screened");
});
test("unconfirmed review fails closed", () => {
  const b=fresh(); get(b,"Task","review-001").status="in-progress";
  assert.equal(evaluateCareLoop(b).status,"blocked");
});
test("no referral stops at reviewed", () => {
  const b=fresh(); remove(b,"ServiceRequest","request-001");
  assert.equal(evaluateCareLoop(b).status,"reviewed");
});
test("missing appointment stops at referred", () => {
  const b=fresh(); remove(b,"Appointment","appointment-001");
  assert.equal(evaluateCareLoop(b).status,"referred");
});
test("cancelled appointment blocks closure", () => {
  const b=fresh(); get(b,"Appointment","appointment-001").status="cancelled";
  assert.equal(evaluateCareLoop(b).status,"blocked");
});
test("missing attendance triggers simulated overdue flag", () => {
  const b=fresh(); get(b,"Appointment","appointment-001").status="booked";
  remove(b,"Encounter","encounter-001");
  const r=evaluateCareLoop(b,{now:"2026-11-03T00:00:00Z"});
  assert.equal(r.status,"overdue");
  assert.equal(r.canClose,false);
});
test("future appointment without encounter stays pending", () => {
  const b=fresh(); get(b,"Appointment","appointment-001").start="2026-11-10T09:00:00Z";
  remove(b,"Encounter","encounter-001");
  assert.equal(evaluateCareLoop(b).status,"appointment-known");
});
test("unfinished encounter prevents closure", () => {
  const b=fresh(); get(b,"Encounter","encounter-001").status="in-progress";
  assert.equal(evaluateCareLoop(b).status,"blocked");
});
test("missing result stops at attended", () => {
  const b=fresh(); remove(b,"Observation","result-001");
  assert.equal(evaluateCareLoop(b).status,"attended");
});
test("wrong-patient result blocks", () => {
  const b=fresh(); get(b,"Observation","result-001").subject.reference="Patient/someone-else";
  assert.equal(evaluateCareLoop(b).status,"blocked");
});
test("missing reviewer provenance blocks", () => {
  const b=fresh(); remove(b,"Provenance","evidence-result-001");
  const r=evaluateCareLoop(b);
  assert.equal(r.status,"blocked");assert.match(r.warnings[0],/provenance/);
});
test("fake reviewer provenance blocks", () => {
  const b=fresh(); get(b,"Provenance","evidence-result-001").agent[0].who.reference="Practitioner/unverified";
  assert.equal(evaluateCareLoop(b).status,"blocked");
});
test("closure requires explicit completed task", () => {
  const b=fresh(); remove(b,"Task","close-001");
  const r=evaluateCareLoop(b);
  assert.equal(r.status,"result-received");assert.equal(r.canClose,false);
});
test("closure cannot refer to another result", () => {
  const b=fresh();get(b,"Task","close-001").focus.reference="Observation/another-result";
  assert.equal(evaluateCareLoop(b).status,"blocked");
});
test("duplicate resource IDs fail closed", () => {
  const b=fresh();b.entry.push(structuredClone(b.entry[1]));
  assert.equal(evaluateCareLoop(b).status,"blocked");
});
test("multiple patients fail closed", () => {
  const b=fresh();b.entry.push({resource:{resourceType:"Patient",id:"demo-002",meta:{tag:[{code:"synthetic"}]}}});
  assert.equal(evaluateCareLoop(b).status,"blocked");
});
test("ambiguous multiple screening observations fail closed", () => {
  const b=fresh();const extra=structuredClone(get(b,"Observation","screen-001"));extra.id="screen-002";
  b.entry.push({resource:extra});
  assert.equal(evaluateCareLoop(b).status,"blocked");
});
test("invalid timestamp and malformed bundles fail closed", () => {
  assert.equal(evaluateCareLoop(fresh(),{now:"not-a-timestamp"}).status,"blocked");
  assert.equal(evaluateCareLoop({resourceType:"Bundle",entry:[]}).status,"blocked");
});
test("unrelated code system cannot trigger screening", () => {
  const b=fresh();get(b,"Observation","screen-001").code.coding[0].system="http://example.invalid/clinical";
  assert.notEqual(evaluateCareLoop(b).status,"closed");
});
