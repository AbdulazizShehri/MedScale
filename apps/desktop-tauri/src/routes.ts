export const areas = ["Home", "Clinical", "Research", "Intelligence", "Operations", "Governance", "Utility"] as const;
export type Area = (typeof areas)[number];

/** `scope` says what a route needs: the open workspace, or also a selected project. */
export const routes = [
  { id: "Home", label: "Home", area: "Home", icon: "home", scope: "workspace", description: "Workspace overview and review queue", aliases: ["command center", "overview"] },
  { id: "Patients", label: "Patients", area: "Clinical", icon: "patients", scope: "workspace", description: "Subjects promoted into this vault", aliases: ["patient", "subject", "timeline"] },
  { id: "Documents", label: "Documents", area: "Clinical", icon: "documents", scope: "workspace", description: "Ingested sources and extraction", aliases: ["files", "sources"] },
  { id: "Insights", label: "Insights", area: "Clinical", icon: "insights", scope: "workspace", description: "Cohort coverage from trusted projections", aliases: ["population", "cohort"] },
  { id: "Evidence", label: "Evidence", area: "Clinical", icon: "evidence", scope: "workspace", description: "Versioned local evidence corpus", aliases: ["proof", "corpus", "retrieval"] },
  { id: "Projects", label: "Projects", area: "Research", icon: "projects", scope: "workspace", description: "Projects, experiments and references", aliases: ["project", "experiment"] },
  { id: "Data", label: "Data", area: "Research", icon: "data", scope: "project", description: "Governed sources and snapshots", aliases: ["datasets", "snapshots"] },
  { id: "Browse", label: "Browse", area: "Research", icon: "browse", scope: "project", description: "Allowlisted, governed fetches", aliases: ["browser", "web"] },
  { id: "Analytics", label: "Analytics", area: "Research", icon: "analytics", scope: "project", description: "Read-only snapshot queries", aliases: ["sql", "query"] },
  { id: "Knowledge", label: "Knowledge", area: "Research", icon: "knowledge", scope: "project", description: "Indexed source search and canvases", aliases: ["search", "index"] },
  { id: "Research OS", label: "Research OS", area: "Research", icon: "research", scope: "project", description: "Hub, compute, packs and federation planes", aliases: ["hub", "compute"] },
  { id: "Models", label: "Models", area: "Intelligence", icon: "models", scope: "workspace", description: "Admitted model packs and runtime", aliases: ["model center", "packs"] },
  { id: "MedAgent", label: "MedAgent", area: "Intelligence", icon: "medagent", scope: "project", description: "Evidence-bound agent runs", aliases: ["agents", "agent"] },
  { id: "Model Fleet", label: "Model Fleet", area: "Intelligence", icon: "fleet", scope: "project", description: "Independent lanes and comparisons", aliases: ["fleet", "lanes"] },
  { id: "Workflows", label: "Workflows", area: "Operations", icon: "workflows", scope: "workspace", description: "Review-first controlled actions", aliases: ["workflow", "outbox"] },
  { id: "Tasks", label: "Tasks", area: "Operations", icon: "tasks", scope: "workspace", description: "Review tasks bound to payload digests", aliases: ["review queue"] },
  { id: "Messages", label: "Messages", area: "Operations", icon: "messages", scope: "workspace", description: "Action-linked message previews", aliases: ["message"] },
  { id: "Collaboration", label: "Collaboration", area: "Operations", icon: "collaboration", scope: "project", description: "Rooms, threads and tasks", aliases: ["rooms", "threads"] },
  { id: "Audio", label: "Audio", area: "Operations", icon: "audio", scope: "project", description: "Local audio lineage and segmentation", aliases: ["audioflow", "voice"] },
  { id: "Privacy", label: "Privacy", area: "Governance", icon: "privacy", scope: "project", description: "Data classes and egress decisions", aliases: ["privacy gate", "egress"] },
  { id: "Audit Trail", label: "Audit Trail", area: "Governance", icon: "audit", scope: "workspace", description: "Disclosure history", aliases: ["audit", "disclosures"] },
  { id: "Exports", label: "Exports", area: "Governance", icon: "exports", scope: "workspace", description: "FHIR R4 interchange posture", aliases: ["fhir", "export"] },
  { id: "Integrations", label: "Integrations", area: "Governance", icon: "integrations", scope: "workspace", description: "Brokered integration posture", aliases: ["connections", "nphies"] },
  { id: "Settings", label: "Settings", area: "Utility", icon: "settings", scope: "none", description: "Appearance, privacy and qualification", aliases: ["preferences", "theme"] },
  { id: "About", label: "About", area: "Utility", icon: "about", scope: "none", description: "Version, runtime and licenses", aliases: ["version", "licenses"] },
] as const satisfies ReadonlyArray<{ id: string; label: string; area: Area; icon: string; scope: "none" | "workspace" | "project"; description: string; aliases: readonly string[] }>;

export type Route = (typeof routes)[number];
export type RouteId = Route["id"];

export function isRouteId(input: string): input is RouteId {
  return routes.some((route) => route.id === input);
}

export function searchRoutes(query: string): readonly Route[] {
  if ([...query].length > 128) return [];
  const terms = query.trim().toLocaleLowerCase("en").split(/\s+/).filter(Boolean);
  return routes.filter((route) => {
    const haystack = [route.id, route.label, route.description, ...route.aliases].join(" ").toLocaleLowerCase("en");
    return terms.every((term) => haystack.includes(term));
  });
}
