import { PERF_MARKETING } from "@/lib/performance-stats";

const sources = {
  search: "https://github.com/AbdulazizShehri/MedScale/blob/main/evidence/045-lexical-10k-corpus/perf_harness_delivery_plan_scale.json",
  timeline: "https://github.com/AbdulazizShehri/MedScale/blob/main/evidence/027-perf-sbom-release-evidence/perf_harness_latest.json",
  scale: "https://github.com/AbdulazizShehri/MedScale/blob/main/evidence/075-data-source-fabric/SCALE_MEASUREMENTS.md",
};

export function PerformanceHighlights() {
  const perf = PERF_MARKETING;
  return (
    <section className="performance-highlights" aria-labelledby="performance-title">
      <div className="shell">
        <div className="performance-topline">
          <div>
            <span className="eyebrow">Performance, measured</span>
            <h2 className="h2" id="performance-title">Built to move.<br /><span className="mute">Measured in milliseconds.</span></h2>
          </div>
          <p className="performance-intro">Source-backed engineering benchmarks from MedScale Core, measured with synthetic data on a Windows development machine.</p>
        </div>
        <div className="performance-numbers">
          <div className="performance-number">
            <strong>{Math.ceil(perf.lexical10k.p95Ms)} <span>ms</span></strong>
            <h3>Search 10,000 documents</h3>
            <p>Measured p95 · 3 runs · synthetic lexical corpus</p>
          </div>
          <div className="performance-number">
            <strong>{Math.ceil(perf.timeline64.p95Ms)} <span>ms</span></strong>
            <h3>Project a 64-event timeline</h3>
            <p>Measured p95 · 30 runs · synthetic health timeline</p>
          </div>
          <div className="performance-number">
            <strong>{(perf.csvScale.syntheticRows / 1000).toFixed(0)}K</strong>
            <h3>CSV rows scale-tested</h3>
            <p>Synthetic import tested · not a throughput claim</p>
          </div>
        </div>
        <p className="performance-context">
          Historical development measurements, not guaranteed speeds or model-inference benchmarks. Hardware, dataset size and application version affect results.
          {" "}<a href={sources.search}>Search evidence</a> · <a href={sources.timeline}>Timeline evidence</a> · <a href={sources.scale}>CSV-scale evidence</a>
        </p>
      </div>
    </section>
  );
}
