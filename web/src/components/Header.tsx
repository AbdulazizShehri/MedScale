"use client";
import Link from "next/link";
import { useState } from "react";
import { asset, docs } from "@/lib/site";

export function Header() {
  const [expanded, setExpanded] = useState(false);
  return <header className="site-header">
    <div className="shell header-inner">
      <Link href="/" aria-label="MedScale — Home" className="brand-link" onClick={()=>setExpanded(false)}><img src={asset("/brand/medscale-horizontal-white.svg")} alt="MedScale" width="164" height="50" /></Link>
      <button type="button" className="mobile-toggle" aria-expanded={expanded} aria-controls="nav-links" aria-label={expanded ? "Close navigation" : "Open navigation"} onClick={()=>setExpanded(!expanded)}>{expanded ? "✕" : "☰"}</button>
      <nav id="nav-links" aria-label="Main navigation" className={expanded ? "main-nav expanded":"main-nav"}>
        <Link onClick={()=>setExpanded(false)} href="/features/">Platform</Link>
        <Link onClick={()=>setExpanded(false)} href="/download/">Download</Link>
        <Link onClick={()=>setExpanded(false)} href="/docs/">Docs</Link>
        <Link onClick={()=>setExpanded(false)} href="/community/">Community</Link>
      </nav>
      <a className="header-github" href={docs.source} target="_blank" rel="noreferrer">Explore source <span aria-hidden="true">↗</span></a>
    </div>
  </header>;
}
