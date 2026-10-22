"use client";
import { useRef, useState, type KeyboardEvent } from "react";
import type { ShotKey } from "@/lib/site";
import { Shot } from "./Shot";

const areas: { id: string; label: string; summary: string; shot: ShotKey; title: string; points: [string, string][] }[] = [
  {
    id: "records", label: "Records", summary: "Longitudinal patients, documents and coverage.", shot: "patients", title: "Patients",
    points: [["Coverage at a glance", "See which facts each record supports before you read further."], ["Review state", "Records that need attention say so in words, not just color."], ["Keyboard first", "Search, open and preview without leaving the list."]],
  },
  {
    id: "insights", label: "Insights", summary: "Answers grounded in the cited record.", shot: "insights", title: "Insights",
    points: [["Present facts only", "Counts come from coverage, not predictions or scores."], ["Conflicts stay visible", "Absent, unknown and conflicting evidence is listed, not smoothed over."], ["Ask the evidence", "Questions are answered from retrieved synthetic documents, with citations."]],
  },
  {
    id: "agents", label: "Agents", summary: "MedAgent runs that leave receipts.", shot: "medagent", title: "MedAgent",
    points: [["Bound context", "You choose which sources an agent may read for each run."], ["Proposals, not actions", "A run's output is a proposal recorded with its receipt."], ["Local packs", "Agents run on admitted local model packs. Nothing is downloaded."]],
  },
  {
    id: "models", label: "Models", summary: "Runtime, catalog and admission state.", shot: "models", title: "Models",
    points: [["Runtime in view", "See which local runtime executes a model and what is admitted."], ["Browsable catalog", "Search models by task and format, with source and size."], ["Listing is not approval", "Catalog presence never implies clinical qualification."]],
  },
  {
    id: "governance", label: "Governance", summary: "An audit trail the Core writes.", shot: "audit", title: "Audit Trail",
    points: [["Recorded by the Core", "Disclosures are written by the trusted Core and cannot be edited."], ["Exports with disclosure", "FHIR export shows what leaves before anything does."], ["Default-deny network", "Browsing and egress stay off until explicitly allowed."]],
  },
];

export function ProductTour() {
  const [index, setIndex] = useState(0);
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);
  const area = areas[index];
  const onKey = (e: KeyboardEvent) => {
    const step = e.key === "ArrowDown" || e.key === "ArrowRight" ? 1 : e.key === "ArrowUp" || e.key === "ArrowLeft" ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    const next = (index + step + areas.length) % areas.length;
    setIndex(next);
    tabs.current[next]?.focus();
  };
  return (
    <div className="tour">
      <div className="tour-tabs" role="tablist" aria-label="MedScale product areas" aria-orientation="vertical" onKeyDown={onKey}>
        {areas.map((a, i) => (
          <button key={a.id} ref={(el) => { tabs.current[i] = el; }} type="button" role="tab" id={`tab-${a.id}`} aria-controls={`panel-${a.id}`} aria-selected={i === index} tabIndex={i === index ? 0 : -1} className="tour-tab" onClick={() => setIndex(i)}>
            <strong>{a.label}</strong><span>{a.summary}</span>
          </button>
        ))}
      </div>
      <div className="tour-panel" role="tabpanel" id={`panel-${area.id}`} aria-labelledby={`tab-${area.id}`}>
        <div key={area.id} className="fade-swap"><Shot name={area.shot} title={area.title} /></div>
        <div className="tour-points">
          {area.points.map(([t, d]) => <div key={t}><strong>{t}</strong><span>{d}</span></div>)}
        </div>
      </div>
    </div>
  );
}
