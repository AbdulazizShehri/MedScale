"use client";
import { useState } from "react";
import { productAreas } from "@/lib/site";
import { PreviewBoard } from "./PreviewBoard";
type Mode = "records"|"evidence"|"models"|"workflow";
export function ProductExplorer(){
 const [selected,setSelected]=useState<Mode>("records");
 const item=productAreas.find(p=>p.id===selected) ?? productAreas[0];
 return <div className="p-explorer">
  <div className="p-explorer-tabs" role="tablist" aria-label="MedScale product areas">{productAreas.map(p=><button key={p.id} type="button" role="tab" aria-selected={selected===p.id} onClick={()=>setSelected(p.id)} className={selected===p.id?"active":""}><span>{p.index} / </span>{p.mode}</button>)}</div>
  <div className="p-explorer-content">
    <div className="p-explorer-copy" role="tabpanel">
      <span className="p-eyebrow">{item.eyebrow}</span>
      <h3>{item.title}</h3><p>{item.text}</p>
      <div className="p-explorer-points">{item.chips.map((c,i)=><div key={c}><span>0{i+1}</span>{c}<b aria-hidden="true">↗</b></div>)}</div>
      <a className="p-text-link" href="https://github.com/AbdulazizShehri/MedScale/tree/main/apps/desktop-tauri">Explore the implementation <span aria-hidden="true">↗</span></a>
    </div>
    <div className="p-explorer-preview"><PreviewBoard key={selected} compact initial={selected}/><small>INTERACTIVE CONCEPT / SYNTHETIC ONLY</small></div>
  </div>
 </div>;
}
