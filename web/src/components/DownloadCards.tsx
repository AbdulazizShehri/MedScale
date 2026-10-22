import { PlatformIcon } from "@/components/PlatformIcon";
import { docs } from "@/lib/site";
import Link from "next/link";
const targets = [
 {key:"windows" as const,title:"Windows",subtitle:"Desktop application",arch:"Windows x64",status:"SIGNED RELEASE NOT AVAILABLE",detail:"Unsigned engineering test packages are not certified end-user releases.",href:docs.desktop},
 {key:"macos" as const,title:"macOS",subtitle:"Desktop application",arch:"Apple desktop",status:"SIGNED RELEASE NOT AVAILABLE",detail:"No notarized production installer is published. Engineering builds are for testing.",href:docs.desktop},
 {key:"linux" as const,title:"Linux",subtitle:"Rust Core + CLI",arch:"Linux desktop deferred",status:"SOURCE BUILD PATH",detail:"The Core and CLI are tested on Linux. A Linux desktop installer is not shipped.",href:docs.cli}
];
export function DownloadCards({full=false}:{full?:boolean}) {
 return <div className={"downloads "+(full?"downloads-full":"")}>
 {targets.map(t=><article className="download-card" key={t.key}>
  <div className="download-heading"><span className="platform-emblem"><PlatformIcon name={t.key} size={32}/></span><span className="platform-arch">{t.arch}</span></div>
  <h3>{t.title}</h3><p className="download-subtitle">{t.subtitle}</p>
  <div className="download-status"><i aria-hidden="true"/>{t.status}</div>
  <p className="download-note">{t.detail}</p>
  <a className="download-link" href={t.href}>View {t.key==="linux"?"CLI instructions":"engineering setup"} <span aria-hidden="true">↗</span></a>
 </article>)}
 </div>
}
export function SourceDownload() {
 return <div className="source-download">
  <div><span className="eyebrow">GET STARTED / SOURCE</span><h3>Clone the project. Inspect the Core.</h3><p>Use the documented Rust toolchain and build from source. This is an engineering workflow, not a clinical deployment.</p></div>
  <div className="terminal"><div className="terminal-bar"><span>TERMINAL</span><span>RUST / LOCAL</span></div><code><span>$</span> git clone https://github.com/AbdulazizShehri/MedScale.git<br/><span>$</span> cd MedScale<br/><span>$</span> cargo run -p medscale-cli --locked -- doctor</code><Link href="/docs/" className="terminal-link">Read setup instructions ↗</Link></div>
 </div>;
}
