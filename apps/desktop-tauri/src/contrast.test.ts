// Deterministic contrast guard for the brand accent tokens (medscale.css
// section 16, docs/design/BRAND_ACCENTS_2026-10-07.md). Reads the real
// stylesheet so a token change that breaks contrast fails the build. This is
// an engineering check, not an accessibility certification.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const css = readFileSync(new URL("./styles/medscale.css", import.meta.url), "utf8");
const section16 = css.slice(css.indexOf("16. Brand accents"));

function block(selectorStart: string): string {
  const i = section16.indexOf(selectorStart);
  assert.ok(i >= 0, `missing block ${selectorStart}`);
  return section16.slice(i, section16.indexOf("}", i));
}
function token(source: string, name: string): string {
  const m = source.match(new RegExp(`${name}:\\s*(#[0-9A-Fa-f]{6})`));
  const value = m?.[1];
  assert.ok(value, `missing ${name}`);
  return value;
}
function channel(hex: string, i: number): number {
  const v = parseInt(hex.slice(i, i + 2), 16) / 255;
  return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
}
function luminance(hex: string): number {
  return 0.2126 * channel(hex, 1) + 0.7152 * channel(hex, 3) + 0.0722 * channel(hex, 5);
}
function contrast(a: string, b: string): number {
  const la = luminance(a);
  const lb = luminance(b);
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

const dark = block(':root:not([data-theme="light"])');
const light = block('[data-theme="light"] {\n  --brand-blue');
const DARK_SURFACE = "#161616";
const LIGHT_SURFACE = "#FFFFFF";

test("accent fills carry black text at AA (4.5:1) in both themes", () => {
  for (const src of [dark, light]) {
    assert.ok(contrast(token(src, "--brand-blue"), token(src, "--accent-on")) >= 4.5);
    assert.ok(contrast(token(src, "--brand-orange"), "#000000") >= 4.5);
  }
});

test("dark-mode accent text is AA on the work surface", () => {
  assert.ok(contrast(token(dark, "--accent-text"), DARK_SURFACE) >= 4.5);
  assert.ok(contrast(token(dark, "--signal-text"), DARK_SURFACE) >= 4.5);
  assert.ok(contrast(token(dark, "--ink-1"), DARK_SURFACE) >= 7);
});

test("light-mode accent text is AA on white and the review glyph meets 3:1", () => {
  assert.ok(contrast(token(light, "--accent-text"), LIGHT_SURFACE) >= 4.5);
  assert.ok(contrast(token(light, "--signal-text"), LIGHT_SURFACE) >= 4.5);
  assert.ok(contrast(token(light, "--sem-review"), LIGHT_SURFACE) >= 3);
  assert.ok(contrast(token(light, "--focus"), LIGHT_SURFACE) >= 3);
});

test("raw accents are never used as light-mode text (they are below 3:1 on white)", () => {
  assert.ok(contrast("#70B8C7", LIGHT_SURFACE) < 3);
  assert.notEqual(token(light, "--accent-text").toUpperCase(), "#70B8C7");
  assert.notEqual(token(light, "--signal-text").toUpperCase(), "#FB905A");
});
