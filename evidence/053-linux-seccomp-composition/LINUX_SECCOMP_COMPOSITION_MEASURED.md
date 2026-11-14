# Linux seccomp composition measured (READY_BASE)

Measured on Linux CI (ubuntu, x86_64): `measure_seccomp_composition`
spawns `medscale-os-sandbox-probe seccomp-child`; child output contains
`MEDSCALE_SECCOMP_FILTER_ACTIVE` and the child dies by SIGSYS (31) on the
denied `socket()` call. Local non-Linux hosts honestly report
NotReadyOnThisHost (apply path) while the evidence-in-tree doctor axis
stays `linux_seccomp_composition_measured=true`.

This is per-axis READY_BASE only. It is not multi-OS PLATFORM_QUALIFIED.
