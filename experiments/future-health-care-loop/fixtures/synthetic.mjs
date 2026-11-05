/** Completely fabricated non-patient records. No names, addresses or live identifiers. */
import { DEMO_SYSTEM } from "../src/care-loop.mjs";

const ref = (kind, id) => ({ reference: kind + "/" + id });
const coding = (code) => ({ coding: [{ system: DEMO_SYSTEM, code }] });
export function makeSyntheticBundle() {
  const patient = "Patient/demo-001";
  const screening = "Observation/screen-001";
  const referral = "ServiceRequest/request-001";
  const appt = "Appointment/appointment-001";
  const encounter = "Encounter/encounter-001";
  const result = "Observation/result-001";
  const records = [
    { resourceType:"Patient", id:"demo-001", meta:{ tag:[{ code:"synthetic" }] } },
    { resourceType:"Observation", id:"screen-001", status:"final", subject:{reference:patient},
      code:coding("positive-screen"), valueBoolean:true },
    { resourceType:"Task", id:"review-001", status:"completed", intent:"order", code:coding("reviewed"),
      for:{reference:patient}, focus:{reference:screening} },
    { resourceType:"ServiceRequest", id:"request-001", status:"active", intent:"order",
      subject:{reference:patient}, reasonReference:[{reference:screening}] },
    { resourceType:"Appointment", id:"appointment-001", status:"fulfilled",
      basedOn:[{reference:referral}], participant:[{actor:{reference:patient}, status:"accepted"}],
      start:"2026-10-31T12:00:00Z", end:"2026-10-31T12:30:00Z" },
    { resourceType:"Encounter", id:"encounter-001", status:"finished", class:{system:"http://terminology.hl7.org/CodeSystem/v3-ActCode",code:"AMB"},
      subject:{reference:patient}, appointment:[ref("Appointment","appointment-001")] },
    { resourceType:"Observation", id:"result-001", status:"final", subject:{reference:patient},
      encounter:{reference:encounter}, basedOn:[{reference:referral}],
      code:coding("followup-result"), valueString:"synthetic-nonclinical-result" },
    { resourceType:"Task", id:"close-001", status:"completed", intent:"order", code:coding("closed"),
      for:{reference:patient}, basedOn:[{reference:referral}], focus:{reference:result} },
  ];
  for (const record of records.slice(1)) {
    records.push({
      resourceType:"Provenance", id:"evidence-" + record.id,
      target:[{reference:record.resourceType + "/" + record.id}],
      recorded:"2026-11-01T12:00:00Z",
      agent:[{who:{reference:"Practitioner/synthetic-human-reviewer"}}],
    });
  }
  return {resourceType:"Bundle",type:"collection",entry:records.map(resource=>({resource}))};
}
