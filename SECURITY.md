# Security Policy

## Scope and support status

MedScale is an **unsigned, synthetic-only engineering preview**. It is not authorized for real patient data, clinical decisions or production deployment. Open-source availability is not a warranty, a vulnerability-free claim or a commitment to support any particular release.

There are no currently supported production releases or clinical deployment versions. We review reports against the current `main` branch and qualifying engineering preview builds.

## Report vulnerabilities privately

**Never disclose an exploitable vulnerability, credentials or patient information in a public Issue, Pull Request, Discussion or screenshot.**

Use GitHub's **[Report a vulnerability](https://github.com/AbdulazizShehri/MedScale/security/advisories/new)** private reporting path. If GitHub does not offer that control to you, avoid publishing exploit details; ask a maintainer for an appropriate private reporting channel without disclosing the vulnerability in the public request.

Please include, where safe: affected commit/version, operating system, impact, a minimal reproduction with **synthetic data only**, relevant redacted logs and any mitigations. Do not test against real patients or systems you do not own or have permission to assess.

Maintainers will triage privately, seek a reproducible fix, run regression/security tests and coordinate disclosure where appropriate. No fixed response or remediation SLA is promised.

## Sensitive data and supply chain

- Do not commit secret values, real PHI, private keys or proprietary partner records.
- Preserve original `LICENSE`/`NOTICE` records, third-party attributions, SBOM evidence and source pinning.
- Prefer smallest-possible security patches; do not hide residual risk or invent clinical/regulatory certification.
- Do not treat a passed CI run or unsigned installer as production release authorization.

See [the external release and privacy gates](docs/planning/EXTERNAL_GATES.md) and [contribution policy](CONTRIBUTING.md).
