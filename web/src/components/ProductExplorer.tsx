"use client";
import { useState } from "react";
import { productAreas } from "@/lib/site";
import { PreviewBoard } from "./PreviewBoard";
type Mode = "records" | "evidence" | "models" | "workflow";
export function ProductExplorer() {
 const [selected,setSelected] = useState<Mode>("records");
 const item=productAreas.find(p=>p.id===selected) ?? productAreas[0];
 return <div className="explorer">
   <div className="explorer-switch" role="tablist" aria-label="Explore platform capabilities">
     {productAreas.map(p=><button type="button" key={p.id} role="tab" aria-selected={selected===p.id} className={selected===p.id?"explorer-option active": "explorer-option"} onClick={()=>setSelected(p.id)}><small>{p.index}</small>{p.mode}</button>)}
   </div>
   <div className="explorer-content">
     <div className="explorer-copy" role="tabpanel">
       <div className="eyebrow"><span className="tiny-bar"/> {item.eyebrow}</div>
       <h3>{item.title}</h3>
       <p>{item.text}</p>
       <ul>{item.chips.map(x=><li key={x}><span aria-hidden="true">↗</span>{x}</li>)}</ul>
       <a href="https://github.com/AbdulazizShehri/MedScale/tree/main/apps/desktop-tauri" className="text-link">Explore the implementation <span aria-hidden="true">↗</span></a>
     </div>
     <div className="explorer-visual"><PreviewBoard initial={selected} key={selected} compact/><p className="visual-caption">CONCEPTUAL WORKSPACE · NOT A PRODUCT SCREENSHOT</p></div>
   </div>
 </div>;
}
