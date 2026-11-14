# Dependency direction

Allowed edges for Spec 001:

```text
medscale-cli -> medscale-core -> medscale-contracts
```

Forbidden:

- `medscale-contracts` depending on `medscale-core` or `medscale-cli`
- `medscale-core` depending on `medscale-cli`
- any future `apps/*` or `workers/*` crate depending directly on storage/keys
  without going through the core authority facade (enforced when those crates appear)

Check locally:

```powershell
pwsh ./scripts/check-dependency-direction.ps1
```
