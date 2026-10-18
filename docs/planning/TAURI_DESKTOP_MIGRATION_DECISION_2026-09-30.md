# Desktop presentation migration decision

**Status:** founder directed; staged implementation authorized, qualification open

**Date:** 2026-09-30

**Scope:** Specs 096–100 and subsequent desktop retirement decision
**Supersedes for future desktop presentation work:** the Slint implementation choice in the 2026-09-29 productization program. It does not rewrite Spec 095 or the closed Spec 006 record.

## Decision

MedScale Desktop will migrate its presentation layer from Slint to **Tauri 2 + React + strict TypeScript + Vite + Tailwind**. The existing Rust Core remains the only trusted authority for evidence truth, provenance, storage, authorization, effect control, network policy, FHIR interchange, model execution authority, recovery and trusted state transitions. The frontend renders Core-derived view data and sends typed intents. It never owns clinical or security authority.

Slint remains the working legacy reference until the Tauri product has route, behavior, privacy and native evidence parity. A separate retirement decision must verify parity, package operation, rollback and the absence of required Core dependencies on Slint before any removal. The migration is reversible and is split into bounded units.

## Trust boundary

```text
React presentation intent
  -> narrow typed Tauri command
  -> bounded input validation
  -> Core Host authorization/policy
  -> existing Rust Core operation
  -> explicit result DTO
  -> React view model
```

The Tauri process is a presentation adapter, not an alternate store or policy engine. It must use the existing single-writer Core Host/authority path, including its capability and session semantics. The frontend may hold ephemeral display state; it may not treat a copied DTO or browser storage as canonical. No direct vault/database/key access, generic Core dispatcher, arbitrary SQL/shell/path/network bridge, ambient privileged plugin or hidden egress is admitted. External-action `UNKNOWN` is never automatically retried. `Proposal` remains distinct from `ClinicalAssertion`.

The first IPC slice is read-only and synthetic-only. Command names and DTOs must be derived from actual Core contracts. Each subsequent mutation requires its own typed intent, validation, authorization, explicit outcome and test. Tauri capabilities start from default deny; each permission and plugin requires a recorded need. No broad filesystem, shell, process, HTTP, clipboard or window grant is included for convenience. Production CSP, URL/navigation, asset loading, WebView storage/cache/crash behavior and external links require a security review before promotion.

## Privacy admission and release limits

Spec 006 deferred Tauri/WebView admission pending PHI containment proof. This founder decision selects a future presentation target; it does **not** establish that proof or authorize real PHI. The new shell may be developed and evaluated only with synthetic data while the WebView privacy gate remains open. A WebView build must not be distributed or described as private-data ready before cache/crash/log/storage behavior is measured and the gate is independently resolved. The existing non-WebView Desktop remains available throughout migration.

`RELEASE_READY=false`, `PRIVATE_DATA_READY=false` and `MULTI_CLIENT_RELEASE_READY=false`. No clinical, regulatory, production, WCAG, cross-platform native or performance superiority claim follows from changing UI technology. Useful local operation must remain offline without a cloud account, CDN font, remote script or remote model. Development-only public dependency retrieval is governed separately from product runtime egress.

## Design and implementation constraints

The approved paired-M geometry, Inter, JetBrains Mono NL and black/white identity remain authoritative. The dark Patients reference and light brand board anchor visual review. React/Tailwind components must express MedScale's own tokens and information hierarchy; component-library defaults, v0 output, Figma AI and Impeccable critique cannot override the approved brand. UI states distinguish loading, empty, partial, unknown, unsupported, conflicting, error, blocked and review required. No invented patient rows, fake sign-in, live literature claim or hidden clinical authority may appear to fill a design.

The first package uses the repository's first JavaScript package manager only after an explicit lockfile and dependency audit. Third-party dependencies are minimized. Fonts/assets are local. Native Tauri frames, not browser mockups alone, are the visual acceptance artifact.

## Staged program and gates

| Unit | New migration slice | Existing intent retained |
|---|---|---|
| 095 | Historical Slint brand foundation; draft and unmerged while its zero-cost review gate is open | Approved mark, type and token authority; do not retrofit Tauri claims |
| 096 | Tauri shell foundation, React/TypeScript/Vite/Tailwind, local themes, one navigation system, first-party icons, typed read-only IPC and minimal capabilities | All 25 route identities, command navigation, focus and minimum-size density |
| 097 | Home, Patients index, Patient detail and Evidence through truthful Core DTOs | Clinical/evidence source, uncertainty, coverage and explicit states |
| 098 | Research/operational route migration in bounded slices | Existing route capabilities and Core authority |
| 099 | Welcome, truthful local access, accessibility, cross-platform native matrix and privacy qualification | Synthetic demo and complete interaction evidence |
| 100 | Journey, security, performance and visual audit; scoped UI freeze only if evidenced | Final product audit, with no release-readiness inference |

Spec 096 cannot close canonically before 095's required gates and dependency are satisfied. Preparatory branch/PR work may continue without representing that dependency as closed. Split large units into small PRs if needed; do not combine the whole migration into a single change.

## Qualification

Every merge candidate binds base/head/tree/merge-base, changed files and diff check, exact-head required CI, native visual evidence, security review, Jev and Alibaba OCR status, and zero-cost blockers. Paid reviewers and APIs are prohibited. A blocked mandatory independent review remains blocked rather than being replaced with a tool-presence or self-review claim. Native matrix targets include Home, Patients index/detail, Evidence and Welcome/Access at 1440×900 and 1100×720 in both themes, with package/PNG digests and platform recorded. Performance measures startup, package size, idle memory, navigation and first meaningful render before any comparative claim.

## Decision consequences

The team accepts a second temporary desktop implementation and added WebView attack surface while migrating. It keeps the existing Core and Slint application intact, uses narrow interfaces, and withholds merge/retirement/readiness claims until the relevant evidence is available. This decision does not grant PHI, partner, cloud, model, MESC or release authority.
