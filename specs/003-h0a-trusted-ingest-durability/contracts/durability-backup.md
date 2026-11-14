# Contract: Durability, Blob Store, Backup/Restore (Synthetic Scope)

**Spec**: 003-h0a-trusted-ingest-durability  
**Status**: Interface sketch for implement  
**Scope**: Synthetic vault durability + backup/restore interface proof; **not** Spec 005 encryption/KeyProvider/key-loss UX

## Traits (logical)

```text
trait DurableStore {
  fn open(vault_root: ClaimScopedPath, lease: CoreHostLease) -> Result<Self>;
  fn put_object(obj: ObjectEnvelope) -> Result<()>;
  fn get_object(id: OpaqueId) -> Result<ObjectEnvelope>;
  fn begin_migration(to: u32) -> Result<()>;
  fn migration_journal() -> Result<MigrationJournal>;
  // ... indexes for visibility, receipts, audit
}

trait BlobStore {
  fn put_blob(bytes: &[u8]) -> Result<BlobRef>;      // temp → fsync → atomic publish
  fn get_blob(digest: DigestSha256) -> Result<Bytes>; // verify digest+size
  fn verify(digest, size) -> Result<()>;
  fn quarantine(digest, reason) -> Result<()>;
  fn mark_live(epoch) -> Result<GcMarkState>;
  fn tombstone_unmarked(epoch) -> Result<u64>;
  fn sweep_tombstones() -> Result<u64>;
}

trait BackupRestore {
  fn backup_vault(dest: ClaimScopedPath) -> Result<BackupManifest>;
  fn restore_vault(src: BackupArtifact, dest_vault_root: ClaimScopedPath) -> Result<RestoreReport>;
}
```

**Rules**:

- Only Core Host holds writable `DurableStore`.
- Exactly **one SQLite implementation** linked in a process that opens canonical metadata (when SQLite candidate used).
- SQLCipher **not** part of this contract in Spec 003.
- No DB/key handles in IPC messages (Spec 002).

## New facade capabilities

```text
Capability::BackupVault
Capability::RestoreVault
Capability::RunBlobGc
Capability::OpenSyntheticVault   // test/dev path under claim scope
Capability::CloseVault
```

## BackupVault

```text
BackupVault {
  destination_hint?: PathBuf,   // must remain claim-scoped / test root
}

→ Ok BackupManifest {
    schema_version,
    vault_id,
    created_at,
    metadata_snapshot_digest,
    blob_entries: [{ digest, size }],
    claim_scope_note: "synthetic-unencrypted-h0a",
  }

→ Err Unauthorized | LeaseRequired | DurableStoreError | PathOutsideClaim
```

**Artifact layout (illustrative)**:

```text
backup/
  manifest.json
  metadata.snapshot
  blobs/<sha256-hex>
```

## RestoreVault

```text
RestoreVault {
  artifact_root: PathBuf,
  dest_vault_root: PathBuf,
}

→ Ok RestoreReport {
    restored_objects: u64,
    restored_blobs: u64,
    closure_ok: true,
  }

→ Err Unauthorized | LeaseRequired
    | ManifestInvalid
    | DigestMismatch
    | RestoreClosureFailed { missing: [digest or object_id] }
    | PathOutsideClaim
    | TargetNotEmpty
```

**Restore closure**: Every visible metadata reference MUST have blob bytes whose digest and size match metadata. Failure → no partial visible custody (fail closed; clean up or leave target non-open).

**Tamper**: Modified blob bytes or manifest digests → `DigestMismatch` / fail closed.

## RunBlobGc

```text
RunBlobGc { }
→ Ok { epoch, tombstoned, swept }
→ Err Unauthorized | LeaseRequired | GcBusy
```

**Invariants**:

1. Must not delete blobs referenced by visible metadata.
2. Must not delete blobs required by in-flight promotion/ingest.
3. Crash mid-GC must not make visible refs point at missing blobs.
4. Race with promotion: promotion wins retention (re-mark / pin).

## Claim-scoped filesystem

```text
ClaimScopedPath {
  vault_root,                 // absolute, validated
  relative,                   // no .. escape
}

FilesystemClaimScope {
  os,
  filesystem_class,
  remote_sync_policy: Refuse | NonClaim,
}
```

**Rules**:

- Reject path escape (`..`, reparse points abused as escape — best-effort per OS).
- Known sync/remote roots: refuse for durability **claim** path or mark NonClaim (do not wave through PASS).
- Evidence archives MUST record `FilesystemClaimScope`.

## Migration interrupt

```text
On open:
  if journal.state == Started:
    resume idempotent migration steps
  else if Finished:
    serve schema to_version
  never serve torn dual-schema as healthy Visible custody
```

## Crash / fault hooks (test-only)

Injectable points (names illustrative):

```text
FaultPoint::BeforeBlobFsync
FaultPoint::AfterBlobFsyncBeforeRename
FaultPoint::BeforeMetadataCommit
FaultPoint::AfterMetadataCommitBeforeVisibility
FaultPoint::MidGcTombstone
FaultPoint::MidGcSweep
FaultPoint::MidMigrationStep
```

Production builds disable injection; tests enable via test feature/cfg.

## Explicit non-goals (Spec 005)

```text
KeyProvider
SQLCipher open
key rotation / key loss recovery UX
encrypted backup wrapping
cloud backup destinations
retention/destruction product policy
PRIVACY_PROOF network observation
```

## Success proof (interface)

Synthetic-scope backup/restore is **proven** when:

1. Contract tests cover Backup/Restore happy path + tamper + closure failure.
2. End-to-end multi-blob vault round-trip passes under recorded FS claim scope.
3. Evidence archive states limitations: unencrypted, synthetic-only, not production recovery UX.
