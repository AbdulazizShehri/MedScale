import type { Metadata, Viewport } from "next";
import "./globals.css";
import "./premium.css";
import { Header } from "@/components/Header";
import { Footer } from "@/components/Footer";

export const metadata: Metadata = {
  title: { default: "MedScale — Evidence first. Action second.", template: "%s | MedScale" },
  description: "MedScale is an open-source, local-first health intelligence workspace. Explore records, evidence, models, and governed workflows. Engineering preview — synthetic data only.",
  applicationName: "MedScale",
  keywords: ["MedScale", "open source", "health informatics", "FHIR", "Rust", "local-first", "clinical intelligence"],
  robots: { index: true, follow: true },
  openGraph: { title: "MedScale — Evidence first. Action second.", description: "Open-source clinical intelligence, with the evidence in view. Engineering preview for synthetic data only.", type: "website", siteName: "MedScale" },
};
export const viewport: Viewport = { width: "device-width", initialScale: 1, themeColor: "#0a0b0c" };

export default function RootLayout({ children }: Readonly<{children:React.ReactNode}>) {
  return (
    <html lang="en">
      <body>
        <a className="skip-link" href="#main">Skip to content</a>
        <div className="top-notice"><span className="notice-light" aria-hidden="true" /> ENGINEERING PREVIEW <span className="notice-separator" /> Synthetic data only. Not for clinical use. <a href="https://github.com/AbdulazizShehri/MedScale/blob/main/docs/qualification/EXTERNAL_GATES.md">Read qualification status <span aria-hidden="true">↗</span></a></div>
        <Header />
        {children}
        <Footer />
      </body>
    </html>
  );
}
