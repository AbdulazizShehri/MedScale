import type { MetadataRoute } from "next";

export const dynamic = "force-static";
const site = (process.env.NEXT_PUBLIC_SITE_URL || "https://abdulazizshehri.github.io/MedScale").replace(/\/$/, "");

export default function robots(): MetadataRoute.Robots {
  return { rules: { userAgent: "*", allow: "/" }, sitemap: `${site}/sitemap.xml` };
}
