# Data model: Spec 022

## ReleaseQualificationDoctorStatus

| Field | Type | Meaning |
|---|---|---|
| `present` | bool | Axis included in doctor report |
| `prep_ready_base` | bool | Spec 022 prep artifacts/paths present |
| `release_ready` | bool | Always `false` in Spec 022 |
| `locked_builds` | bool | CI/docs use Cargo `--locked` |
| `immutable_ci_action_pins` | bool | Workflows pin Action SHAs + contents:read |
| `cargo_lock_committed` | bool | `Cargo.lock` expected in repo |
| `windows_linux_ci_baseline` | bool | Matrix includes Windows+Linux |
| `macos_qualified` | bool | Always `false` (unqualified) |
| `mobile_release_qualified` | bool | Always `false` (scaffold only) |
| `branch_protection_configured` | bool | Always `false` until EXTERNAL_GATES closed |
| `missing_evidence_classes` | string[] | Honest gap list (no secrets) |

No durable vault objects are introduced.
