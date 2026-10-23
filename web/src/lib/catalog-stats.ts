/**
 * Marketing-safe counts from pinned third-party model metadata, not from
 * MedScale-published model weights, users, installs or production capabilities.
 * `npm run check` audits the values against the exact source catalog.
 */
export const MODEL_CATALOG_STATS = {
  modelRecords: 2266,
  languageTags: 35,
  modelFamilies: 5,
} as const;
