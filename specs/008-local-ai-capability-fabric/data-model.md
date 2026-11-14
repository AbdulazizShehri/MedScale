# Data Model: Spec 008 Packs

## PackPromotionState

`candidate` | `current` | `last_green` | `canary`

## PackManifestV0

| Field | Required |
|---|---|
| pack_id | yes |
| version | yes |
| content_digest | yes (SHA-256 of canonical manifest body sans digest field, or directory digest policy) |
| artifact_digests | yes map path→sha256 |
| rights_uri | yes |
| sbom_ref | yes |
| runtime_requirements | yes |
| benchmark_links | optional |
| promotion_state | yes |
| artifacts[].kind | yes — `fixture_bytes` \| `tokenizer_meta` \| `onnx_model` \| `pickle` \| `code_bin` \| `onnx_custom_op` |
| forbidden kinds on admit | pickle, code_bin, onnx_custom_op (unsigned) |

## PacksRuntimeDoctorStatus

present, offline_only, admitted_count, current_pack_id, confinement_claim, online_download_authorized=false

## WorkerSupervisionPolicy

Existing ambient deny + `confinement_profile` string (e.g. `policy_ambient_deny_v0`).
