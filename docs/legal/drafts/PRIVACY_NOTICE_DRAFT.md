# MedScale privacy notice (draft)

**DRAFT FOR LEGAL REVIEW. NOT IN EFFECT. NOT LEGAL ADVICE.**

This draft covers two things: the MedScale app (desktop and command line) and the medscale.si website.

## The app

**The project receives no data from the app.** MedScale has no accounts, no sign-in, no telemetry, no analytics and no crash reporting. There is no MedScale server.

| What | Where it stays | Source |
|---|---|---|
| Records, documents, notes and audit history | On the user's computer, in a vault folder the user controls | `docs/security/TRUST_AND_PRIVACY_MODEL.md` |
| Encrypted vault contents | Sealed with AES-GCM under a key wrapped by the user's passphrase (Argon2id); readable only while unlocked | `crates/medscale-storage/src/encrypted_vault.rs`, `crates/medscale-keys` |
| Sample workspace | Not encrypted, and meant only for synthetic data; it refuses files that name national patient identifier systems | `apps/desktop-tauri/src-tauri/src/host.rs` |
| Backups | Wherever the user chooses to save them; they stay encrypted | `docs/beta/BACKUP_AND_RESTORE.md` |

**Network access.** The app's Core denies network access by default. The one network action today is an optional, user-started download of speech-recognition model files from Hugging Face's servers (`apps/desktop-tauri/src-tauri/src/audio_model_manager.rs`); that download sends Hugging Face a normal web request (IP address, the file requested) and no record content. **[DECISION]** List any later network features here before they ship.

**Reporting a problem.** Help links open GitHub in the user's browser. Anything posted there is public and subject to GitHub's terms; the app attaches nothing automatically, and users are told never to include patient information.

**Who controls the data.** Because the data never leaves the user's computer, the user (or the organization they work for) decides what is stored and is responsible for it. **[COUNSEL]** Confirm the controller/processor and covered-entity analysis (question 2 in `README.md`).

**Limits.** The app can't control copies the operating system makes, such as swap files, snapshots, indexing or backups of the computer (`docs/legal/PHI_READINESS_CHECKLIST.md`). While a vault is unlocked, its working database is on disk unencrypted inside the vault folder.

## The website (medscale.si)

- A static site hosted on GitHub Pages. It sets no cookies and has no analytics, forms or accounts of its own.
- GitHub, as host, receives standard request data such as IP address and browser details, under GitHub's own privacy statement. **[COUNSEL]** Wording for this (question 4).
- Download links point to GitHub Releases, also under GitHub's terms.

## Contact

**[DECISION]** A contact for privacy questions (today: the project's GitHub security policy for vulnerabilities, and Discussions for questions).
