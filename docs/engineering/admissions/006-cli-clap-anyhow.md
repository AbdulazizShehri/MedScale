# Dependency admission: clap + anyhow (Spec 006)

| Crate | Version pin (workspace) | Purpose |
|---|---|---|
| clap | 4.6.6 (features: derive; workspace req 4.5.53+) | CLI command tree |
| anyhow | 1.0.104 (workspace req 1.0.100+) | CLI error context |

| Field | Value |
|---|---|
| Owning Spec | 006 |
| Placement | `medscale-cli` only |
| Tauri | **NOT admitted** (v2.11.5 remains SOURCE_ACQUISITION candidate; gate `TAURI_WEBVIEW_PRIVACY_QUALIFICATION`) |
| License | MIT/Apache-2.0 |
| Exit strategy | Replace clap with argv parser; anyhow with thiserror at binary edge |

Research pin 4.6.6 was not required; 4.5.53 is the tested crates.io line frozen at implement.
