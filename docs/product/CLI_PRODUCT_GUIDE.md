# MedScale CLI Product Guide

`medscale` is a first-class local client of the same Rust authority path used by Desktop. It has no direct database, key, or network privilege.

## Start here

```text
medscale status
medscale status --json
medscale capabilities
medscale capabilities --json
medscale doctor --json
```

## Patient reads

```text
medscale patient show --vault-id demo --vault-root <vault folder> --subject synthetic-subject --json
medscale patient timeline --vault-id demo --vault-root <vault folder> --subject synthetic-subject --json
medscale patient brief --vault-id demo --vault-root <vault folder> --subject synthetic-subject --json
medscale patient coverage --vault-id demo --vault-root <vault folder> --subject synthetic-subject --json
```

`--vault-root` opens the vault for the read. Leave it out only when a persistent Core Host already has the vault open.

Legacy top-level `timeline`, `brief`, and `coverage` paths remain compatible.

## Authority and audit reads

```text
medscale actions outbox --vault-id demo --vault-root <vault folder> --json
medscale audit disclosures --vault-id demo --vault-root <vault folder> --json
medscale fhir support --vault-id demo --json
medscale fhir export --vault-id demo --vault-root <vault folder> --subject synthetic-subject --purpose "why it leaves" --json
```

`fhir export` prints a loss-aware FHIR R4 export of one subject and records a disclosure for it in the same step; it writes no file. Outbox and disclosure commands are read-only. `UNKNOWN` action state is not a retry authorization. FHIR support is a narrow support matrix, not a full-conformance claim.

## Encrypted vaults

Every command that takes `--vault-root` works on an encrypted vault as well as a synthetic one. When the folder holds an encrypted vault, MedScale opens it with the passphrase in `MEDSCALE_PASSPHRASE`, or, if that is unset, a recovery code in `MEDSCALE_RECOVERY_CODE`. It seals the vault again when the command finishes. Any other folder opens as a synthetic vault, as before.

```text
medscale vault create --vault-id demo --vault-root <vault folder>
medscale vault open --vault-id demo --vault-root <vault folder> --recovery-code-env MEDSCALE_RECOVERY_CODE
```

`vault create` prints the recovery codes once, on standard error, so a redirected output log never holds them. Write them down and keep them offline. They can't be shown again.

`REAL_PHI` is still not authorized: an encrypted vault is for synthetic or permitted development data until the founder says otherwise.

## Vault, Packs, journey, and Host IPC

Use `medscale vault --help`, `medscale packs --help`, `medscale journey --help`, and `medscale host-ipc --help` for mutation and persistent-host workflows. Passphrases are read from a named environment variable, never argv.

The current CLI may own a transient in-process Core Host when no persistent host is used. A transient command does not imply a long-running shared-vault session. Use Host IPC when operating an explicitly running Core Host.

## Exit codes and output

| Exit code | Meaning |
|---|---|
| 0 | Success |
| 1 | Failure. The reason is on standard error as `error: ...` |
| 2 | Usage error: an unknown command, or a missing or invalid argument |
| 3 | Failure of a command run with `--json`. Standard error holds one JSON object with `error`, `code` and `message`, and standard output is empty |

With `--json`, a successful command prints exactly one JSON value on standard output. Secrets (passphrases, recovery codes) never go to standard output.

## Install

Release archives hold one `medscale` (Windows: `medscale.exe`) binary, `LICENSE`, `NOTICE.md` and `SHA256SUMS`. Nothing else is needed: no Rust toolchain, no runtime and no network. Check the archive with `sha256sum -c SHA256SUMS` (Windows: `Get-FileHash`), unpack it anywhere, and put the folder on `PATH`. Builds are unsigned engineering builds: Windows SmartScreen and macOS Gatekeeper warn on first run. On macOS, remove the quarantine attribute with `xattr -d com.apple.quarantine medscale` only if you checked the hash.

## Privacy / release boundary

Current product scope is synthetic/permitted development data only. `REAL_PHI` is not authorized. JSON/text output must not contain secrets. `RELEASE_READY`, `PRIVATE_DATA_READY`, `MULTI_CLIENT_RELEASE_READY`, full FHIR conformance, and WCAG conformance remain false unless separately evidenced.
