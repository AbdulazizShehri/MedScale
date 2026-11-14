# Mobile pack format constraints (feedback to Spec 015)

READY_BASE documents requirements for later online/sideload packs:

| Constraint | Requirement |
|---|---|
| Chunking | resumable chunked download |
| Per-arch | separate artifacts for ios-arm64 / android-arm64-v8a |
| 16KB | Android native libs must link for 16KB pages |
| Offline verify | TUF/offline root before install |
| No silent sync | pack install must not sync keys via iCloud Keychain |

No mobile pack install runtime in Spec 009 READY_BASE.
