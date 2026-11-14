import assert from "node:assert/strict";
import test from "node:test";
import { parseDensity, parseTheme } from "./ui-preferences.ts";

test("only supported themes survive persisted preference parsing", () => {
  assert.equal(parseTheme("light"), "light");
  assert.equal(parseTheme("dark"), "dark");
  for (const value of [null, "", "auto", "LIGHT", "transparent", "patient-123"]) {
    assert.equal(parseTheme(value), null);
  }
});

test("explicit standard and compact URL density both override prior stored state", () => {
  assert.equal(parseDensity("standard") ?? parseDensity("compact") ?? "standard", "standard");
  assert.equal(parseDensity("compact") ?? parseDensity("standard") ?? "standard", "compact");
});

test("invalid density falls back to a valid saved preference or the default", () => {
  assert.equal(parseDensity("unknown") ?? parseDensity("compact") ?? "standard", "compact");
  assert.equal(parseDensity("unknown") ?? parseDensity("malformed") ?? "standard", "standard");
  assert.equal(parseDensity(null), null);
});
