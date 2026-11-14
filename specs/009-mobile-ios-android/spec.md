# Feature Specification: Mobile iOS + Android (READY_BASE)

**Branch**: `spec/009-mobile-ios-android`  
**Status**: QUALIFIED for READY_BASE (005+006 CLOSED; AI packs consume 008 later)  
**Input**: Shared Rust authority semantics for future native shells; Keychain/Keystore policy (no silent Apple sync); 16KB Android compatibility claim stubs; FFI admission templates; doctor mobile axes. No real apps, REAL_PHI, Tauri, or model engines.

## User Stories
### US1 Shared authority surface (P1)
Mobile hosts call the same Core Host facade; no DB/key handle export across FFI.

### US2 Keystore policy (P1)
iOS Keychain: `synchronizable=false` required; Android Keystore HW-backed preferred when available; MemoryKeyStore remains CI default.

### US3 16KB / ABI admission stubs (P1)
FfiAdmissionRecord templates for arm64-v8a / arm64-apple-ios with 16KB page-size note; incomplete until PLATFORM_QUALIFIED evidence.

### US4 Doctor mobile axes (P1)
Doctor reports mobile_surface, key_sync_policy, page_size_16kb_claim, sideload_posture.

## Anti-scope
Xcode/Gradle apps; App Store signing; ONNX/MLX; REAL_PHI; Tauri; OpenMed mobile wholesale copy.
