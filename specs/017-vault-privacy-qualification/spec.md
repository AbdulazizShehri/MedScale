# Feature Specification: Vault Privacy Lifecycle Qualification (Q03)

**Feature Branch**: `spec/017-vault-privacy-qualification`  
**Created**: 2026-09-09  
**Status**: Package complete; READY for implementation after analyze  
**Depends on**: Spec 016 `CLOSED_CANONICAL` (durable record), Spec 005 EncryptedVault  
**Does not**: authorize REAL_PHI; claim unconditional `PRIVATE_DATA_READY` without measured OS evidence; admit SQLCipher until dedicated crate isolation; implement Q04 sessions.

## User Story (P1)

An operator using EncryptedVault (synthetic data) opens a vault, writes authority objects, and closes it. After close, plaintext working metadata files (`meta.work.sqlite3`, WAL/SHM/journal sidecars) are absent. Doctor reports honest privacy posture. Crash leftovers are detected and cleaned or refused explicitly—never silently treated as sealed.

## Requirements

- **FR-001**: Inventory open-vault plaintext surfaces and document each.
- **FR-002**: On EncryptedVault close, seal meta then remove work file and SQLite sidecars (`-wal`, `-shm`, `-journal`).
- **FR-003**: On open, detect unexpected leftover work/WAL from prior crash; clean or refuse with typed error.
- **FR-004**: Doctor/privacy status distinguishes sealed-at-rest vs open-vault residual risk; must not claim PRIVATE_DATA_READY until acceptance criteria met.
- **FR-005**: ADR records SQLCipher deferred (crate isolation) vs AES-GCM sealed work-file path retained/strengthened.
- **FR-006**: SyntheticVault remains synthetic-engineering path; privacy claims bind EncryptedVault only.
- **FR-007**: Synthetic-only fixtures.

## Success Criteria

- **SC-001**: After close, work/WAL/SHM absent in tests.
- **SC-002**: Crash-leftover fixture cleaned or refused explicitly.
- **SC-003**: Evidence LIMITATIONS list remaining OS key-custody / snapshot gaps before PRIVATE_DATA_READY.
- **SC-004**: Exact-head CI green; BUILD_QUEUE updates Spec 017 state.

## Out of scope

Q04 host auth; live PHI; SQLCipher enablement in this unit unless ADR reverses with admission record; MESC; UI redesign.
