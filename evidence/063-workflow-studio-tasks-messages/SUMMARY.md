# Spec 063 Evidence Summary

State: `CLOSED_CANONICAL` after exact-head CI, normal merge, and successful post-merge main qualification.

Spec 063 makes Workflows, Tasks, and Messages native product surfaces while preserving the existing action authority model. Workflow Studio visualizes evidence→review→payload identity→durable outbox→reconciliation. Tasks are derived review cues over `OutboxEntry`; Messages are local status/review previews only.

`EffectState::Unknown` remains explicitly fail-closed and requires reconciliation before retry. Payload digests remain visible identity bindings. Desktop does not open storage/network clients and does not create or commit controlled actions from these surfaces.

Canonical-branch local qualification passed formatting, workspace Clippy with `-D warnings`, targeted Spec 063 regression, Desktop unit/runtime tests, Rust 1.88 workspace/all-target checking, Desktop smoke/perf probes, and diff checking. The local full-workspace test attempt was storage-limited during linking (`ENOSPC`), with no assertion failure; exact-head CI must prove the full workspace suite before merge. See `LOCAL_QUALIFICATION.md`.
