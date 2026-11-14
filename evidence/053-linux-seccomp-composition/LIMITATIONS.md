# Spec 053 limitations

- ReadyBaseMeasured Linux seccomp composition only; `platform_qualified=false`.
- x86_64 + aarch64 filter tables; other arches NotReadyOnThisHost.
- Parent process is not seccomp-confined; worker-spawn confinement is later work.
- Old kernels without seccomp yield honest ApplyFailed, never PASS.
- WORKER_OS_SANDBOX_PLATFORM_QUALIFIED remains OPEN (signed App Sandbox
  enforcement + multi-OS qualification external).
- Not RELEASE_READY / PRIVATE_DATA_READY / MULTI_CLIENT_RELEASE_READY.
