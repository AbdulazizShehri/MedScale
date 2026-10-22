export const github = "https://github.com/AbdulazizShehri/MedScale";
export const released = false; // Explicitly false until a signed, qualified public release exists.
export const basePath = process.env.NEXT_PUBLIC_SITE_BASE_PATH || "";
export const asset = (path: string) => basePath + path;
export const docs = {
  start: github + "/blob/main/README.md",
  desktop: github + "/blob/main/apps/desktop-tauri/README.md",
  cli: github + "/blob/main/docs/product/CLI_PRODUCT_GUIDE.md",
  qualification: github + "/blob/main/docs/qualification/EXTERNAL_GATES.md",
  contribution: github + "/blob/main/CONTRIBUTING.md",
  security: github + "/blob/main/SECURITY.md",
  governance: github + "/blob/main/GOVERNANCE.md",
  source: github + "/tree/main/crates",
  discussions: github + "/discussions",
  issues: github + "/issues",
  releases: github + "/releases"
};
export const productAreas = [
  { id:"records", index:"01", eyebrow:"RECORDS + TIMELINES", title:"Health information with a history.", text:"Explore longitudinal, synthetic patient records alongside timelines, documents, laboratory results, and source context.", chips:["Longitudinal context","FHIR R4 fixtures","Source provenance"], rows:["Subject timeline","Evidence-backed documents","Coverage + source detail"], mode:"Patient workspace" },
  { id:"evidence", index:"02", eyebrow:"SOURCE-AWARE EVIDENCE", title:"Trace the insight to its origin.", text:"Make uncertainty and review status visible. Preserve source distinctions instead of treating every generated statement as a verified fact.", chips:["Provenance graph","Human review","Audit trail"], rows:["Imported record → linked evidence","Summary → explicit review","Audit → inspectable trail"], mode:"Evidence workspace" },
  { id:"models", index:"03", eyebrow:"MODEL CENTER", title:"Know what a model can do — and what it cannot.", text:"Inspect available model sources, local runtimes, pack admission, and qualification status without treating a catalog listing as clinical approval.", chips:["Local model catalog","Runtime state","Admission controls"], rows:["Source identity + integrity","Runtime + device expectation","Qualification ≠ installed"], mode:"Model workspace" },
  { id:"workflow", index:"04", eyebrow:"CONTROLLED WORKFLOWS", title:"Keep actions reviewable.", text:"Connect governed workflow steps to source-backed contexts. Suggestions remain distinct from authorized changes.", chips:["Tasks","Reviewable steps","Bounded operations"], rows:["Prepare context","Inspect evidence + policy","Review before action"], mode:"Workflow workspace" }
] as const;
