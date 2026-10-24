/**
 * Historical synthetic-only Windows development-host measurements.
 * Values are frozen to exact evidence fixtures and checked at web build time.
 * p95 is a percentile of the recorded small-run sample, not an SLA.
 */
export const PERF_MARKETING = {
  lexical10k: { p95Ms: 279.9535, samples: 3, syntheticDocuments: 10000 },
  timeline64: { p95Ms: 31.9214, samples: 30, syntheticEvents: 64 },
  csvScale: { syntheticRows: 20000 },
} as const;
