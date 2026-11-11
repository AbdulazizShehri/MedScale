# Real patient data: decision package

**Status: `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`. This document does not change it.**

This package is for the founder. It sets out what MedScale needs before it may hold real patient data, what engineering has finished, what is still open, and the decision options. It authorizes nothing, and no engineer or AI agent can sign it for the founder.

Engineering state: `main` on 2026-11-11, after PRs #485–#494. The detailed blocker list is [REAL_DATA_READINESS.md](REAL_DATA_READINESS.md), and the engineering inventory is [PHI_READINESS_CHECKLIST.md](../legal/PHI_READINESS_CHECKLIST.md).

## 1. What would be authorized

A decision has to name all of the following. A decision that leaves any of them out is incomplete.

| Item | What has to be named |
|---|---|
| Who | The people or organization that would use MedScale with real records |
| Where | The devices, their operating systems, and whether disk encryption (BitLocker, FileVault, LUKS) is required |
| Which data | Record types (for example FHIR exports from a named portal), volume, and whether the data is identifiable |
| Which surfaces | Desktop encrypted vault only, or also the CLI. The sample workspace is never authorized |
| Jurisdictions | Where the people and the data are. This decides which laws apply (section 4) |
| Duration | A start date, an end date, and how the data is deleted at the end |
| Purpose | Research or personal record-keeping only. Clinical use needs a separate decision (section 6) |

## 2. Engineering safeguards in place

Each row was verified by an automated test in CI. That makes it engineering evidence, not an independent assessment.

| Safeguard | State | Evidence |
|---|---|---|
| Encrypted vault (SQLCipher metadata, sealed blobs, Argon2id-wrapped key, recovery codes) | In place | Storage and Core tests, Spec 005/035/109 |
| FHIR import goes into the encrypted vault, and the patient list stays sealed | In place (#492) | Desktop IPC test scans the whole data folder for plaintext markers |
| Auto-lock after 15 minutes without activity | In place | Timed-lock test (B3) |
| Verified backup and restore that never deletes the replaced vault | In place (#485) | Storage, Core, desktop IPC and CLI binary tests |
| CLI opens encrypted vaults and seals them after every command | In place (#489) | CLI unit and binary tests |
| Network default-deny; downloads only through allowlisted, verified paths | In place | Broker and model-acquisition tests |
| Crash recovery that keeps what it sets aside | In place | Spec 109 tests and Windows kill journeys |
| MedAgent refuses to generate from evidence that is no longer current | In place (#482, #488) | Core and desktop tests |

## 3. What is still open

None of these is closed. Each one, on its own, is a reason to keep `NOT_AUTHORIZED`.

| # | Gap | Why it matters | Owner |
|---|---|---|---|
| B1 (rest) | The sample workspace is unencrypted and still accepts any FHIR file | Someone could put a real record there by mistake | Engineering; restricting it needs a founder decision |
| B2 | Real-world FHIR mostly fails: decimals rejected, 1 MiB limit, Bundles not split | Real portal exports can't be used | Engineering |
| B3 (rest) | No passphrase change and no recovery-code revocation | A leaked code works forever | Engineering |
| B4 | No rollback protection; the audit trail isn't tamper-evident | Records and accountability can be altered by someone with file access | Engineering |
| B6 | No privacy notice, terms or intended-use statement | Legal exposure | Founder and counsel |
| Keys | Argon2id at 19 MiB, below the 64 MiB target; key copies not wiped from memory | Offline guessing and memory exposure | Engineering |
| Residue | OS swap, crash dumps and WebView caches are not proven clean | Copies outside the vault | Engineering plus OS policy |
| Plaintext helpers | Pack and model folder records sit beside the vault | They reveal what was installed, not patient data | Engineering |
| Review | No independent human security review | The current audit was done by an AI reviewer | Founder to commission |
| Release | Builds are unsigned; `RELEASE_READY=false` | Users can't verify who built the app | Founder (paid certificates) |

## 4. External reviews required before any authorization

1. **Independent security review** by a qualified human assessor. It must cover the vault, key handling, the IPC boundary, the update and download paths, and residue on disk.
2. **Legal review.** It decides which of these apply and what each requires:
   - Saudi PDPL and SFDA
   - GDPR Article 9
   - HIPAA, if a covered entity or business associate is involved
   - the US FTC Health Breach Notification Rule
   - Washington's My Health My Data Act
   - medical-device rules (EU MDR Rule 11, FDA clinical decision support guidance, SFDA)
   - SNOMED CT, LOINC and ICD licensing
3. **Data protection impact assessment** for the named scope in section 1.

These cost money or need outside parties, so they need the founder's approval. Engineering has not started any of them.

## 5. Decision options

| Option | Meaning | Preconditions |
|---|---|---|
| **A. Keep NOT_AUTHORIZED** (current) | Synthetic and permitted test data only | None |
| **B. Authorize a limited internal pilot** | Named people, named devices, encrypted vault only, defined dates, deletion at the end | Every row in section 3 except Release closed or formally accepted; both reviews in section 4 done; a written scope (section 1) |
| **C. Authorize general use** | Anyone may use real records | Everything in B, plus signed releases, public notices and an incident process |

Engineering's recommendation is **A** until the section 3 gaps marked *Engineering* are closed and the section 4 reviews are commissioned. Option B is the earliest realistic step after that.

## 6. Clinical use is a separate decision

Authorizing real data does not make MedScale fit for clinical decisions. MedScale is not clinically validated. Its outputs are proposals and research notes. Clinical use would need clinical validation, a regulatory classification and quality-management evidence, and none of these exist. `CLINICAL_USE` stays not authorized whatever is decided here.

## 7. Founder decision record

Only the founder fills this in, in writing. Engineering then changes the gate in `EXTERNAL_GATES.md` and the doctor report, and adds scoped tests, in a reviewed PR that cites this record.

```text
Decision:            A / B / C
Scope (section 1):
Reviews (section 4): security review reference:          legal review reference:          DPIA reference:
Accepted open items (section 3), with reasons:
Valid from:          Valid until:
Signed:              Date:
```
