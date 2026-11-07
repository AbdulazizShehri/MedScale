# Real health data: what stands in the way

**Status:** `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`. MedScale must not be used with real patient records yet. This page lists what has to change before the founder can authorize it, and tracks each item to done.

The list comes from a code audit of `main@8ce2119` on 2026-11-07. That audit was done by an AI reviewer, not an independent human security assessor.

## Blockers

| # | Problem | Why it matters | Fix | Proof needed | Status |
|---|---|---|---|---|---|
| B1 | FHIR import only writes to the sample workspace, which isn't encrypted. Nothing stops someone importing a real record there | A real record would sit unencrypted on disk | Import into encrypted vaults. Make the sample workspace refuse files that aren't its bundled fixtures. Keep the patient list inside the vault | An encrypted-vault import test, plus a disk scan finding no markers outside sealed files | Open |
| B2 | Real FHIR files mostly fail: decimal numbers are rejected, the size limit is 1 MiB, Bundles aren't split into entries, and citations point at the first matching key | Portal exports such as US Core and IPS can't be used | Keep decimals exactly. Split Bundles. Raise the limit and stream. Give citations precise paths | Import tests on public US Core and IPS example bundles | Open |
| B3 | An unlocked vault stays open until someone locks it. The passphrase can't be changed, and recovery codes can't be revoked | Shared or unattended computers | **Auto-lock after 15 minutes without activity (this change).** Passphrase change and code revocation | Timed-lock test, plus tests that a revoked code and an old passphrase both fail | **Auto-lock done**; the rest open |
| B4 | No protection against rolling a vault back to an older copy. The audit trail isn't tamper-evident, and its rows can be lost in a crash | Integrity of records and of who did what | A generation counter in the sealed metadata. A hash chain for audit rows | Rollback and tamper tests | Open |
| B5 | No backup or restore in the app, although the storage layer has both | People can lose their data | Backup and restore commands, with checks on the copy | A backup, restore and compare test | Open |
| B6 | No privacy notice, terms or intended-use statement for real data, and clinical-sounding features | Legal and safety exposure | A legal review, then the notices | Founder and legal sign-off | Needs a founder decision |

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
