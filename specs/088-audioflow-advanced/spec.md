# Spec 088 — AudioFlow Advanced (huddle foundation)

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promotion date:** 2026-09-26
**Canonical base:** `a40bd1efc05e355c6249f9f3452c953dde62bccc` (Spec 087 closure merge)
**Target branch:** `spec/088-audioflow-advanced`

## Authority

The founder's standing continuation directive requires promoting the next
dependency-ready Research OS unit after each closure. Dependency proof:
`RESEARCH_OS_V2_SPEC_IMPLEMENTATION_CONTRACTS.md` numbers AudioFlow
Advanced **088**, hard dependency **081 + 084** (the roadmap adds 076;
Compute integration only for remote/heavy audio workers). All are
`CLOSED_CANONICAL`. This spec admits no dependency.

Evidence truth inherited from Spec 081: there is no native capture
backend and no real ASR, diarization or speech-synthesis engine; the only
recognition route in CI is the labeled fixture engine. Medical ASR quality
(medications, doses, routes, frequencies, numbers, units, labs, dates,
negation, allergies, speaker attribution, Arabic, English, Arabic-English
code switching) is `UNMEASURED`.

## Scope decision

The authority's goal lists huddles, TTS/listen-back, duplex agent voice,
media annotations, batch workflows and voice design/cloning. Speech
synthesis, duplex voice and cloning need model admission (Audio Pack,
Q18/Q21) and are **`NOT_ADMITTED`** here. This slice delivers the
closure-gate core that needs no model:

```text
audio source != transcript != proposal != reviewed item
  != export (consented, labeled) != external effect (none)
```

- Project huddles with human and explicitly scoped agent participants.
- Separate, revocable consents per participant for join, record,
  transcribe and export; each act on shared media requires the consent of
  every human participant at that moment; withdrawing `join` withdraws
  everything; agents may only join and never consent for humans.
- Media is a Spec 081 source attached to a huddle; agent-attributed audio
  is always labeled synthetic and exports carry the label.
- Transcription through the Spec 081 route; task and evidence proposals
  cite transcript segments and stay proposals until a human reviews them.
- Retention per huddle (days, host-supplied day); explicit deletion and
  retention sweeps remove the audio row and bytes, its transcripts and
  their receipts in one transaction (SQLite `secure_delete` on), leaving a
  receipt with the removed digests.
- Storage v17, backup/restore, consistency; CLI `medscale huddle ...`.

## Explicitly not authorized

- Speech synthesis, voice design or cloning, duplex agent voice, live
  capture backends, remote audio workers, any cloud route.
- Persistent speaker identity (enrollment) — deferred; speaker labels stay
  anonymous as in Spec 081.
- Real PHI; claims about medical ASR quality.

Recorded residuals: deletion cannot reach copies outside the vault
(exports the user saved, OS backups) or earlier vault backups; long-session
behavior is not load-tested; no Desktop surface.

## Completion rule

`CLOSED_CANONICAL` only after merge on a green exact head and recorded
post-main verification.
