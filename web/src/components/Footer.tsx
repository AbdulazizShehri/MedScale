import Link from "next/link";
import { asset, docs, github } from "@/lib/site";
export function Footer() { return <footer className="footer">
  <div className="shell footer-inner">
   <div><Link href="/" aria-label="MedScale home"><img src={asset("/brand/medscale-horizontal-white.svg")} alt="MedScale" width="158" height="47" /></Link><p>Evidence first. Action second.</p><small>Open-source clinical intelligence engineering preview.</small></div>
   <div className="footer-col"><strong>Explore</strong><Link href="/features/">Platform</Link><Link href="/download/">Download</Link><Link href="/docs/">Documentation</Link><Link href="/community/">Community</Link></div>
   <div className="footer-col"><strong>Open source</strong><a href={github}>Repository</a><a href={docs.contribution}>Contributing</a><a href={docs.security}>Security</a><a href={docs.releases}>Releases</a></div>
   <div className="footer-col"><strong>Project status</strong><p>Only synthetic data is authorized. No clinical or production release.</p><a className="subtle-link" href={docs.qualification}>External qualification ↗</a></div>
  </div>
  <div className="shell footer-bottom"><span>© {new Date().getFullYear()} MedScale · Apache-2.0 first-party source</span><span>Public code ≠ clinically qualified software</span></div>
 </footer> }
