# Spec 061 Evidence Summary

State: `CLOSED_CANONICAL`.

Spec 061 replaces the Patients placeholder with a native longitudinal patient workspace. The Desktop boundary consumes existing `SubjectBriefV1`, `SubjectTimelineV1`, and `SubjectCoverageV1` semantics and maps them into read-only view state; it does not open storage, keys, or network clients and does not create a second clinical authority path.

The deterministic runtime demonstration is synthetic and visibly labeled. Overview, timeline, labs, and sources preserve provenance and coverage language. Medication, document-understanding, and care-plan areas are explicit non-claims: unsupported trusted medication projection, source custody without OCR/ASR completeness, and review-first care-plan routing with no authority-changing commit in the patient surface.

Slint remains exactly 1.16.1 with femtovg 0.23.2 and AccessKit; Rust MSRV remains 1.88; no Tauri/WebView is admitted. Spec 012/MESC is untouched and remains optional/deferred.

Local qualification passed the full workspace tests, clippy, Rust 1.88 MSRV check, headless Desktop probes, and diff check. Timeline now includes an explicit keyboard-reachable `Inspect sources` affordance that routes to the provenance/source view without changing authority.

The first PR #105 pull-request Windows run exposed insufficient process-lifetime allowance for hosted-runner `tasklist` RSS sampling while the duplicate push run on the same SHA passed Windows, proving timing sensitivity. The bounded correction increases only the Windows engineering idle-probe window to 10 seconds while retaining five samples and fail-closed RSS requirements; targeted runtime-perf tests and clippy pass locally, and the change does not weaken performance or release gates.


Canonical qualification closed on final exact head `ee9e076fd39d4832c2037b077af18979e428b420`: pull-request run `34916956439` passed all six required jobs, PR #105 merged without bypass as `1438460e1b965518a9cd09d4289e842c016d83df`, and post-merge main run `34917999834` passed all six required jobs with cross-platform portable package and runtime-performance evidence artifacts. This closes Spec 061 only; it does not claim real-PHI authorization, release readiness, qualified-hardware budget attainment, signing/notarization, or WCAG conformance.
