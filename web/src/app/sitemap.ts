import type { MetadataRoute } from "next";

export const dynamic = "force-static";
const site = (process.env.NEXT_PUBLIC_SITE_URL || "https://abdulazizshehri.github.io/MedScale").replace(/\/$/, "");

export default function sitemap(): MetadataRoute.Sitemap {
  return ["/", "/product/", "/models/", "/download/", "/docs/", "/community/"].map((p) => ({ url: site + p, changeFrequency: "weekly", priority: p === "/" ? 1 : 0.7 }));
}
