# Spec 072 Rebuilt-Product Qualified-Hardware Performance Action Packet

Hosted CI and development-host measurements are feasibility evidence only. This packet is required before qualified-hardware performance-budget attainment is claimed for the rebuilt product.

## Hardware/software binding

For each Windows, macOS, and Linux release target, record CPU, RAM, storage, GPU/display path where relevant, OS build, power mode, display scaling, Rust/toolchain, exact commit/tree, Cargo.lock digest, package digest, and whether the machine is the declared minimum or reference release hardware.

## Measurements

- **Cold model-free Desktop launch** — 3 warmups + 30 timed runs; preserve raw samples and report p50/p95; target p95 <= 2000 ms.
- **Final route interaction response** — measure route changes and representative native state updates for Home, Patients, Documents, Insights, Models, Evidence, Workflows, Tasks, Messages, Audit Trail, Exports, Integrations, Settings, About, and command palette; preserve raw samples and report p50/p95; target <= 100 ms under the canonical methodology.
- **Models interaction scope** — measure route entry/render, local session inventory refresh, and representative signed-Pack admission-result projection using only an already-authorized local synthetic/evidence Pack. Do not perform online acquisition or real-PHI/model promotion.
- **Evidence interaction scope** — measure route entry/render and representative claim-state presentation with the pinned 39-row ledger/baseline already embedded in the product; do not convert feature presence into a parity/surpass claim.
- **Model-free idle RSS** — use the locked runtime probe; preserve all samples and report max MiB; target <= 250 MiB.
- **Delivery-plan scale harness** — run the existing bound timeline/lexical/FHIR harness and preserve that evidence independently from UI measurements.

## Honesty rule

A result is attributable only to the exact bound hardware/software/package tuple. Hosted-runner or development-host success cannot set `budgets_claimed_met=true`. Failed or unavailable measurements remain open evidence classes. `FINAL_UI_PRESENT_EXTERNAL_INTERACTION_MEASUREMENT_REQUIRED` remains the correct state until this packet is completed on declared qualified hardware.
