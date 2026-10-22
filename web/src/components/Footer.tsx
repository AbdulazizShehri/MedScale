import Link from "next/link";
import { asset, docs, github } from "@/lib/site";

export function Footer() {
  return (
    <footer className="footer">
      <div className="shell">
        <div className="footer-grid">
          <div className="footer-brand">
            <Link href="/" aria-label="MedScale home"><img src={asset("/brand/medscale-horizontal-white.svg")} alt="" width={95} height={24} /></Link>
            <p>An open-source, local-first workspace for health records, evidence and AI models.</p>
            <p>Engineering preview. Synthetic data only. Not for clinical use.</p>
          </div>
          <div className="footer-col">
            <h2>Product</h2>
            <Link href="/product/">Overview</Link>
            <Link href="/models/">Models</Link>
            <Link href="/download/">Download</Link>
            <a href={docs.qualification}>Release status</a>
          </div>
          <div className="footer-col">
            <h2>Developers</h2>
            <Link href="/docs/">Documentation</Link>
            <a href={docs.crates}>Rust Core</a>
            <a href={docs.cli}>CLI guide</a>
            <a href={docs.brand}>Brand system</a>
          </div>
          <div className="footer-col">
            <h2>Community</h2>
            <a href={github}>GitHub</a>
            <a href={docs.contributing}>Contributing</a>
            <a href={docs.discussions}>Discussions</a>
            <a href={docs.security}>Security</a>
          </div>
        </div>
        <div className="footer-bottom">
          <span>© 2026 MedScale contributors · <a href={docs.license}>Apache-2.0</a></span>
          <span>Open source is not clinical qualification.</span>
        </div>
      </div>
    </footer>
  );
}
