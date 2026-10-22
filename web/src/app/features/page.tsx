import type { Metadata } from "next";
import Link from "next/link";
import { ProductExplorer } from "@/components/ProductExplorer";
import { docs } from "@/lib/site";
export const metadata:Metadata={title:"Platform",description:"Explore MedScale's local-first health intelligence workspace: synthetic records, provenance, model state, and governed workflows."};
const features=[
["01","Clinical records","Patient workspace, longitudinal timelines, labs, documents, and source-aware insights on synthetic fixtures."],
["02","Evidence and research","Provenance, source identity, review state, audit, and evidence-led exploration."],
["03","Local models","Catalog and runtime inspection, with explicit separation of downloaded, installed, admitted, and qualified states."],
["04","Workflows and collaboration","Tasks, messages, and governed workflow surfaces without silent authority changes."],
["05","Interoperability","Bounded FHIR R4 exchange and synthetic stubs; no authorized live partner connectivity."],
["06","Data protection","Encrypted workspace controls with privacy and device-qualification boundaries still open."]
];
export default function Features(){return <main id="main"><div className="shell subpage-hero"><div className="eyebrow"><span className="tiny-bar"/> PLATFORM / CAPABILITIES</div><h1>Everything in context.<br/><em>Nothing assumed.</em></h1><p>MedScale brings the tools for examining clinical information and the evidence around it into a single local-first architecture.</p></div><section className="shell subpage-section"><ProductExplorer/></section><section className="shell subpage-section"><div className="eyebrow"><span className="tiny-bar"/> WHAT YOU CAN EXPLORE</div><div className="detail-grid">{features.map(([n,t,d])=><article key={n}><small>{n}</small><h2>{t}</h2><p>{d}</p></article>)}</div></section><section className="shell page-end"><h2>One Rust Core.<br/>Multiple interfaces.</h2><p>Desktop and CLI present the Core's governed results. This is source-available engineering software, not a validated medical device.</p><div className="hero-actions"><Link href="/download/" className="button button-white">Download & build status <span>↗</span></Link><a className="button button-outline" href={docs.source}>Read source <span>↗</span></a></div></section></main>}
