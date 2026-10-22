# MedScale catalog entry identifiers

MedScale is the authority for **catalog entry identifiers**, browsing, filtering, admission policy and UI state. It does not become the author of an external model's weights by assigning a local catalog identifier.

Each imported model row has two deliberately different fields:

| Field | Purpose |
| --- | --- |
| `medscale_id` | MedScale's deterministic lookup/display identifier, `medscale-` plus the first 12 bytes of SHA-256 of the exact model repository ID in hexadecimal |
| `repo_id` | The unchanged, actual repository/model ID required for provenance, snapshot verification, download and exact model selection |

These are not interchangeable. `medscale_id` is stable across catalog revisions and different catalog source snapshots for the same exact underlying model ID, but not across a deliberate upstream rename. Different model IDs must not share the same MedScale ID within one import: the parser refuses any collision before publishing results.

No source hash or model weights are edited. Models remain individually subject to rights, integrity and qualification gates. The UI shows the MedScale entry ID beneath a readable model label while retaining the true source ID in the typed catalog contract.

The extra `medscale_id` field is **additive JSON**: no existing `repo_id` values, serialized model archive formats or source identifiers are changed. Downstream strict-schema clients that reject unknown fields must update to accept the field. CLI and desktop regression tests verify deterministic IDs and that the display layer uses them.

Remaining work: source-neutral pack discovery, independently qualified model runtimes, and user-facing identifiers in every runtime status surface. See Issue #273. No release is authorized by this naming work.
