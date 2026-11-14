# RELEASE_READY checklist vs Spec 027 READY_BASE

Trusted V1 `RELEASE_READY` remains **FALSE**. Spec 027 adds scaffolds only.

| Requirement | Spec 027 | Notes |
|---|---|---|
| Perf measurement path | PARTIAL | Harness + methodology present; budgets not attained/claimed |
| Reproducible package contents | PARTIAL | Source/lock sha256 manifest; not a release package |
| SBOM incl. native/model assets | FALSE | cargo-metadata scaffold only |
| Checksums / provenance / signing | FALSE | Unsigned checksums only |
| Qualified OS matrix / branch protection / license / migration bar | FALSE | Spec 029 adds macOS **CI** (`macos_ci_present`); product `macos_qualified` still false; other gaps unchanged |

**Verdict: `RELEASE_READY = FALSE`**
