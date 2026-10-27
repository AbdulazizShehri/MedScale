# Experiment A0 — Contract-level provenance controls

**Status:** implementation added; execution result pending CI.

**Scientific base:** `1e2b7d94e970256b38bda15fa91f62bc397e825a`

**Harness location:** `crates/medscale-contracts/tests/paper_provenance_contracts.rs`

## Purpose

A0 establishes the smallest validated controls needed before system-level provenance mutation experiments. It deliberately does **not** claim that Core, storage, collaboration, recovery, or external-effect paths reject every corresponding mutation.

## Pre-specified cases

| Case | Valid control | Mutation / condition | Expected contract behavior |
|---|---|---|---|
| A0-01 | Source bytes and SHA-256 agree | mutate bytes, retain digest | `digest_valid() == false` |
| A0-02 | Source bytes and SHA-256 agree | mutate digest, retain bytes | `digest_valid() == false` |
| A0-03 | Two sources may contain identical bytes | use distinct source ids with identical bytes | digest equal; source identity unequal |
| A0-04 | Existing citation may support | set `citation_exists=no` while relation remains `supports` | validation error |
| A0-05 | Non-retracted citation may support | set `retracted=yes` while relation remains `supports` | validation error |
| A0-06 | Fully recorded support | population applicability becomes `unknown` | valid assessment; verdict `unknown` |
| A0-07 | Existing, non-retracted, supporting, applicable citation | no mutation | verdict `supported` |

## Interpretation boundary

A green A0 result establishes contract-level behavior only. It is a prerequisite/control suite for A1 system-level experiments. It must not be cited as evidence that malformed objects cannot enter storage through every path.

## Planned execution

```bash
cargo test -p medscale-contracts --test paper_provenance_contracts
```

The authoritative result must record exact harness SHA, CI run, platform, test counts, and output. Until execution is observed, the manuscript must not report A0 as PASS.
