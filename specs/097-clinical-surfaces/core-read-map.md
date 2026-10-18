# Core read map — next governed slice

This is an implementation plan, not current frontend data or a new capability.
The current Patients index remains unavailable until the 096 adapter exists.

## Owned subject context

Use an explicitly owned synthetic Host and a fixed Rust-held subject context.
The existing presentation golden has Ada Synthetic (`pres-patient`, identifier
P-004), not Sarah Chen. It is a source fixture, not an ingested record in this app.
Do not change its name or silently reinterpret the Slint view as a Core roster.

Any future fixture import must use governed Core ingest and explicit synthetic
promotion. It must record source/proposal/assertion identities separately; the
frontend cannot auto-promote or construct clinical authority. No automatic
private vault discovery or broad CliSession operator session is acceptable.

## Supported reads and mapping

| Rust contract | Surface mapping | Limitation |
|---|---|---|
| SubjectBriefV1.sections.identity | Compact identity with field coverage and citation refs | Display name may be missing, unknown or conflicting; never synthesize one. |
| SubjectBriefV1.sections.conditions | Supported condition facts with literal coverage | Empty values alone do not establish clinical absence. |
| SubjectTimelineV1.events | Longitudinal section with effective/recorded time and assertion/source refs | Preserve missing time and source identity; do not imply freshness or review. |
| SubjectCoverageV1 | Explicit coverage slots and conflict/unknown/unit distinctions | Evidence quality is distinct from clinical review status. |
| DrillDownPresentation | Bounded provenance inspector | Named Rust operation only, no raw source bytes, paths or generic request JSON. |

There is no inspected general roster contract. One owned subject context can
produce one supported row once the Core reads are real and qualified; it cannot
establish a complete workspace patient count. The UI must label that scope.

## Execution gates and failure semantics

First satisfy specs/096-tauri-foundation/ipc-contract.md, including Rust-owned
session/lease/context, request/schema correlation, bounded fields and denial
tests. Add only explicit domain commands and generated minimum capabilities.
Unavailable, denied, unsupported and corrupt results never become empty success.
Projection data is rebuildable and non-authoritative. No LLM or runtime egress.

## Adapter implementation decisions — 2026-10-01

The post-render source trace identifies concrete work before a patient read can
be installed. These are preparation decisions, not implemented commands or test
results. T096-05 and T097-04 remain open.

| Observed production path | Consequence for the adapter |
|---|---|
| `ipc/mod.rs:102` accepts an explicit capability allowlist, while `into_background` at line 108 calls the unrestricted stream handler. | Do not use the convenience unrestricted background loop. The owned reader transport must retain the four presentation reads as its fixed allowlist throughout its lifecycle. Bootstrap/setup authority stays Rust-owned and outside that reader endpoint. |
| `HostIpcClient::dispatch` at line 173 serializes a request and deserializes the response without checking request ID/schema. | Compare response request ID and authority schema against the pending Rust request before inspecting or mapping result data. An unexpected response variant is a protocol error, including on otherwise valid JSON. |
| `AuthorityRequest::new` sets a 5,000ms deadline and 1MiB response budget; `ipc/framing.rs` bounds frame length but uses blocking `read_exact`. | A deadline field is not proof of a transport timeout. Qualify stalled-prefix/payload handling and bounded shutdown before native installation; do not wrap an indefinite read in detached per-request threads or claim timeout from a JSON field. |
| `authority/facade.rs:679` starts strict session validation, but read/health requests may omit a session; presentation reads at line 1181 require a lease. | Always attach the explicit narrow reader session. Check owned context first and preserve Core's expired/revoked/ungranted session errors; an existing lease alone does not prove that the presentation caller is admitted. |
| `tests/h0b_presentation_coverage.rs` uses `new_legacy_lease_only_engineering` with ingest/create-proposal/promote setup. | Use the golden fixture as data evidence, not that legacy constructor as the new security model. New lifecycle tests must use strict Core and separate explicit synthetic setup authority from reader grants. |

For the first one-subject slice, prefer zero-selector commands for the Rust-held
context: `get_owned_subject_brief`, `get_owned_subject_timeline`, and
`get_owned_subject_coverage`. These names are proposed, not current allowlist
entries. The frontend supplies no vault, realm, scope, session, endpoint or
arbitrary subject ID. A future provenance command accepts only a closed field
selector and an opaque reference issued for that same context; its lookup is
Rust-owned. It never accepts a free-form capability, object path or source bytes.

The setup cannot silently ingest/promote a fixture on launch. Governed synthetic
preparation must preserve its source/proposal/assertion receipts and prove that
the opened Host and its subject context belong to that preparation. Until the
setup and lifecycle are implemented and qualified, retain unavailable display.
One admitted subject may establish one row; it still cannot establish a complete
patient roster or workspace-wide total.

### Required qualification matrix for the next slice

- Strict Core reader grants allow only brief, timeline, coverage and bounded
  presentation drilldown. Ingest, promotion, session bootstrap, raw object/file,
  network and generic bridge operations are refused at the reader endpoint.
- Missing ownership, foreign context/reference and missing/expired/revoked
  session fail before clinical DTO publication. Never map them to an empty list.
- Wrong schema, request ID or result variant, oversized frame, stalled prefix or
  body, EOF, poison/lock failure and owner shutdown yield bounded explicit errors.
- Synthetic fixture setup receipts establish the actual identity and lineage;
  proposals alone cannot appear as promoted assertions or populated rows.
- Existing generated-capability tests remain on the production handler builder.
  Install only the qualified named reads and repeat denied-origin/window/generic
  capability tests. No plugin, broad handler or permissive mock context shortcut.
- Inspect the actual populated row in both native themes/targets before Patient
  Detail and Evidence implementation. The current unavailable render evidence
  cannot qualify that later data state.

Inspect and qualify the resulting actual row before implementing Patient Detail
and Evidence. Keep the current unavailable presentation when a read fails.
