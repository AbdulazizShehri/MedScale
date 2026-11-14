# Worker ambient deny (Spec 008S-lite)

| Ambient privilege | Allowed |
|---|---|
| Canonical DB | no |
| Master keys | no |
| Unrestricted filesystem | no |
| Network | no |
| Secrets | no |
| Authority | no |

Profile: `policy_ambient_deny_v0`

OS Landlock / AppContainer / Seatbelt = `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` (OPEN). Policy confinement is the Spec 008 exit claim.
