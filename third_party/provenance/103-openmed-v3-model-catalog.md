# Provenance: OpenMed v3.0.0 model catalog (Spec 103)

| Field | Value |
|---|---|
| Component id | `openmed-v3.0.0-models-jsonl` |
| Upstream URL | `https://github.com/maziyarpanahi/openmed` |
| Revision / tag | `v3.0.0` / `ea920f36fadd7b45935247d639f0ffa1ef493b23` (the pinned catalog commit used by `model-qualification-batch.yml`) |
| Original path(s) | `models.jsonl`, `LICENSE`, `NOTICE` |
| Content identity (hash) | `models.jsonl` sha256 `805e79f7db5514cf308c6deffa67d72c4ce9a59bf4e54487d2783eecbb1e34ab` (2,266 rows, 3,057,702 bytes); `LICENSE` sha256 `c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4`; `NOTICE` sha256 `94bcb50459cf2603cb9ce8cbdeb00b0e09ebd82616844c87931ce7270e4ba1c2` |
| License / NOTICE / permission | Apache-2.0 (upstream `LICENSE`, copied alongside); upstream `NOTICE` copied unchanged |
| Local target path | `third_party/medscale-model-catalog-v3/` |
| Modifications | None. Byte-identical copy |
| Transitive dependencies | None (data file) |
| Security placement | P2: read-only listing data. Core verifies the snapshot against the pinned repository and commit before listing; a listing implies no download or execution |
| Owning MedScale spec | 103 (local model catalog); used by the desktop Models workspace |
| Operation | `COPY_BOUNDED` (catalog data only) |
| Tests | `apps/desktop-tauri/src-tauri/src/feature_tests.rs` (catalog page through the desktop); `crates/medscale-core/tests/model_catalog_capability_103.rs` |
| Update strategy | Replace all three files from a new pinned upstream commit, update the hashes here and the commit constant in `apps/desktop-tauri/src-tauri/src/cmd_ops.rs` |
| Exit strategy | Remove the directory; the desktop then shows no catalog |

## Explicit non-permissions

- The catalog lists models. It grants no rights to model weights or datasets; each model's own card license applies (`evidence/103-local-model-catalog/RIGHTS_LEDGER.md`).
- No OpenMed code is copied by this record.
