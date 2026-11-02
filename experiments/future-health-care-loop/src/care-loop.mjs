/**
 * Synthetic-only, non-clinical FHIR R4-style care loop evaluator.
 * Independent competition experiment, NOT integrated into MedScale Core.
 * No network, model decisions or actual care recommendations.
 * Apache-2.0.
 */
export const DEMO_SYSTEM = "https://medscale.si/future-health/synthetic";
const ref = (r) => r && r.resourceType && r.id ? r.resourceType + "/" + r.id : null;
const linked = (items, target) => Array.isArray(items) && items.some(x => x?.reference === target);
const coded = (r, c) => r?.code?.coding?.some(x => x.system === DEMO_SYSTEM && x.code === c) === true;
const output = (status, evidence, warnings = []) => Object.freeze({
  status, canClose: status === "closed", syntheticOnly: true,
  evidence: [...evidence], warnings: [...warnings]
});

export function evaluateCareLoop(bundle, {now = "2026-11-03T00:00:00Z"} = {}) {
  const evidence = [];
  const block = (reason) => output("blocked", evidence, [reason]);
  const missing = (status, reason) => output(status, evidence, [reason]);
  if (!Number.isFinite(Date.parse(now))) return block("Invalid evaluation timestamp");
  if (bundle?.resourceType !== "Bundle" || bundle.type !== "collection"
    || !Array.isArray(bundle.entry) || !bundle.entry.length || bundle.entry.length > 250) {
    return block("Expected nonempty collection Bundle of at most 250 entries");
  }
  const records = bundle.entry.map(x => x?.resource);
  const keys = new Set();
  for (const r of records) {
    const key = ref(r);
    if (!key || !/^[A-Za-z][A-Za-z0-9]*\/[A-Za-z0-9.-]{1,64}$/.test(key))
      return block("Invalid resource type or ID");
    if (keys.has(key)) return block("Duplicate resource: " + key);
    keys.add(key);
  }
  const get = (type) => records.filter(x => x.resourceType === type);
  const patients = get("Patient");
  if (patients.length !== 1 || !patients[0].meta?.tag?.some(x => x.code === "synthetic"))
    return block("Exactly one explicitly synthetic patient required");
  const patient = ref(patients[0]);
  const hasProof = (r) => get("Provenance").some(p =>
    linked(p.target, ref(r)) && Number.isFinite(Date.parse(p.recorded ?? ""))
    && p.agent?.some(a => a?.who?.reference?.startsWith("Practitioner/synthetic-")));
  const one = (xs, label) => {
    if (xs.length > 1) throw Error("Ambiguous " + label + ": multiple matches");
    return xs[0] || null;
  };
  const confirm = (r, label) => {
    if (!hasProof(r)) throw Error("Missing reviewer provenance for " + label);
    evidence.push(ref(r));
  };
  const same = (r, field) => r?.[field]?.reference === patient;

  try {
    const screen = one(get("Observation").filter(o =>
      coded(o, "positive-screen") && o.status === "final" && o.valueBoolean === true), "screening");
    if (!screen) return missing("unknown", "No final positive synthetic screening");
    if (!same(screen, "subject")) return block("Screening patient link invalid");
    confirm(screen, "screening");
    let status = "screened";

    const review = one(get("Task").filter(t => coded(t, "reviewed")), "review");
    if (!review) return missing(status, "Human review not recorded");
    if (review.status !== "completed" || !same(review, "for") ||
      review.focus?.reference !== ref(screen)) return block("Review not completed for screening/patient");
    confirm(review, "review");
    status = "reviewed";

    const request = one(get("ServiceRequest").filter(r => linked(r.reasonReference, ref(screen))), "referral");
    if (!request) return missing(status, "Linked referral missing");
    if (!["active", "completed"].includes(request.status)
      || request.intent !== "order" || !same(request, "subject")) return block("Invalid referral");
    confirm(request, "referral");
    status = "referred";

    const appt = one(get("Appointment").filter(a => linked(a.basedOn, ref(request))), "appointment");
    if (!appt) return missing(status, "Linked appointment missing");
    if (appt.status === "cancelled") return block("Appointment cancelled");
    if (!["booked", "fulfilled"].includes(appt.status) ||
      !appt.participant?.some(p => p.actor?.reference === patient)) return block("Appointment patient/status invalid");
    confirm(appt, "appointment");
    status = "appointment-known";

    const visit = one(get("Encounter").filter(e => linked(e.appointment, ref(appt))), "encounter");
    if (!visit) {
      if (Number.isFinite(Date.parse(appt.start ?? "")) && Date.parse(appt.start) < Date.parse(now))
        return missing("overdue", "Appointment passed without evidence of attendance");
      return missing(status, "Attendance not evidenced");
    }
    if (visit.status !== "finished" || !same(visit, "subject")) return block("Encounter invalid or unfinished");
    confirm(visit, "encounter");
    status = "attended";

    const result = one(get("Observation").filter(o =>
      coded(o, "followup-result") && linked(o.basedOn, ref(request))), "result");
    if (!result) return missing(status, "Follow-up result missing");
    if (result.status !== "final" || !same(result, "subject")
      || result.encounter?.reference !== ref(visit)) return block("Result not final or linked");
    confirm(result, "result");
    status = "result-received";

    const close = one(get("Task").filter(t => coded(t, "closed")), "closure");
    if (!close) return missing(status, "Human closure confirmation missing");
    if (close.status !== "completed" || !same(close, "for")
      || close.focus?.reference !== ref(result)
      || !linked(close.basedOn, ref(request))) return block("Closure linkage invalid");
    confirm(close, "closure");
    return output("closed", evidence);
  } catch (error) {
    return block(error instanceof Error ? error.message : "Invalid synthetic evidence");
  }
}
