# Spec 006 evidence summary

## Delivered

- CLI wedge: `doctor`, `vault`, `ingest`, `timeline`/`brief`/`coverage`, `privacy-proof`
- In-process `CliSession` over Core Host authority facade (no direct DB)
- `medscale-desktop` smoke scaffold without Tauri/WebView
- `PRIVACY_PROOF.json` with mandatory limitations
- EXTERNAL_GATES: `TAURI_WEBVIEW_PRIVACY_QUALIFICATION=DEFERRED`

## Commands

```text
cargo test --workspace
cargo run -p medscale-cli -- doctor --json
cargo run -p medscale-desktop -- --smoke
cargo clippy --workspace --all-targets -- -D warnings
```

## Exit gates

| Gate | Status |
|---|---|
| CLI non-bypass (no rusqlite/storage dep) | PASS |
| Doctor axes + secret scan | PASS |
| Longitudinal wedge (in-process) | PASS |
| PRIVACY_PROOF limitations | PASS |
| No Tauri in lock/workspace product path | PASS |
| v0 absent non-blocking | PASS (FINAL_V0_UI_ARTIFACT) |
