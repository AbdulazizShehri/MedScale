import type { Metadata, Viewport } from "next";
import localFont from "next/font/local";
import "./globals.css";
import { Header } from "@/components/Header";
import { Footer } from "@/components/Footer";
import { asset } from "@/lib/site";

// Latin subsets of the repository's OFL fonts (licenses in /fonts/): Inter for text, Plus Jakarta
// Sans (the wordmark family) for headlines, JetBrains Mono NL for data.
const inter = localFont({ src: "../fonts/InterVariable-latin.woff2", variable: "--font-inter", weight: "100 900", display: "swap" });
const display = localFont({ src: "../fonts/PlusJakartaSans-latin.woff2", variable: "--font-pjs", weight: "200 800", display: "swap" });
const mono = localFont({ src: "../fonts/JetBrainsMonoNL-latin.woff2", variable: "--font-jbm", weight: "400", display: "optional", preload: true });

const site = process.env.NEXT_PUBLIC_SITE_URL || "https://abdulazizshehri.github.io/MedScale";
const title = "MedScale — Your health AI. One open workspace.";
const description = "Open-source, local-first workspace for health records, evidence and AI models. Beta software in active development for synthetic-only evaluation.";

export const metadata: Metadata = {
  metadataBase: new URL(site.endsWith("/") ? site : site + "/"),
  title: { default: title, template: "%s — MedScale" },
  description,
  applicationName: "MedScale",
  keywords: ["MedScale", "open source", "health AI", "clinical intelligence", "local-first", "FHIR", "Rust", "evidence", "provenance"],
  icons: {
    icon: [{ url: asset("/brand/medscale-favicon.svg"), type: "image/svg+xml" }, { url: asset("/brand/favicon-32.png"), sizes: "32x32", type: "image/png" }],
    apple: asset("/brand/apple-touch-icon.png"),
  },
  openGraph: { type: "website", siteName: "MedScale", title, description, images: [{ url: "og.png", width: 1200, height: 630, alt: "MedScale — Your health AI. One open workspace." }] },
  twitter: { card: "summary_large_image", title, description, images: ["og.png"] },
  robots: { index: true, follow: true },
};

export const viewport: Viewport = { width: "device-width", initialScale: 1, themeColor: "#08090a", colorScheme: "dark" };

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en" className={`${inter.variable} ${display.variable} ${mono.variable}`}>
      <body>
        <a className="skip-link" href="#main">Skip to content</a>
        <Header />
        {children}
        <Footer />
      </body>
    </html>
  );
}
