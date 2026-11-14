# Dependency admissions for Spec 003

## rusqlite 0.37.0 (features: bundled)

| Field | Value |
|---|---|
| Crate / component | rusqlite (+ libsqlite3-sys bundled) |
| Version / revision pin | 0.37.0 (Cargo.lock) |
| Owning Spec | 003 |
| Purpose | Unencrypted metadata DurableStore for synthetic H0-A vaults |
| Alternatives considered | sqlx; sled; pure FS JSON |
| License / NOTICE | MIT |
| Security / advisory review | cargo-deny clean at admission |
| Transitive dependency notes | libsqlite3-sys bundled — one SQLite copy per process |
| Unsafe / FFI surface | rusqlite/libsqlite3 (P1-ish native); confined behind DurableStore |
| Placement | medscale-storage (Core Host only) |
| Tests required | ingest/backup/restore/gc suites |
| Update strategy | pin minor; re-deny |
| Exit strategy | swap DurableStore backend |
| SBOM / provenance path | Cargo.lock + deny |

SQLCipher is **not** admitted (Spec 005).
