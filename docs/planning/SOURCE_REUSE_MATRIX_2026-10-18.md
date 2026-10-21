# Source reuse matrix (2026-10-18)

**Rule:** COPY > ADAPT > INTEGRATE > BUILD, used only where a license or a separately retained authorization permits it. Authorization documents are private and are not referenced here. Every row names its evidence in the repository.

**Operations:**
- **COPY:** byte-identical third-party material, kept with its license and notice.
- **ADAPT:** third-party code changed for MedScale, with the upstream file and commit named at the change.
- **INTEGRATE:** an unmodified dependency, pinned and admitted through `deny.toml`.
- **BUILD:** first-party MedScale code.

| Source | License / basis | Operation | What MedScale uses | Evidence |
|---|---|---|---|---|
| OpenMed v3.0.0 `models.jsonl`, `LICENSE`, `NOTICE` (commit `ea920f3`) | Apache-2.0 | COPY | Model catalog bundled in the desktop and listed through Core | `third_party/medscale-model-catalog-v3/`, `third_party/provenance/103-openmed-v3-model-catalog.md` |
| OpenMed v3.0.0 Python source | Apache-2.0 | ADAPT (port to Rust) | Catalog parsing, reproducibility hash, residency policy, document windowing, entity decoding (Spec 103) | Each module names its upstream file and commit; `docs/brand/PRE_RELEASE_IDENTITY_AND_PROVENANCE_AUDIT.md` |
| candle-core 0.9.1 | MIT OR Apache-2.0 | ADAPT (pinned backport) | `zip` 9.0.3 backport adapted from upstream PR #4041, plus a traversal test | `vendor/candle-core-0.9.1/MEDSCALE_PATCH.md` |
| candle-nn / candle-transformers 0.9.1 | MIT OR Apache-2.0 | INTEGRATE | BERT, RoBERTa, DeBERTa-v2, ModernBERT and text-generation runtimes (Spec 104/105) | `Cargo.lock`, `deny.toml` |
| tract (ONNX) | MIT OR Apache-2.0 | INTEGRATE | Portable ONNX token-classification runtime (Spec 069/103) | `Cargo.lock`, `deny.toml` |
| Tauri 2.12 / wry / tao | MIT OR Apache-2.0 | INTEGRATE | Desktop shell on Windows and macOS (Specs 096–101) | `apps/desktop-tauri/src-tauri/Cargo.lock`, tauri dependency policy job |
| React 19, `@tauri-apps/api` | MIT / MIT OR Apache-2.0 | INTEGRATE | Desktop UI | `apps/desktop-tauri/package-lock.json`, release NOTICE |
| Inter, JetBrains Mono | OFL-1.1 | COPY | Desktop typography | `assets/brand/fonts/`, `third_party/provenance/095-native-typography.md` |
| OpenMed models (weights) | Each model card's own license | Acquire on demand, never bundled | Qualification runs on CI from pinned revisions | `evidence/103-local-model-catalog/RIGHTS_LEDGER.md` |
| OpenEvidence | Proprietary; studied only through public, unauthenticated material | BUILD (no code or content reuse) | Product-pattern reference only: evidence-cited answers | Not open source; no code, prompts or content copied |
| Abridge | Proprietary; studied only through public, unauthenticated material | BUILD (no code or content reuse) | Product-pattern reference only: ambient documentation | Not open source; no code, prompts or content copied |
| MedAgent workbench, Model Fleet, evaluation gold sets, entity evaluation | First-party | BUILD | Spec 077 desktop flow; synthetic gold sets and scorer (Spec 103) | `apps/desktop-tauri/src-tauri/src/cmd_ops.rs`, `scripts/evaluation/`, `crates/medscale-pack/src/entity_eval.rs` |

## REA (reverse engineering) usage

- `rea-agents` 6.7.0 (MIT) runs from npm. Its `setup` is interactive and rewrites agent configuration, so it was not run unattended.
- REA was not used on OpenEvidence or Abridge applications. Reverse engineering a proprietary product is not treated as permission to reuse it. Any study of those products stays with public, unauthenticated pages, and nothing is copied.
- OpenMed is open source, so its source was read directly; REA adds nothing there.

## Not claimed

- No row implies clinical validation, superiority over any product, or rights to model weights or datasets.
