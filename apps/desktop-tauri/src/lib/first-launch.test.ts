import assert from "node:assert/strict";
import { test } from "node:test";
import { gateView, isReturningToSample, type GateRequest } from "./first-launch.ts";

const view = (encrypted: boolean, request: GateRequest = "auto", open = false) =>
  gateView({ runtimeError: false, status: { open, encrypted_vault_exists: encrypted }, routeNeedsWorkspace: true, request });

test("a fresh install opens on Welcome", () => {
  assert.equal(view(false), "welcome");
});

test("an existing encrypted vault opens on unlock, not Welcome", () => {
  assert.equal(view(true), "unlock");
  assert.equal(view(true, "access"), "unlock");
});

test("no request can turn a closed vault into an open workspace", () => {
  for (const request of ["auto", "welcome", "access"] as const) {
    for (const encrypted of [true, false]) assert.notEqual(view(encrypted, request), "workspace");
  }
});

test("choosing the sample from unlock shows Welcome, and only then", () => {
  assert.equal(view(true, "welcome"), "welcome");
});

test("creating a vault is explicit when none exists", () => {
  assert.equal(view(false, "access"), "create");
});

test("an open workspace shows the workspace", () => {
  assert.equal(view(false, "auto", true), "workspace");
  assert.equal(view(true, "access", true), "workspace");
});

test("Settings and About stay reachable with no workspace open", () => {
  assert.equal(gateView({ runtimeError: false, status: { open: false, encrypted_vault_exists: true }, routeNeedsWorkspace: false, request: "auto" }), "workspace");
});

test("an unreachable runtime never falls through to a workspace", () => {
  assert.equal(gateView({ runtimeError: true, status: null, routeNeedsWorkspace: true, request: "auto" }), "runtime-unavailable");
  assert.equal(gateView({ runtimeError: true, status: null, routeNeedsWorkspace: false, request: "welcome" }), "runtime-unavailable");
  assert.equal(gateView({ runtimeError: false, status: null, routeNeedsWorkspace: true, request: "auto" }), "loading");
});

test("only Core reporting the sample on disk counts as returning", () => {
  assert.equal(isReturningToSample({ sample_workspace_exists: true }), true);
  for (const value of [null, undefined, {}, { sample_workspace_exists: false }]) assert.equal(isReturningToSample(value), false);
});

test("shown-once recovery codes hold the screen until confirmed", () => {
  for (const open of [false, true]) for (const routeNeedsWorkspace of [false, true]) {
    assert.equal(gateView({ runtimeError: false, status: { open, encrypted_vault_exists: true }, routeNeedsWorkspace, request: "auto", recoveryCodesShown: true }), "create");
  }
});
