import Link from "next/link";
import { ArrowRight } from "@/components/Icons";

export default function NotFound() {
  return (
    <main id="main" className="shell page-hero" style={{ minHeight: "60vh", alignContent: "center" }}>
      <span className="eyebrow">404</span>
      <h1 className="display" style={{ fontSize: "clamp(40px, 6vw, 72px)" }}>Nothing here.<br /><span className="mute">Try the front door.</span></h1>
      <div className="actions"><Link className="btn btn-primary" href="/">Back to MedScale <ArrowRight /></Link><Link className="btn btn-secondary" href="/docs/">Documentation</Link></div>
    </main>
  );
}
