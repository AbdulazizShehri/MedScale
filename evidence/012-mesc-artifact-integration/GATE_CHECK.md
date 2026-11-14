# Spec 012 MESC artifact gate check

**Checked**: 2026-08-26 (independent re-fetch from live GitHub truth)  
**Against**: `TheHalfMoon/MESC`  
**MedScale main**: `18d6eca7055749cf87f2711d42079fe13c2bc7cf`  
**MedScale tree**: `1bf2aa0a308032139ac5a3e07a04bb2e92b94678`

## Live MESC repository observation

| Item | Observation |
|---|---|
| MESC `main` SHA | `4b193c01f5f94447afd359b0420640647b449a69` |
| MESC `main` tree | `3f48f3a9af89d8e82f04c0501019b09084ff860a` |
| `TRAINING_CODE_READY` | `YES` (repository training/readiness code path) |
| `REAL_TRAINING_AUTHORIZED` | `NO` |
| `RELEASE_STATUS` | `BLOCKED` |
| `MEDSCALE_SPEC_012_ADMISSION_READINESS` | `NOT_READY` |

## Live release observation

| Item | Observation |
|---|---|
| GitHub Release | `v0.1.0` — release_id `352847712` |
| Release assets | **`assets: []`** (zero downloadable files; independently re-fetched) |
| Tag `v0.2.0` | Exists but **no** GitHub Release (`404` on `/releases/tags/v0.2.0`) |
| `MESC_RELEASED_ARTIFACT` | `NOT_AVAILABLE` |

## Qualifying ARTIFACT_IMPORT requirements

| Requirement | Status |
|---|---|
| Released immutable artifact bundle (non-empty downloadable assets) | **MISSING** |
| Immutable/versioned release identity bound to asset digests | **MISSING** |
| Exact content hashes / SHA-256 per asset / `weights_sha256` | **MISSING** |
| Model / base-model / tokenizer / corpus identity | **MISSING** |
| Training receipt + runtime/GPU qualification + evaluation evidence | **MISSING** |
| Rights / license NOTICE for artifact redistribution into MedScale Pack | **MISSING** as Pack-bound artifact |
| SBOM / provenance bound to artifact identity | **MISSING** |
| Pack admission of MESC bytes | **BLOCKED** (gate) |

## Non-admission notes

- `TRAINING_CODE_READY = YES` is **not** `RELEASE_READY` and does **not** clear `MESC_RELEASED_ARTIFACT`.
- Repository receipts, tags, and training-code readiness are **not** released model/evaluation artifacts.
- In-repo `data/` trees (litdb/bench) are **not** a released immutable ARTIFACT_IMPORT package with hashes/rights/SBOM for MedScale Pack.
- MedScale must not copy/import MESC Python package, DB, keys, ambient service, or internal runtime.
- MedScale must not mutate `TheHalfMoon/MESC`.
- Admission boundary remains **ARTIFACT_FIRST**.

## Gate decision

**Do not clear** `MESC_RELEASED_ARTIFACT`.  
Keep Spec 012 `BLOCKED_BY_RELEASED_MESC_ARTIFACT` for admit/closeout tasks (T002/T003/T005).

Fail-closed admit contracts + doctor `mesc_artifact` axis remain authoritative while the gate is open (`ExternalGateRequired`).
