# Fuzz Policy (Spec 001 scaffold)

Hostile-input fuzzing is mandatory for later ingest/document/network paths
(Specs 003+). Spec 001 establishes the policy stub only:

1. Prefer `cargo fuzz` targets co-located with the owning crate when fuzz is in scope.
2. Corpus fixtures MUST be synthetic/non-PHI.
3. Fuzz evidence archives under `evidence/<spec>/fuzz/` with commit and toolchain identity.
4. Do not claim fuzz coverage for Spec 001 bootstrap crates beyond unit tests.
