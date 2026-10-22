"use client";
import { useState } from "react";

type State = "supported" | "conflict" | "missing";
type Claim = { id: string; text: string; state: State; label: string; why: string; source?: { doc: string; quote: [string, string, string] }; second?: { doc: string; quote: [string, string, string] } };

// Entirely synthetic. "Ada Synthetic" is the fixture subject used by MedScale's own tests.
const claims: Claim[] = [
  {
    id: "c1", text: "has asthma", state: "supported", label: "Supported",
    why: "The statement matches a condition recorded in the subject's synthetic record.",
    source: { doc: "condition-asthma.json · synthetic-subject-ada", quote: ["Condition: ", "asthma, persistent", " · clinical status active"] },
  },
  {
    id: "c2", text: "uses a rescue inhaler twice a week", state: "conflict", label: "Conflict",
    why: "Two sources disagree. MedScale shows both instead of picking one.",
    source: { doc: "visit-note-2019-11.txt · synthetic", quote: ["Reports using the rescue inhaler ", "about twice a week", "."] },
    second: { doc: "visit-note-2019-12.txt · synthetic", quote: ["Rescue inhaler used ", "daily for the past two weeks", "."] },
  },
  {
    id: "c3", text: "no known drug allergies", state: "missing", label: "No evidence",
    why: "Nothing in the bound sources supports this. The gap stays visible and the statement is not treated as fact.",
  },
];

export function EvidenceExample() {
  const [active, setActive] = useState("c2");
  const claim = claims.find((c) => c.id === active) ?? claims[0];
  const btn = (id: string) => {
    const c = claims.find((x) => x.id === id)!;
    return <button type="button" className="claim" data-state={c.state} aria-pressed={active === id} onClick={() => setActive(id)}>{c.text}</button>;
  };
  return (
    <div className="evidence" role="group" aria-label="Interactive example of source-aware evidence with synthetic data">
      <div className="evidence-note">
        <div className="evidence-meta"><span>Draft summary · Ada Synthetic</span><span>Proposal · not reviewed</span></div>
        <p className="evidence-text">The subject {btn("c1")}, {btn("c2")}, and has {btn("c3")}.</p>
        <div className="evidence-legend" aria-hidden="true">
          <span><i style={{ background: "var(--blue)" }} /> Supported</span>
          <span><i style={{ background: "var(--orange)" }} /> Conflict or no evidence</span>
        </div>
        <p className="body" style={{ fontSize: 14 }}>Select a highlighted phrase to see what supports it.</p>
      </div>
      <div className="evidence-side" aria-live="polite">
        <span className="state-tag" data-state={claim.state}>{claim.label}</span>
        <p className="body" style={{ color: "var(--ink)" }}>{claim.why}</p>
        {claim.source && <SourceCard {...claim.source} state={claim.state} />}
        {claim.second && <SourceCard {...claim.second} state={claim.state} />}
        {!claim.source && <div className="source-card"><small>Bound sources · 2 documents</small><p className="body" style={{ fontSize: 14 }}>No sentence in the bound sources mentions allergies.</p></div>}
      </div>
      <div className="evidence-foot"><span>Interactive example · synthetic data</span><span>Illustrates MedScale&apos;s evidence states; not a screenshot</span></div>
    </div>
  );
}

function SourceCard({ doc, quote, state }: { doc: string; quote: [string, string, string]; state: State }) {
  return <div className="source-card" data-state={state}><small>{doc}</small><q>{quote[0]}<mark>{quote[1]}</mark>{quote[2]}</q></div>;
}
