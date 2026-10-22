# MedScale Community Governance

## Mission

MedScale aims to make local-first, evidence-grounded health-data tooling inspectable, reproducible and useful to the open-source community, without misrepresenting experimental code as a clinically validated product.

## How decisions are made

MedScale is currently **maintainer-led, community-contributed**. The repository owner and designated maintainers are accountable for architecture, project scope, technical review, security response, release decisions and contributor conduct. There is no elected governing body or independent foundation at this stage.

Anyone can suggest changes, open evidence-backed Issues, join Discussions or submit Pull Requests. Significant changes should begin with an Issue or Discussion explaining the problem, design options, risks and compatibility effects. Maintainers should explain material accept/reject decisions and favor small, reversible, tested changes.

Current authoritative engineering plans live in [the build queue](docs/planning/BUILD_QUEUE.md) and [project architecture documentation](docs/planning/START_HERE.md). They are maintained project records, **not a requirement to obtain personal founder approval for every external contribution**.

## Contribution lifecycle

1. **Propose:** share a use case or reproducible problem without confidential data.
2. **Discuss:** evaluate fit, safety, reproducibility, licensing and effort.
3. **Implement:** submit focused PRs with tests and explicit limitations.
4. **Review:** verify CI, safeguards, licensing/provenance and maintainability.
5. **Merge or decline:** an authorized maintainer makes the final decision, with a clear rationale where possible.
6. **Release:** separate approval and qualification gates apply; merging a PR does not authorize clinical or production distribution.

Issue and PR participation follows the [Code of Conduct](CODE_OF_CONDUCT.md). Vulnerability disclosure follows the [Security Policy](SECURITY.md).

## Technical and safety guardrails

- Protect the trusted Rust Core and local-first design.
- Keep real patient data, private credentials and live partner access out of public development.
- Preserve the provenance of third-party sources, weights, datasets and historical evidence.
- Do not assert regulatory approval or clinical validity based on community tests or synthetic benchmarks.
- Maintain backward compatibility or explicit migrations for persistent/wire contracts.

## Evolving governance

As the community grows, maintainers may propose documented roles, review expectations, versioned contributor decision records or a more representative governance structure. Such changes should be discussed publicly before adoption. Until then, **community driven means open participation and transparent maintainer stewardship**, not a claim of community ownership or a formal voting process.
