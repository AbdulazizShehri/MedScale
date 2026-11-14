# CLI Qualification — Spec 079

Module `crates/medscale-cli/src/privacy_gate.rs`, command `medscale privacy`
(17 subcommands, human and JSON output). Tests pass in run `35802759307`:

- `parse_rules_fills_unnamed_kinds_and_refuses_bad_input`
- `privacy_commands_run_through_core_across_fresh_sessions`: source add,
  bad class refused, profile create, map create, transform, egress allow and
  deny, operator-session re-identification refused, audited
  re-identification, then read commands in fresh sessions (human and JSON),
  unknown boundary refused, receipt revoke, and egress denied afterwards.

Transforms from the CLI always use `synthetic_only = true`. The CLI installs
the OS key store (when available) before its first key-using call.
Limitation: the OS keyring path itself is not exercised in CI tests, which
use an in-memory store.
