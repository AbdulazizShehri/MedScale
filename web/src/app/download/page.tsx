import type { Metadata } from "next";
import { DownloadCards,SourceDownload } from "@/components/DownloadCards";
import { docs } from "@/lib/site";
export const metadata:Metadata={title:"Download",description:"MedScale platform support and source-build instructions. No signed public desktop releases are currently available; engineering preview uses synthetic data only."};
export default function Download(){return <main id="main">
 <section className="shell subpage-hero"><div className="eyebrow"><span className="tiny-bar"/> DOWNLOAD / PLATFORMS</div><h1>Build locally.<br/><em>Stay in control.</em></h1><p>MedScale's source is public. Signed, clinically qualified desktop installers are not released, so downloads are deliberately distinguished from engineering builds.</p></section>
 <section className="shell download-page-block"><div className="section-mini-heading"><h2>Choose your environment.</h2><p>View the actual implementation guidance; no placeholder installer links.</p></div><DownloadCards full/></section>
 <section className="shell subpage-section"><SourceDownload/></section>
 <section className="shell download-warning"><div className="eyebrow">RELEASE QUALIFICATION</div><div><h2>Engineering build ≠ production release.</h2><p>Windows engineering packages are unsigned and macOS packages are not notarized. Linux desktop packaging is deferred. Real patient data and live clinical integrations are not authorized. No signed public release files are available.</p><a className="inline-arrow" href={docs.qualification}>Read the external gates <span>↗</span></a><a className="inline-arrow" href={docs.releases}>View GitHub Releases <span>↗</span></a></div></section>
 </main>}
