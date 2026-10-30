# MedScale desktop: user guide (engineering preview)

MedScale is an engineering preview for **synthetic health workspaces** and optional, deliberately selected, licensed **nonpatient audio**. It is not clinically validated; real patient records, PHI and clinical decisions are not authorized. Every model output is a **proposal**; nothing becomes a record until a person accepts it through review. The [first-run guide](../BETA_FIRST_RUN.md) and [qualification record](../qualification/EXTERNAL_GATES.md) define the release and privacy boundaries.

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
- **MedScale Catalog:** a pinned, read-only catalog snapshot with documented upstream provenance, model licenses and per-model IDs. A listing does not mean MedScale owns the model or has rights to distribute its weights.
  - Search, filter and page through it. Each row shows MedScale's pre-download expectation (for example *executed tested* or *known unsupported*).
  - A listing is not a download and not an executable model.
- **Try a model:**
  - Run an admitted Pack on synthetic text.
  - For token classification, choose the entity decoding. *Word: first sub-token* is preselected because it scored at or above the original baseline decoder on every synthetic head-to-head row.
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

AudioFlow can process a **lawfully sourced nonpatient** recording locally. A model being listed in the catalog does not install it; Whisper weights are not bundled in the desktop installer.

1. In **Audio → Audio Model Manager**, choose a pinned Whisper model appropriate for the recording. English-only checkpoints do **not** support Arabic; choose a multilingual checkpoint and the Arabic language setting for Arabic recordings.
2. Select **Check disk budget**. Review the pinned model source, expected storage and available space. Check the explicit approval box and choose **Download, verify and install**. The optional model download is a consented network operation, not an automatic application update. You can also choose **Activate saved model** or **Browse folder… → Admit and install** for an already verified local Audio Pack.
3. Under **Import a WAV recording**, use **Browse WAV…** or enter an absolute file path. The supported import is **16-kHz, 16-bit PCM WAV, up to 32 MiB** (approximately 17 minutes for mono audio). Cancelling a native picker keeps an existing manual path. Choose **Import**; an optional **Import synthetic tone sample** exercises the fixture route without human speech.
4. Choose **Local Whisper (Audio Pack)** for actual speech recognition; **Fixture engine** is a test double and **Segmentation only** does not transcribe speech. Choose English or Arabic as appropriate, then **Transcribe** and inspect the draft.
5. **Correct** a segment with a reason to record a new revision. **Draft note** uses an independently admitted, local NER Pack and remains a reviewable proposal, not a clinical result.
6. Use **Stored data** to inspect a source's recording bytes, transcript revisions and evidence links before retention decisions. Confirmed removal is limited to eligible **unreferenced imported recordings in the active vault**; it does not erase backups, snapshots, linked transcripts or historical audit receipts. Never assume closing the app or uninstalling deletes recordings.

Real English and Arabic read-speech results, actual Windows/macOS native GUI evidence, and remaining OS privacy, accessibility and signing gaps are described in the [qualification record](../qualification/EXTERNAL_GATES.md). Read-speech benchmarks are not medical-speech accuracy measurements.

## 7. Exports and the Audit Trail

- **Exports → Export a subject:**
  1. Choose the subject and state a purpose.
  2. Core runs the loss-aware FHIR R4 export and, in the same step, appends a disclosure record.
- The page shows each resource's lost or unsupported fields. Full conformance is never claimed.
- The desktop shows the export; it does not write a file.
- **Audit Trail** lists every disclosure, including these exports.

## 8. Limits that stay in force

- Synthetic health/FHIR data only; optional licensed nonpatient audio is permitted with explicit selection and model approval. No real PHI or clinical use.
- The 1 GiB Pack artifact bound is unchanged, so large generative models do not run.
- Packages are unsigned engineering builds, and release readiness stays false.
- Model outputs, transcripts, drafts and comparisons are proposals. A person decides.
