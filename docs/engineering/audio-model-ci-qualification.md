# Audio Model Acquisition: isolated real-checkpoint qualification

Source: PR #321, pinned Whisper Tiny English from `medscale_pack::ASR_SOURCES`.

This `qual-audio-model/**` branch triggers the dedicated
`.github/workflows/audio-model-real-qualification.yml` job on a
disposable GitHub-hosted Windows runner. The job obtains real public Apache-2.0
weights from the pinned HTTPS Hub snapshot, enforces byte budgets and redirects,
verifies model hashes against immutable Hub metadata, produces the existing
signed Audio Pack, checks the temporary acquisition folder is cleaned, and
deletes test-owned files on completion.

No user audio, private data, model checkpoints, or installers are published to
Artifacts. The verification result must be sourced to the exact workflow run
and commit; it is not proof of UI activation, restart, macOS picker behavior, or
full public Beta release readiness.

The job is deliberately opt-in and does not run on main or ordinary PRs.
