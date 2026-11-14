# NO_NETWORK_LOCAL_PATH — Spec 075

## Claim

With network egress disabled, all local-source workflows remain fully
useful: local file source create/preview/import/refresh, external-SQLite
read, snapshot rows, saved views, deterministic transforms, and dataset
releases. Remote dataset adapters report explicit denied/unavailable states;
they never silently fetch and never block local work.

## Design argument (why this holds by construction)

1. Local file and external-SQLite acquisition perform zero network calls:
   `acquire_parsed` for `LocalPath` uses `std::fs::read` only;
   for `Database` it uses the in-process read-only SQLite reader only.
2. The only network touchpoint in 075 is `brokered_dataset_fetch`, reached
   exclusively through `SourceLocator::RemoteDataset`. Local/database
   sources cannot reach it (exhaustive match in `acquire_parsed`).
3. The facade injects `FixtureTransport` (no sockets) for all 075 arms, so
   even remote paths cannot open sockets in this unit; live hosts additionally
   deny at the broker allowlist (empty by default) or refuse with
   `ExternalGateRequired` at `UreqTransport`.
4. Snapshot bytes, receipts, views, transforms, and releases are vault-local
   (sqlite + blob stores); reopen requires no network.

## Test proof

- Core `remote_import_without_allowlist_denies_before_socket` proves the
  deny-before-socket posture with an empty allowlist.
- Core CSV/SQLite suites exercise the full local path with no allowlist
  entries configured (no broker involvement possible).
- CLI vertical-slice log (logs/cli-vertical-slice.log) records a local
  CSV end-to-end run; desktop transcript records the workbench run.
- Full workspace tests run on CI runners without any dataset-host allowlist.

```text
NO_NETWORK_LOCAL_PATH = PASS: exact-head CI run 35580861670 (head 9f84a6e)
green 6/6; PR #129 merged as 89a88cf; post-merge main run 35582548200
green 6/6.
```
