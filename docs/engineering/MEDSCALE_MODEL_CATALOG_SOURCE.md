# MedScale model catalog — source-neutral engineering contract

MedScale owns the **local catalog interface** and its admission/qualification policy. An imported catalog is data, not the identity of the application or a claim that MedScale created the underlying model weights.

## Native catalog boundary

- The Tauri desktop embeds `third_party/medscale-model-catalog-v3/models.jsonl`, with a small source descriptor at `third_party/medscale-model-catalog-v3/source.json`.
- The descriptor records the **actual repository, immutable source revision, and SHA-256 of the included manifest**. Neither an upstream repository name nor a fabricated MedScale mirror is hardcoded as a CLI default.
- `models_catalog` reads the descriptor using a strict schema and **refuses** a content-digest mismatch before returning rows. Catalog listing does not imply weights are installed, task-qualified, licensed for production, or clinically validated.
- `medscale packs catalog|variants|admit-snapshot|acquire` require an explicit `--repository` catalog source identity. Existing automated qualification workflows supply it explicitly. This is an intentional CLI behavior change: invocation without a source now fails closed rather than silently assuming another project's repository.
- The model-qualification workflows reuse the **version-controlled MedScale catalog bytes**. They verify the SHA-256 against the source descriptor and fail if the workflow's pinned commit/repository differs. This avoids refetching a mutable remote catalog while retaining the exact true source revision.
- Downloading model weights is a separate, governed action. Existing consent and byte-budget flags still apply; no new remote action or implicit download was added.

## Verification

```sh
python3 -m unittest scripts/tests/test_medscale_catalog_bundle.py
python3 scripts/verify_medscale_catalog_bundle.py
cargo test -p medscale-cli --locked
cargo check --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml --locked
```

The first-party catalog verifier uses only the Python standard library and refuses missing metadata, altered bytes, and mismatched workflow pins. GitHub's required supply-chain policy check runs the same verifier. The Tauri unit test compares the metadata SHA with the Rust `ModelCatalog::import` digest.

## Remaining native-model work

This is a **native catalog/integration layer**, not native training of every listed weight. The installed external model IDs remain factual source identifiers, and the optional benchmark test runner remains an external comparison. Replacing actual model weights/runtimes requires a real MedScale-owned implementation or a correctly attributed distribution, with independently reproducible benchmarks and versioned compatibility tests.

Historical research receipts and required notices remain separate from first-party UI branding. No new release was created. `REAL_PHI=NOT_AUTHORIZED`.
