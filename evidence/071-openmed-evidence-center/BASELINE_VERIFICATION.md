# Spec 071 OpenMed Baseline Verification

- Upstream: `https://github.com/maziyarpanahi/openmed`
- Tag: `v2.2.0`
- Commit: `59d9cb0a2e0ccbba8fa3d891a66d83ffaf45e837`
- Tree: `1c949e35b2b8f2ea69da4284b370074fc4bf84ab`
- Verification method: local Git clone of the pinned tag followed by `git rev-parse HEAD`, `git rev-parse HEAD^{tree}`, and exact-tag verification.
- `models.jsonl` rows at the pin: `2266` (dated inventory context only; raw model count is not a parity metric).

Comparator source SHA-256 values:

- `docs/feature-map.md`: `3c819092b5775f12ffaf5c4f4315b68aadcc6acc1e5ff3b0709462c9c2950ad6`
- `docs/eval-harness.md`: `a02cd31d71168bf575cc80d37a214b56c41e8132a5b71e6273f4363df5c53ad8`
- `docs/mlx-backend.md`: `39e2d2be7b177172d2180e4c7c5bd8ba158f1dcb23c5a44742dbda323a73c6ef`
- `docs/model-registry.md`: `270b3538c378f13ae85d7a022ca576ceacff6f79157ade8e44498e03d227977d`
- `docs/clinical-validation-protocol.md`: `1204ddbd9755501d5592400ed4533ccc30cc2238521613cbfb433dd7c42edbeb`
- `docs/anonymization.md`: `1e51a6a2dbf36bf08ac3a7ac948d3053c21513fbfcf8d4b119b09a33e09a9103`

This verification proves comparator identity and source presence only. It does not establish any MedScale parity, surpass, privacy superiority, or runtime winner.
