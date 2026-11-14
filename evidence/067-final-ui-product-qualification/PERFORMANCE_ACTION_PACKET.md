# Final UI Qualified-Hardware Performance Action Packet

Hosted CI and development-host measurements are feasibility evidence only. This packet is required before performance-budget attainment is claimed.

## Hardware declaration
For each Windows, macOS, and Linux release target, record CPU, RAM, storage, GPU/display path where relevant, OS build, power mode, display scaling, Rust/toolchain, commit/tree, Cargo.lock digest, package digest, and whether the machine is the declared minimum or reference release hardware.

## Measurements
- Cold model-free Desktop launch: 3 warmups + 30 timed runs; report raw samples, p50, p95; target p95 <= 2000 ms.
- Final UI interaction response: measure route changes and representative interactive state updates across Patients, Insights, Workflows/Tasks, Audit/Exports/Settings/Integrations, Documents, and command palette; report raw samples, p50, p95; target <= 100 ms according to the canonical methodology.
- Model-free idle RSS: use the locked runtime probe and report all samples plus max MiB; target <= 250 MiB.
- Run the existing delivery-plan scale harness and preserve its evidence independently of UI measurements.

## Honesty rule
A result is attributable only to the exact bound hardware/software/package tuple. Hosted-runner or development-host success cannot set `budgets_claimed_met=true`. Failed or unavailable measurements remain open evidence classes rather than being converted to estimated passes.
