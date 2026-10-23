/** Source-of-truth inventory: apps/desktop-tauri/README.md and src/App.tsx.
 * Every listed area maps to an existing Tauri desktop route.
 * Copy describes the synthetic Beta interface, not clinical readiness.
 */
export const featureGroups = [
  {
    id: "clinical", title: "Clinical workspace",
    summary: "Bring records, supporting documents and cited evidence into view.",
    items: [
      { name: "Patients", detail: "Explore synthetic patient profiles, longitudinal timelines, labs, source coverage and review state." },
      { name: "Documents", detail: "Inspect imported synthetic documents alongside the records they support." },
      { name: "Insights", detail: "Review coverage, gaps and conflicting information with evidence-grounded answers." },
      { name: "Evidence", detail: "Trace claims back to their sources and distinguish support from uncertainty." },
    ],
  },
  {
    id: "research", title: "Research & discovery",
    summary: "Organize a project and explore its data without losing provenance.",
    items: [
      { name: "Projects", detail: "Keep project context and research materials organized in the workspace." },
      { name: "Data", detail: "Inspect synthetic data snapshots and their source context." },
      { name: "Browse", detail: "Use governed browsing with explicit network controls." },
      { name: "Analytics", detail: "Explore queries and analytical outputs within the local workspace." },
      { name: "Knowledge", detail: "Search a local source index and inspect receipts for the results." },
      { name: "Research OS", detail: "Bring research-oriented work into the same desktop environment." },
    ],
  },
  {
    id: "intelligence", title: "Models & agents",
    summary: "Discover available models, inspect runtimes and keep agent output reviewable.",
    items: [
      { name: "Models", detail: "Browse indexed model metadata by task, language, format and source; listings are not installed weights." },
      { name: "MedAgent", detail: "Run bounded agents on admitted local packs, with source-bound proposals and receipts." },
      { name: "Model Fleet", detail: "Inspect local model and runtime state, admission and pack information." },
    ],
  },
  {
    id: "operations", title: "Work & collaboration",
    summary: "Connect everyday tasks to the same accountable workspace.",
    items: [
      { name: "Workflows", detail: "Explore structured steps and their visible execution state." },
      { name: "Tasks", detail: "Review and manage work items in the desktop interface." },
      { name: "Messages", detail: "Open the workspace messaging surface for synthetic scenarios." },
      { name: "Collaboration", detail: "Explore collaboration rooms and shared-work interfaces in Beta." },
      { name: "Audio", detail: "Try locally processed speech-to-text using synthetic audio fixtures." },
    ],
  },
  {
    id: "governance", title: "Control & transparency",
    summary: "Keep privacy controls, disclosures and system state understandable.",
    items: [
      { name: "Privacy", detail: "Inspect local workspace controls, access boundaries and privacy state." },
      { name: "Audit Trail", detail: "Read Core-recorded disclosure history and trace what happened." },
      { name: "Exports", detail: "Preview governed exports, including bounded FHIR R4 interchange." },
      { name: "Integrations", detail: "Inspect the integration surface and its governed connection boundaries." },
      { name: "Settings", detail: "Adjust display preferences and inspect local configuration." },
      { name: "About", detail: "Find application identity, version and engineering release information." },
    ],
  },
] as const;

export const DESKTOP_FEATURE_COUNT = featureGroups.reduce((total, group) => total + group.items.length, 0);

export function FeatureAtlas({ compact = false }: { compact?: boolean }) {
  return (
    <div className={"feature-atlas" + (compact ? " is-compact" : "")}>
      {featureGroups.map((group, index) => (
        <section className="atlas-group" key={group.id} aria-labelledby={`atlas-${group.id}`}>
          <div className="atlas-intro">
            <span className="atlas-index" aria-hidden="true">0{index + 1} / 05</span>
            <h3 id={`atlas-${group.id}`}>{group.title}</h3>
            <p>{group.summary}</p>
          </div>
          <ul className="atlas-features">
            {group.items.map((feature) => (
              <li key={feature.name}>
                <strong>{feature.name}</strong>
                {!compact && <span>{feature.detail}</span>}
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
