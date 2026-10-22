export const github = "https://github.com/AbdulazizShehri/MedScale";
export const basePath = process.env.NEXT_PUBLIC_SITE_BASE_PATH || "";
/** Public URL of a file in web/public, honouring the GitHub Pages base path. */
export const asset = (path: string) => basePath + path;

const blob = (path: string) => `${github}/blob/main/${path}`;
const tree = (path: string) => `${github}/tree/main/${path}`;

export const docs = {
  readme: blob("README.md"),
  product: blob("PRODUCT.md"),
  desktop: blob("apps/desktop-tauri/README.md"),
  cli: blob("docs/product/CLI_PRODUCT_GUIDE.md"),
  qualification: blob("docs/qualification/EXTERNAL_GATES.md"),
  contributing: blob("CONTRIBUTING.md"),
  security: blob("SECURITY.md"),
  governance: blob("GOVERNANCE.md"),
  conduct: blob("CODE_OF_CONDUCT.md"),
  brand: tree("docs/brand"),
  crates: tree("crates"),
  desktopSource: tree("apps/desktop-tauri"),
  issues: `${github}/issues`,
  discussions: `${github}/discussions`,
  releases: `${github}/releases`,
  license: blob("LICENSE"),
};

export const nav = [
  { href: "/product/", label: "Product" },
  { href: "/models/", label: "Models" },
  { href: "/download/", label: "Download" },
  { href: "/docs/", label: "Docs" },
  { href: "/community/", label: "Community" },
] as const;

/** Real screenshots of the desktop app, captured by the desktop-e2e journeys on the synthetic workspace. */
export const shots = {
  patients: { src: "/shots/patients-dark.webp", w: 1028, h: 749, alt: "MedScale Patients view listing two synthetic subjects with coverage and review status" },
  insightsHero: { src: "/shots/insights-hero-dark.webp", w: 1028, h: 620, alt: "MedScale Insights view showing cohort coverage, conflicts and an evidence-grounded answer for synthetic records" },
  insights: { src: "/shots/insights-dark.webp", w: 1028, h: 749, alt: "MedScale Insights view showing cohort coverage, conflicts and an evidence-grounded answer for synthetic records" },
  medagent: { src: "/shots/medagent-dark.webp", w: 1028, h: 749, alt: "MedScale MedAgent view with an agent run bound to selected synthetic sources" },
  models: { src: "/shots/models-dark.webp", w: 1028, h: 749, alt: "MedScale Models view with the local runtime state and the browsable model catalog" },
  audit: { src: "/shots/audit.webp", w: 1028, h: 749, alt: "MedScale Audit Trail listing a recorded synthetic disclosure" },
  knowledge: { src: "/shots/knowledge.webp", w: 1028, h: 749, alt: "MedScale Knowledge view with a local index and search receipt" },
} as const;
export type ShotKey = keyof typeof shots;

/** Page metadata with a canonical URL relative to metadataBase (Next replaces, not merges, openGraph). */
export function pageMeta(path: string, title: string, description: string) {
  const url = path.replace(/^\//, "") || "./";
  const full = `${title} — MedScale`;
  const images = [{ url: "og.png", width: 1200, height: 630, alt: "MedScale — Your health AI. One open workspace." }];
  return {
    title,
    description,
    alternates: { canonical: url },
    openGraph: { type: "website" as const, siteName: "MedScale", title: full, description, url, images },
    twitter: { card: "summary_large_image" as const, title: full, description, images: ["og.png"] },
  };
}
