"use client";
import { useState } from "react";
import { asset, productAreas } from "@/lib/site";
type Mode = "records" | "evidence" | "models" | "workflow";
export function PreviewBoard({compact=false,initial="records"}:{compact?:boolean;initial?:Mode}) {
 const [selected,setSelected] = useState<Mode>(initial);
 const current = productAreas.find(x=>x.id===selected) ?? productAreas[0];
 return <div className={compact?"board board-compact":"board"}>
  <div className="board-top"><div className="traffic"><i/><i/><i/></div><span>MedScale / Synthetic workspace</span><span className="board-top-right">DEMONSTRATION</span></div>
  <div className="board-inner">
   <aside className="board-sidebar" aria-label="Illustration navigation">
    <img src={asset("/brand/mark-white.svg")} alt="" width="24" height="29"/>
    {productAreas.map(p=><button key={p.id} className={selected===p.id?"board-nav-active":""} title={p.mode} type="button" onClick={()=>setSelected(p.id)} aria-label={"Preview "+p.mode} aria-pressed={selected===p.id}>{p.index}</button>)}
    <span className="board-rail-bottom">MS</span>
   </aside>
   <div className="board-content">
    <div className="board-breadcrumb">WORKSPACE <span>/</span> {current.mode.toUpperCase()}</div>
    <div className="board-heading"><div><h3>{current.mode}</h3><p>Design illustration with synthetic examples</p></div><span className="board-pill">SYNTHETIC</span></div>
    <div className="board-tabs"><span className="board-tab-current">Overview</span><span>Sources</span><span>Review state</span></div>
    <div className="board-data">
      <div className="board-data-top"><span>CONTEXT / {current.eyebrow}</span><span>{current.index} / 04</span></div>
      <h4>{current.title}</h4>
      <p>{current.text}</p>
      <div className="board-list">
        {current.rows.map((row,i)=><div className="board-row" key={row}><span className="board-row-icon">{i===0?"↗":i===1?"▧":"◈"}</span><span>{row}</span><small>{i===2?"REVIEWABLE":i===1?"CONTEXTUAL":"TRACEABLE"}</small></div>)}
      </div>
    </div>
    <div className="board-status"><span><i/> LOCAL CORE / ILLUSTRATIVE</span><span>NOT A CLINICAL RECORD</span></div>
   </div>
  </div>
 </div>;
}
