# Spec 027 limitations

- `RELEASE_READY = FALSE`
- `PRIVATE_DATA_READY = FALSE`
- `MULTI_CLIENT_RELEASE_READY = FALSE`
- Performance budget attainment **not claimed** (`budgets_claimed_met=false`)
- READY_BASE harness scale may be smaller than delivery-plan 10k events / 10k lexical records
- SBOM scaffold ≠ full release SBOM (native binaries / model assets / signed package contents missing)
- Package checksums are sha256 source/lock digests only — **not signed**, not a reproducible binary install package
- macOS / mobile release qualification still missing
- Branch protection / required checks remain EXTERNAL_GATES
- Public SPDX license choice remains EXTERNAL_GATES
- Spec 012 MESC remains blocked externally; untouched
- Synthetic-only; REAL_PHI unauthorized
