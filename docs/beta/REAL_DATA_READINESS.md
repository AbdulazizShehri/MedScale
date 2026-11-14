# Real health data: what stands in the way

**Status:** `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`. MedScale must not be used with real patient records yet. This page lists what has to change before the founder can authorize it, and tracks each item to done.

The list comes from a code audit of `main@8ce2119` on 2026-11-07. That audit was done by an AI reviewer, not an independent human security assessor.

The founder's decision options, the scope a decision must name, and the record to sign are in [REAL_PHI_AUTHORIZATION_PACKAGE.md](REAL_PHI_AUTHORIZATION_PACKAGE.md).

## Blockers

| # | Problem | Why it matters | Fix | Proof needed | Status |
|---|---|---|---|---|---|
| B1 | FHIR import only writes to the sample workspace, which isn't encrypted. Nothing stops someone importing a real record there | A real record would sit unencrypted on disk | Import into encrypted vaults. Make the sample workspace refuse files that aren't its bundled fixtures. Keep the patient list inside the vault | An encrypted-vault import test, plus a disk scan finding no markers outside sealed files | **Partly done:** FHIR import works in the encrypted vault, and its patient list is read from Core's sealed records instead of a file. A desktop test scans the whole data folder for plaintext markers. The sample workspace now refuses an import until the person confirms it holds no real patient information, and refuses any file naming a national patient identifier system (US SSN, US MBI, NHS number) even when confirmed. Still open: a determined person can still confirm and import a real record that names none of those systems |
| B2 | Real FHIR files mostly fail: decimal numbers are rejected, the size limit is 1 MiB, Bundles aren't split into entries, and citations point at the first matching key | Portal exports such as US Core and IPS can't be used | Keep decimals exactly. Split Bundles. Raise the limit and stream. Give citations precise paths | Import tests on public US Core and IPS example bundles | **Partly done:** decimals are admitted and kept exactly as written (`1.10` stays `1.10`, never rounded through f64), and the import limit is 8 MiB. A Bundle is stored once and each entry becomes its own record citing it (up to 2,000 entries; nested Bundles are refused). A citation covers the exact member it came from, key and value, inside the cited resource (found by type and id, also inside a Bundle), and gives no span rather than a guess. A Bundle over 8 MiB (up to 64 MiB) and any NDJSON export are imported entry by entry, each entry stored as its own source with its exact bytes; one resource is still at most 8 MiB. Tested on five public HL7 examples (the IPS example Bundle, a US Core Bundle and three US Core resources; `fixtures/public/hl7`): each passes the gate, keeps every decimal exactly, splits into its entries by byte position, and cites the exact member of the right resource. **Done for import; real portal exports from specific vendors are not yet tested** |
| B3 | An unlocked vault stays open until someone locks it. The passphrase can't be changed, and recovery codes can't be revoked | Shared or unattended computers | **Auto-lock after 15 minutes without activity (this change).** Passphrase change and code revocation | Timed-lock test, plus tests that a revoked code and an old passphrase both fail | **Done:** auto-lock after 15 minutes; passphrase change and replacement of every recovery code in Settings and the CLI (`medscale vault change-passphrase`, `medscale vault replace-recovery-codes`), both needing the current passphrase. Tests in the key provider, storage, Core, desktop IPC and the CLI show the old passphrase and every old code fail and the new ones open. Limit: backups made before a change keep the passphrase and codes they had |
| B4 | No protection against rolling a vault back to an older copy. The audit trail isn't tamper-evident, and its rows can be lost in a crash | Integrity of records and of who did what | A generation counter in the sealed metadata. A hash chain for audit rows | Rollback and tamper tests | **Partly done:** every lock raises a generation number sealed with the data, and each device records the last one it locked in a file beside (not inside) the vault folder. Opening an older copy shows a warning in the app and the CLI; the app's own backup restore and crash-recovery restore reset the record so they do not warn. Limit: someone who can also delete that record defeats the check, and a vault opened for the first time on a device has nothing to compare with. Audit: every audit record written, changed or removed adds an entry to an append-only hash chain in the same transaction, and `medscale vault verify-audit` recomputes it and reports any record or entry changed outside it (storage tests edit the database directly to prove this). Limits: the chain lives in the same sealed database, so it shows changes made by a bug or by someone with the key, not a wholesale replacement of the database; a synthetic-workspace backup restarts the chain. **Done, with those limits** |
| B5 | No backup or restore in the app, although the storage layer has both | People can lose their data | Backup and restore commands, with checks on the copy | A backup, restore and compare test | **Done in the app and CLI:** verified backup of a locked vault, and restore that keeps the replaced vault ([BACKUP_AND_RESTORE.md](BACKUP_AND_RESTORE.md)). Tested in storage, Core, desktop IPC and the CLI |
| B6 | No privacy notice, terms or intended-use statement for real data, and clinical-sounding features | Legal and safety exposure | A legal review, then the notices | Founder and legal sign-off | **Drafts ready for legal review** ([docs/legal/drafts](../legal/drafts/README.md): intended use, privacy notice, terms, and seven questions for counsel). Not in effect; needs counsel and the founder |

## Also needed

- **Key strength:** raise Argon2id from 19 MiB to at least 64 MiB for new vaults, and rewrap on unlock.
- **Secrets in memory:** wipe the remaining copies of the passphrase-derived key and the recovery key from memory after use.
- **Blob names:** name sealed files with a keyed hash, so someone holding a copy of a file can't confirm it's in a vault.
- **Error text:** replace raw storage errors with fixed messages, so file paths can't appear on screen.
- **Crash dumps:** keep Windows crash reports from capturing process memory.
- **Browse:** check the URL path for personal data as well as the query, and turn Browse off in vaults that hold real data.
- **Unencrypted helper files:** move the seed list and the model and pack records next to the vault inside it.
- **Disk encryption:** decide whether to check for BitLocker or FileVault, or require it.

## Decisions only the founder can make

1. **Commission an independent human security review.** This is required before authorizing real data.
2. **Commission a legal review.** It needs to cover:
   - the US FTC Health Breach Notification Rule
   - GDPR Article 9
   - Saudi PDPL and SFDA
   - Washington's My Health My Data Act
   - medical-device classification under EU MDR Rule 11, FDA clinical decision support guidance and SFDA
   - code-system licensing for SNOMED, LOINC and ICD
3. **Shared-computer policy:** the auto-lock default, and whether local rollback attacks are in scope.

## Done means

Every blocker is closed with its proof, both reviews are complete, and the founder authorizes real data in writing. Only then can the product and website describe real-data workflows.
