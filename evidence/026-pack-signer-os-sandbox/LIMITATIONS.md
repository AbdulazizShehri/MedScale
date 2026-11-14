# Spec 026 limitations

- Synthetic trust root only; not production code-signing PKI or vendor model roots.
- Fixture/synthetic Packs only; no gated HF/model terms accepted.
- Linux Landlock is READY_BASE measured, **not** multi-OS PLATFORM_QUALIFIED.
- Windows AppContainer / macOS Seatbelt remain scaffolds (`NotPlatformQualified`).
- Landlock apply tests run only on `cfg(target_os = "linux")`; Windows CI compiles stubs.
- No seccomp / network deny / rlimit composition in Spec 026 (see Spec **052** for Landlock FS+TCP+rlimit composition READY_BASE; seccomp still open).
- No native ONNX/llama/CPU engine admission.
- Does not claim RELEASE_READY, PRIVATE_DATA_READY, or MULTI_CLIENT_RELEASE_READY.
- EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` remains OPEN.
