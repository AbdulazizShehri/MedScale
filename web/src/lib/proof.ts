/**
 * Numbers the site may show. Each one comes from a public file, and
 * `npm run check` recounts them, so a figure here can't drift from its source.
 * None of them measures users, adoption or clinical outcomes: MedScale has no such data yet.
 */
export const PROOF = {
  /** third_party/medscale-model-catalog-v3/models.jsonl. Every row is published by OpenMed. */
  catalogModels: 2266,
  catalogLanguages: 35,
  catalogFamilies: 5,
  /** EXECUTED_MODELS in crates/medscale-pack/src/compatibility.rs. */
  catalogExecuted: 1466,
  /** MEASURED_OVER_BOUND in the same file. */
  catalogOverBound: 262,
  /** A floor, not an exact count: #[test] functions in crates/ and the desktop host. */
  rustTestsFloor: 1300,
  /** Directories in specs/. */
  specs: 103,
} as const;

export const count = (n: number) => n.toLocaleString("en-US");
