# Desktop Qualification — Spec 079

`crates/medscale-desktop/src/privacy_workspace.rs` plus a "Privacy" route in
`ui/app.slint` (Governance section), backed only by `CliSession`.

Tests pass in run `35802759307`:
- `privacy_view_models_flow_through_a_real_core_session` (refresh, egress
  deny/allow, invalid input refused, newest-first decisions, receipt revoke,
  stale revoke reported as a conflict, egress denied after revocation);
- `unknown_project_reports_an_explicit_status`.

The Slint markup compiled in the exact-head CI build on all three platforms.
Accessible roles and labels are set on every row group.

Scope: profiles, transforms and re-identification are CLI-only in this
slice (same pattern as Specs 077/078). No rendered screenshot exists (no CI
rendering step), the same honest residual as Specs 075-078.
