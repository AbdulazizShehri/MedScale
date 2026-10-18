# Read-only Core bridge contract and implementation frontier

The current native allowlist contains only `get_shell_status`. Its version-1 DTO reports unavailable Core/workspace connectivity and synthetic-only scope; it does not inspect a live vault. TypeScript rejects a malformed or mismatched native response. This is scaffolding, not completed Core Host integration.

## Existing authority path

The existing implementation is `medscale_core::HostIpcClient` / `HostIpcServer` in `crates/medscale-core/src/ipc/mod.rs`, with length-prefixed `AuthorityRequest` / `AuthorityResponse` frames. The server dispatches through `CoreFacade`; `serve_connection_limited` rejects capabilities outside an explicit allowlist before Core dispatch. `CliSession::connect` creates a broad operator session and therefore must not be used as the frontend's automatic authority shortcut.

Core already defines `GetBrief`, `GetTimeline`, `GetCoverage` and `DrillDownPresentation`, returning the corresponding presentation contracts. It does not supply a patient roster/list API in the inspected contract. A Patients index must use an actually admitted subject inventory or the one owned synthetic subject context; it must not manufacture a roster from decorative rows.

## First adapter requirements

1. Rust owns the endpoint, vault/realm/scope, lease/session and allowed subject context. None are frontend-controlled request fields or browser storage.
2. Initial connection must be to an explicitly owned synthetic Host, with no automatic attachment to an arbitrary existing endpoint or private vault. The app may not infer synthetic-only scope from an unverified remote response or naming convention.
3. Expose named reads derived from Core: subject brief/timeline/coverage and bounded provenance inspection. Never expose `AuthorityRequest`, a capability selector, a Core method name, a path or arbitrary JSON as a generic invoke payload.
4. Validate bounded subject IDs and current allowed context before dispatch. Return unavailable when unattached; denied when scope/session is invalid; unsupported when the requested projection is unsupported. Do not convert denied/unknown into empty data.
5. Core evaluates its existing lease/session/capability rules. Strict Core currently requires a client session for mutations; read/health operations have distinct semantics and may require a lease. The adapter must retain an explicit narrow session for scoped reads rather than assuming that a read is automatically authorized.
6. Correlate response request ID and schema version before mapping typed DTOs. A transport timeout, corrupt frame, unexpected variant or poison/lock failure is an explicit error with no clinical authority claim. Retry rules must remain operation-specific; no effects or generic retries exist in this slice.
7. Keep raw diagnostics, tokens, filesystem paths and source bytes out of user-facing errors. Rust owns the source/proposal/assertion distinctions; DTO rendering does not create authority.

Required adapter tests: unknown/unattached context; oversized/malformed/foreign subject ID; unauthorized/revoked session; wrong response ID/schema/variant; bounded provenance; one truthful synthetic context; denial of any write/network/file/shell capability. Native capability-negative tests and keyboard/visual evidence remain separate.

## Status

Contract tracing is complete for this bounded design. The Rust adapter/session lifecycle and actual Core-derived read DTOs are **not implemented**. T096-05 remains open. This planning record does not qualify the shell or authorize a private-data connection.

Four invoke-boundary tests now use the production handler builder and generated capability context, rather than an unrestricted mock context. They cover an admitted local main-window status read, rejected remote origins, a rejected ungranted window, and refused generic SQL/shell/file/Core/plugin commands. The Tauri MockRuntime is test infrastructure, not native WebView containment evidence. Outcomes must be recorded from compiled native CI; adding the tests alone is not PASS.
