"use client";
import { useState } from "react";
import { asset } from "@/lib/site";

type SceneId = "records"|"evidence"|"models"|"workflow";
type Scene = {name:string,subheading:string,headline:string,description:string,tags:string[],records:{title:string,detail:string,state:string}[],note:string};
const scenes:Record<SceneId,Scene>={
 records:{name:"Patient context",subheading:"SYNTHETIC SUBJECT / S-001",headline:"A record, not just a result.",description:"A traceable view of a longitudinal synthetic record.",tags:["Timeline","Laboratory results","Documents"],records:[{title:"Imported FHIR R4 fixture",detail:"Local test source · no real patient information",state:"SOURCE"},{title:"Longitudinal context",detail:"Preserve dates, coverage and origin",state:"CONTEXT"},{title:"Review before action",detail:"Interpretations require deliberate review",state:"REVIEW"}],note:"Synthetic source inputs are used for demonstration."},
 evidence:{name:"Evidence review",subheading:"SYNTHETIC EVIDENCE / REVIEW",headline:"Know what supports the claim.",description:"Keep source material and derived statements separate.",tags:["Source trace","Coverage","Review state"],records:[{title:"Source document",detail:"Identity and provenance are inspectable",state:"SOURCE"},{title:"Derived interpretation",detail:"A proposal, not an authoritative observation",state:"PROPOSAL"},{title:"Evidence boundary",detail:"Missing coverage remains explicit",state:"REVIEW"}],note:"No model claim is represented as clinically verified."},
 models:{name:"Model intelligence",subheading:"LOCAL RUNTIME / CATALOG",headline:"A model is more than a name.",description:"Understand source, runtime and admission states.",tags:["Provenance","Runtime","Qualification"],records:[{title:"Catalog source",detail:"Traceable repository and integrity context",state:"LISTED"},{title:"Local runtime",detail:"Availability differs from model admission",state:"RUNTIME"},{title:"Clinical qualification",detail:"Not granted by presence in a catalog",state:"NOT ADMITTED"}],note:"Catalog models are not endorsed clinical tools."},
 workflow:{name:"Governed workflows",subheading:"SYNTHETIC WORKFLOW / REVIEW",headline:"The final decision stays yours.",description:"Follow an explicit boundary from observation to action.",tags:["Prepare","Inspect","Review"],records:[{title:"Prepare record context",detail:"Source-aware synthetic input",state:"INPUT"},{title:"Inspect evidence",detail:"Review gaps, conflicts and policy",state:"CHECK"},{title:"Authorize next step",detail:"No silent authority-changing operations",state:"REVIEW"}],note:"This workflow visualization is illustrative only."}
};
const choices:{id:SceneId;icon:string}[]=[{id:"records",icon:"▤"},{id:"evidence",icon:"◇"},{id:"models",icon:"◈"},{id:"workflow",icon:"↗"}];
export function PreviewBoard({compact=false,initial="records"}:{compact?:boolean;initial?:SceneId}){
 const [selected,setSelected]=useState<SceneId>(initial);
 const [details,setDetails]=useState<"sources"|"review">("sources");
 const scene=scenes[selected];
 return <div className={"p-window"+(compact?" p-window-compact":"")} aria-label="Interactive conceptual preview of the synthetic MedScale workspace">
   <div className="p-window-chrome"><span className="p-window-traffic" aria-hidden="true"><i/><i/><i/></span><span className="p-window-chrome-title">MedScale <span>/</span> Workspace preview</span><span className="p-window-chrome-right"><i aria-hidden="true"/> LOCAL CONCEPT</span></div>
   <div className="p-window-body">
    <aside className="p-window-rail"><img src={asset("/brand/mark-white.svg")} alt="MedScale" width="27" height="36"/><div className="p-rail-links">{choices.map(item=><button key={item.id} type="button" title={scenes[item.id].name} aria-label={"Show "+scenes[item.id].name} aria-pressed={selected===item.id} onClick={()=>setSelected(item.id)} className={selected===item.id?"active":""}>{item.icon}</button>)}</div><span className="p-rail-end" aria-hidden="true">◌</span></aside>
    <aside className="p-window-nav">
      <div className="p-window-label">WORKSPACE</div><div className="p-window-nav-title">Synthetic workspace <span>⌄</span></div>
      <div className="p-window-nav-divider"/>
      <span className="p-window-nav-section">EXPLORATION</span>
      <div className="p-window-nav-list">{choices.map(item=><button type="button" key={item.id} onClick={()=>setSelected(item.id)} aria-pressed={selected===item.id} className={selected===item.id?"active":""}><span>{item.icon}</span>{scenes[item.id].name}</button>)}</div>
      <div className="p-window-nav-lower"><span className="p-window-nav-section">SECURITY STATE</span><div><i aria-hidden="true"/> Local fixture only</div><small>Clinical use not authorized</small></div>
    </aside>
    <div className="p-window-main">
      <div className="p-window-route"><span>Workspace</span><b>/</b><span>{scene.name}</span><span className="p-window-route-right">READ ONLY DEMO <i aria-hidden="true">⋯</i></span></div>
      <div className="p-window-main-inner"><span className="p-window-overline">{scene.subheading}</span>
       <div className="p-window-headline"><div><h3>{scene.name}</h3><p>{scene.description}</p></div><span className="p-window-synthetic">● SYNTHETIC</span></div>
       <div className="p-window-inline-tabs">{["Overview",...scene.tags.slice(0,2)].map((tab,i)=><span key={tab} className={i===0?"active":""}>{tab}</span>)}</div>
       <div className="p-window-content">
         <div className="p-window-note"><span className="p-note-index">01 — CONTEXT</span><span className="p-note-dots">◇ ◇ ◇</span></div>
         <h4>{scene.headline}</h4><p className="p-window-summary">{scene.description}</p>
         <div className="p-window-visual" aria-hidden="true"><span/><span/><span/><span/><span/><span/><span/><span/><span/></div>
         <span className="p-window-section">SOURCE &amp; REVIEW TRAIL</span>
         <div className="p-window-records">{scene.records.map((r,i)=><div key={r.title} className="p-window-record"><span className="p-record-icon">{i===0?"↗":i===1?"▧":"◇"}</span><div><strong>{r.title}</strong><small>{r.detail}</small></div><span className={"p-record-state"+(i===2?" caution":"")}>{r.state}</span></div>)}</div>
       </div>
       <div className="p-window-bottom"><span>◉ LOCAL CORE · ILLUSTRATIVE</span><span>NOT A CLINICAL RECORD</span></div>
      </div>
    </div>
    <aside className="p-window-inspector"><div className="p-inspector-top"><strong>Record context</strong><span>↗</span></div><div className="p-inspector-tabs"><button type="button" className={details==="sources"?"active":""} onClick={()=>setDetails("sources")}>Sources</button><button type="button" className={details==="review"?"active":""} onClick={()=>setDetails("review")}>Review</button></div>
      {details==="sources"?<div className="p-inspector-content"><span>TRACEABILITY</span><div className="p-inspector-source"><b>↗</b><strong>Source identity</strong><small>Preserved as supplied</small></div><div className="p-inspector-source"><b>◇</b><strong>Review boundary</strong><small>Explicit, never assumed</small></div><div className="p-inspector-source"><b>⌑</b><strong>Authority</strong><small>Human-led by design</small></div></div>:<div className="p-inspector-content"><span>REVIEW POSTURE</span><div className="p-inspector-source"><b>○</b><strong>Pending review</strong><small>No final approval is implied</small></div><div className="p-inspector-source"><b>◇</b><strong>Evidence gaps</strong><small>Must remain visible</small></div></div>}
      <div className="p-inspector-bottom"><i/> {scene.note}</div></aside>
   </div>
   <div className="p-window-caption"><span><i/> SYNTHETIC DATA / STATIC EXAMPLE</span><span>DESIGN PREVIEW — NO LIVE PATIENT DATA OR MODEL EXECUTION</span></div>
 </div>;
}
