# Backing up and restoring the encrypted vault

MedScale keeps one encrypted vault per device. You can copy it to a folder you choose and put it back later, on this device or another one. A backup holds only the vault's encrypted files. It opens with the passphrase and recovery codes the vault had when it was backed up, and with nothing else.

`REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED` still applies. Backups don't make MedScale ready for real patient records.

## In the desktop app

**Back up:** go to **Settings → Workspace → Back up vault** and choose **Lock and back up…**. Then pick a folder.

- MedScale locks the vault first, so everything you did is in the backup.
- It writes a new folder named `MedScale-backup-<time>` inside the folder you picked. It never overwrites an earlier backup.
- It reads every copied file back and compares it with the original. Only if every file matches does it report the backup as done.
- When it finishes you are on the lock screen. Unlock to keep working.

**Restore:** lock the vault. On the lock screen choose **Restore from a backup…**, pick a `MedScale-backup-*` folder, and confirm.

- The current vault is not deleted. It is moved beside the restored one as `encrypted-workspace.before-restore-<time>`, inside MedScale's data folder.
- The restored files are compared with the backup.
- Unlock with the passphrase or a recovery code that belonged to the vault **when it was backed up**.

## On the command line

```bash
medscale vault backup --vault-root <vault folder> --to <existing folder> [--label <name>] [--json]
```

```bash
medscale vault restore --from <MedScale-backup folder> --vault-root <vault folder> [--label <name>] [--json]
```

Both commands go through MedScale Core, like the desktop app. With `--json` they print one JSON object. If anything is refused they exit with a non-zero status.

## What is refused, and why

| Situation | Result |
|---|---|
| The vault is open in MedScale or another process | Refused. The newest changes aren't sealed yet. The desktop app locks the vault for you first |
| The vault has an unsealed session left over from a crash | Refused. Unlock it once so MedScale can recover or set aside the session, then lock it and back up |
| The backup folder would be inside the vault folder, or the vault would be restored inside the backup | Refused |
| A cloud-synced folder (OneDrive, Dropbox and similar) | Refused, the same as for the vault itself |
| The chosen folder isn't a MedScale backup | Refused before anything is moved |
| A copied file doesn't match its original | Reported as an error. Don't rely on that backup |

## What a backup does not cover

- **Audit trail:** backup and restore run while the vault is locked, so they don't write rows to the vault's own audit log. This page and the result shown on screen are the record.
- **Rolling back:** restoring an older backup puts the vault back to that time, on purpose. MedScale doesn't yet stop someone with access to the files from doing the same thing without asking. See blocker B4 in [REAL_DATA_READINESS.md](REAL_DATA_READINESS.md).
- **Model and pack folders:** Audio Pack and model folder settings depend on the machine, so they aren't in the backup. Install them again on a new device.
- **Patient list:** the desktop's patient list (`desktop-roster.json`) goes into the backup and comes back on restore. It is not encrypted, the same as the copy kept beside the vault today.
- **Where you keep it:** a backup is only as safe as the disk it's on. Keep it on a drive only you can reach.
