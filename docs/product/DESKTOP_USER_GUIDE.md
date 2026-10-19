# MedScale desktop: user guide (engineering preview)

MedScale is an engineering preview. It is for **synthetic data only**: it is not clinically validated, not a medical device, and real patient data is not authorized. Every model output is a **proposal**; nothing becomes a record until a person accepts it through review.

The desktop runs on **Windows and macOS**. A Linux desktop waits on an upstream Tauri change (`TAURI_LINUX_DESKTOP_GTK3`); the CLI runs on Linux.

## 1. Open a workspace

- **Synthetic workspace:**
  - Ingests the bundled synthetic FHIR fixtures through Core: two subjects and their observations, conditions and medications.
  - Runs, agents, transcripts and imports persist in its vault and resume after a restart.
- **Encrypted workspace:** protected by a passphrase. It holds projects, data, analytics and collaboration. Core admits no clinical ingest into it yet.

Lock the workspace from Settings or with `Ctrl/Cmd + Shift + L`. Locking closes the vault and releases keys from memory.

## 2. Documents: import synthetic FHIR

- **Documents → Import synthetic FHIR:**
  1. Give the absolute path of a FHIR R4 JSON file (at most 1 MiB) and a subject id.
  2. Core ingests and promotes the file exactly as it does the bundled fixtures.
- The source then appears in Documents and Patients, and it can be bound to an agent's context.
- The synthetic workspace only.

## 3. Models

- **Packs:** the Packs admitted in this session.
- **Catalog:** the pinned OpenMed v3.0.0 catalog (2,266 rows, Apache-2.0, bundled unchanged).
  - Search, filter and page through it. Each row shows MedScale's pre-download expectation (for example *executed tested* or *known unsupported*).
  - A listing is not a download and not an executable model.
- **Try a model:**
  - Run an admitted Pack on synthetic text.
  - For token classification, choose the entity decoding. *Word: first sub-token* is preselected because it scored at or above the OpenMed-compatible decoder on every synthetic head-to-head row.
  - For GLiNER, enter labels.
  - The result is evaluation evidence only.

## 4. MedAgent

1. **New agent:** give it a name, choose a model (an admitted Pack), and choose its tools (read and/or search bound sources).
2. **Admit a local Pack folder** to make another model available. Core verifies its manifest, digests and signature; nothing is downloaded.
3. **Create a run:** choose the agent and the ingested sources its context binds (1–16), then write the prompt.
4. **Start → Execute on local runtime → Complete.** Complete records a receipt. Cancel is available until the run ends.
5. **Evidence** lists each bound source with its reference resolution. **Turns** show the prompt and the model output.

Revoking an agent keeps its history. Runs, turns, agents and evidence resume after a restart; remembered Pack folders are admitted again automatically.

## 5. Model Fleet

- **New lane:** a role, an agent (its Pack is the lane's model) and a source.
- **Create fleet run:**
  1. Dispatch to two or more active lanes.
  2. Execute each lane. Each runs on its own agent's Pack.
  3. Compare.
- A comparison records agreement and disagreement. It never picks a winner.

## 6. Audio (AudioFlow)

1. **Admit an Audio Pack folder:** a signed Whisper Pack (Spec 106). This installs local speech recognition. Audio never leaves the device.
2. **Import a synthetic WAV** (16 kHz, 16-bit, absolute path, at most 64 MiB), or the generated tone sample.
3. Choose the route:
   - *Local Whisper*: real recognition.
   - *Fixture engine*: a labelled test double, not recognition.
   - *Segmentation only*.
4. Then choose **Transcribe**.
5. **Correct** a segment with a reason. Each correction is a new revision.
6. **Draft note from this revision:** an admitted NER Pack finds entities in each segment and groups them by label. Every item cites its segment and time span. The draft is a proposal.

Transcripts are drafts. Medical accuracy is unmeasured. On synthetic speech, both pinned checkpoints missed "metformin".

## 7. Exports and the Audit Trail

- **Exports → Export a subject:**
  1. Choose the subject and state a purpose.
  2. Core runs the loss-aware FHIR R4 export and, in the same step, appends a disclosure record.
- The page shows each resource's lost or unsupported fields. Full conformance is never claimed.
- The desktop shows the export; it does not write a file.
- **Audit Trail** lists every disclosure, including these exports.

## 8. Limits that stay in force

- Synthetic data only. No real PHI.
- The 1 GiB Pack artifact bound is unchanged, so large generative models do not run.
- Packages are unsigned engineering builds, and release readiness stays false.
- Model outputs, transcripts, drafts and comparisons are proposals. A person decides.
