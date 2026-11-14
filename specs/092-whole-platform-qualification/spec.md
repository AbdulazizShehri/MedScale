# Spec 092 — Whole-Platform Qualification

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promotion date:** 2026-09-27
**Canonical base:** `bef016f009fdbe676c342a0b467a2166881c0349` (Spec 091 closure merge)
**Target branch:** `spec/092-whole-platform-qualification`

## Authority

`RESEARCH_OS_V2_SPEC_IMPLEMENTATION_CONTRACTS.md` §092: hard dependency
074-091 as actually promoted and implemented; goal "integrated
qualification of Research OS as a coherent product rather than separately
passing features"; terminal rule: "No subsystem's local PASS substitutes
for integrated evidence. Pending/external/unavailable stays explicit.
Whole-platform qualification may establish a release profile only for the
exact features/platforms/data classes actually proven."

## Scope

A qualification campaign, not a documentation sweep:

1. **Integrated campaign test** (`whole_platform_092`): one synthetic vault
   through every Research OS plane via Core (075 snapshot, 082 analytics,
   083 knowledge index, 085 compute, 086 R workspace, 087 extensions, 088
   huddles, 089 research packs, 090 adapters, 091 federation), then
   backup, restore into a new directory, every storage consistency
   verifier on the restored copy, restart through Core, state read back,
   documented restore losses asserted (federation identity secret), and
   honest product defaults asserted after restart.
2. **Qualification matrix** (`evidence/092-whole-platform-qualification/MATRIX.md`)
   classifying 35 areas as `PROVEN`, `IMPLEMENTED`, `PARTIAL`, `MISSING`,
   `DEFERRED`, `EXTERNAL` or `N/A`, each with evidence or the gap, bound to
   `EXTERNAL_GATES.md`.
3. **Release profile**: the only profile this campaign can establish is
   *local Personal/Lab, synthetic data, CLI through Core, hosted CI
   runners*. It establishes no production release.

Exact-head CI on Linux, Windows and macOS is the execution evidence.

## Explicitly not claimed

`RELEASE_READY`, `PRIVATE_DATA_READY`, `PLATFORM_QUALIFIED`, clinical
validation, regulatory status, performance on qualified hardware, rendered
accessibility, signed packages — all remain false or external.

## Completion rule

`CLOSED_CANONICAL` only after merge on a green exact head and recorded
post-main verification; then the final completion audit.
