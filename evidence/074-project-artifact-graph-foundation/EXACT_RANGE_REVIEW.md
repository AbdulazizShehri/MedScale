# EXACT RANGE REVIEW — Spec 074 (074-F, T074-12)

## Binding (preliminary; re-bound at merge head)

```text
RANGE_BASE=a80c33307afc4577790282652e5b20911beb4bbe (canonical pre-074 base)
RANGE_HEAD=83719f8c3d78dbd2b820236a86b0913dd3c885a (this review; exact-head CI green)
MERGE_BASE_WITH_MAIN=ee8daef3a2782bbdbcb3324766a5d6b95c09fa09 (V2 amendment, merged in)
REVIEW_METHOD=alibaba/open-code-review delegation (ocr v1.12.5:
  deterministic range file selection over origin/main..HEAD, 19 reviewable
  Rust files, system ruleset) executed by the implementation agent; no other
  review tool used. Findings applied: vault-first lock order, checked revision
  conversions with tamper tests, is_multiple_of lint.
```

## Verdicts

```text
AUTHORIZED_SCOPE_ONLY=true
UNEXPECTED_FILES=none
NEW_DEPENDENCIES=none (no Cargo.toml/Cargo.lock delta in range)
NETWORK_AUTHORITY_CHANGE=false (no sockets/listeners/clients; offline preserved)
REAL_PHI=false (synthetic fixtures/vaults only; grep clean)
MESC_CHANGE=false (no mesc paths touched; grep clean)
SPEC_075_PLUS_IMPLEMENTATION=false
NO_DUPLICATE_ID_AUTHORITY_MODEL=true (OpaqueId reused; no ProjectId newtypes)
NO_CANONICAL_TARGET_PAYLOAD_COPY=true (references carry identity/kind/binding only)
```

## Scope audit

Code (all 074-owned): `contracts/project_graph.rs` + envelope vocabulary;
`core/authority/project_graph.rs` + 19 facade arms + capability pairs +
session holder accessor + 17 `CliSession` helpers + default-root helper;
`storage/project_graph.rs` + schema-3 migration + snapshot-3/restore-v3;
CLI `project.rs` + 2 commands + 8 MiB main-thread stack;
Desktop `project_workspace.rs` + Projects route + session wiring;
focused tests per slice. No other crate touched.

Forbidden families (verified by diff file list + case-insensitive grep over
the range; sole hit is the planning filename
`RESEARCH_OS_V2_REPOSITORY_MAP_ADDENDUM.md`): no database connectors, no
Kaggle, no Hugging Face datasets, no Data Source Fabric, no R/RStudio/Posit,
no Community Extensions, no plugin/WASM runtime, no marketplace/registry, no
Hub, no collaboration, no MedAgent, no Analytics, no AudioFlow, no Browse, no
RAG, no Compute. V2 planning docs enter the branch docs-only via the normal
merge of `origin/main` (PR #123) without altering implementation scope.

Docs/evidence: V2 planning amendment (merge), `SPEC_074_PROMOTION.md`,
`specs/074-*/` package incl. frozen contract/migration records,
`evidence/074-*/` qualification set + logs + rendered captures.

Line-ending discipline: `crates/medscale-cli/src/main.rs` range delta is
+58/-0 (CRLF preserved); one intermediate commit carried a whole-file
LF rewrite from a `cargo fmt` run and was immediately repaired by a
whitespace-restoration commit with no force-push and no history rewrite.
Rule for the rest of the lane: `cargo fmt --check` only; manual formatting
fixes via the editor.
