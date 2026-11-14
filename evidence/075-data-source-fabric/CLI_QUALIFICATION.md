# CLI_QUALIFICATION — Spec 075

## Groups (all through Core; JSON stable; exits 0/1/2 per existing convention)

```text
medscale datasource create|list|show|update|archive
medscale snapshot preview|import|list|show|rows|refresh [--allow-schema-change]
medscale dataview create|list|show|update
medscale datatransform execute --input ... --op ...
medscale datarelease create|list|show
```

## Parity and error codes

- Human + `--json` output for every command; typed codes: denied,
  not_found, conflict, stale_reference, invalid, corrupt, unsupported_schema,
  unavailable, cancelled, internal.
- CLI has no storage/driver/network edge (`cli_does_not_depend_on_storage_or_rusqlite`
  suite still passes; direction gate still passes).

## Vertical-slice log

`logs/cli-vertical-slice.log` records a local CSV end-to-end run
(source create -> preview -> import -> rows -> view -> transform ->
release -> refresh-unchanged). Local `cargo run` is unavailable on this
workstation (toolchain destruction); the log is produced from CI-attested
test output plus command transcripts at close. No log is fabricated: if a
live CLI run cannot be captured, this file says so explicitly.

```text
RESULT = PASS: exact-head CI run 35580861670 (head 9f84a6e) green 6/6; PR #129
merged as 89a88cf; post-merge main run 35582548200 green 6/6. The CLI
vertical-slice log below is still not a live local `cargo run` transcript
(this workstation's toolchain remains unable to link) - it is derived from
the passing CI test output as originally recorded, not a fabricated live
session.
```
