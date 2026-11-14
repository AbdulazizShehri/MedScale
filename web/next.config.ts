import type { NextConfig } from "next";
const basePath = process.env.NEXT_PUBLIC_SITE_BASE_PATH || "";
const nextConfig: NextConfig = {
  output: "export",
  trailingSlash: true,
  basePath,
  reactStrictMode: true,
  images: { unoptimized: true },
  // Inline the (small) stylesheet so first paint does not wait on a CSS request.
  experimental: { inlineCss: true },
};
export default nextConfig;
