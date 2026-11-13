import Link from "next/link";
import { asset, docs, github, sponsors } from "@/lib/site";

export function Footer() {
  return (
    <footer className="footer">
      <div className="shell">
        <div className="footer-grid">
          <div className="footer-brand">
            <Link href="/" aria-label="MedScale home"><img src={asset("/brand/medscale-horizontal-white.svg")} alt="" width={104} height={17} /></Link>
            <p>An open-source, local-first workspace for health records, evidence and AI models.</p>
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
            <a href={sponsors}>Sponsor development</a>
            <a href={docs.security}>Security</a>
          </div>
        </div>
        <div className="footer-bottom">
          <span>© 2026 MedScale contributors · <a href={docs.license}>Apache-2.0</a></span>
          <span className="footer-scope">For synthetic workspaces + licensed nonpatient audio; real patient records, clinical decisions and production deployments are not authorized.</span>
        </div>
      </div>
    </footer>
  );
}
