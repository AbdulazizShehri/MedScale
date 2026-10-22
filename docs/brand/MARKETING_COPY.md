# MedScale marketing copy system

**Status:** active for the public website (2026-10-22). It extends [`VOICE_AND_COPY.md`](VOICE_AND_COPY.md) and never overrides [`PRODUCT.md`](../../PRODUCT.md) or [`docs/qualification/EXTERNAL_GATES.md`](../qualification/EXTERNAL_GATES.md).

## Truth rules

Every public line must survive these checks.

1. **It is implemented, or it is labelled as direction.** "One workspace for your health AI" describes what the desktop app does today: records, evidence, local models and agents in one window. "Every health AI tool in one place" is vision and may appear only as clearly framed direction, never as a capability claim.
2. **No invented numbers.** No model counts, downloads, stars, benchmarks, customers, integrations or approvals unless a source in this repository proves them at the time of publishing. Catalog listings are not qualified models. Write "a browsable model catalog" rather than a count.
3. **No competitor comparisons on product surfaces.** Comparative results stay in technical evidence (`PRODUCT.md`).
4. **The preview state is always visible, and never shouts.** One quiet line in the hero, the downloads and the FAQ: open-source engineering preview, synthetic data only, not for clinical use.
5. **Screens are real or labelled.** Product imagery is a screenshot of the real desktop app with the synthetic workspace. Illustrations and interactive examples say so.

## Positioning

**Category:** an open-source clinical intelligence workspace.
**Promise:** records, evidence and local AI models in one source-aware workspace, with a person in charge of every decision.
**Difference:** most health AI tools give you an answer. MedScale shows the source behind it, runs the model on your machine, and keeps the decision with you.

### What makes MedScale different

- **One workspace, not a stack of tools.** Patients, documents, insights, models and agents share one window and one Rust Core.
- **The source stays in view.** Insights and agent runs link back to the record spans they used. Missing or conflicting evidence is shown, not smoothed over.
- **Models are inspectable.** You can see where a model came from, what runs it, and whether it is admitted. Being listed is not the same as being qualified.
- **Local first.** The Core runs on your machine. Network access is off by default and every outbound action is governed.
- **A person decides.** AI proposes. Changes that carry authority stay explicit and auditable.
- **Open source.** Apache-2.0 first-party code, built in public.

## Hero headlines (15 candidates)

1. **Your health AI. One open workspace.** ← selected
2. Open-source intelligence for healthcare.
3. Records, evidence and AI. Finally in one place.
4. The open workspace for clinical intelligence.
5. Health AI you can inspect.
6. See the source behind every answer.
7. Your clinical AI, brought together.
8. Less fragmentation. More understanding.
9. One workspace. Clearer decisions.
10. Local AI for health data, with the evidence in view.
11. Where health records meet open AI.
12. Built open. Designed for clarity.
13. Every insight, with its source.
14. The calm workspace for consequential work.
15. Health intelligence, on your machine.

**Selection rationale:** (1) names the category (health AI) and the shape (one open workspace) in six words. It is accurate today, memorable, and leaves room for the visual. (6) and (13) are the strongest proof lines, so they become the evidence section headline and a social line.

## Supporting descriptions (15)

1. **MedScale brings records, evidence and local AI models into one source-aware desktop workspace. Open source, local-first, and built so a person stays in charge of every decision.** ← selected
2. One window for patients, documents, insights, models and agents, with the source behind every result kept in view.
3. Explore health records, inspect AI models and run agents on your own machine, without losing track of where an answer came from.
4. An open-source Rust Core with a desktop app and a CLI. Your data stays local; your decisions stay yours.
5. Stop stitching together viewers, notebooks and model demos. MedScale keeps them in one inspectable place.
6. Every insight links back to the record it came from. Every model shows where it came from.
7. Built for clinicians, researchers and engineers who need to know why, not just what.
8. Local models, governed network access and an audit trail, in one open-source app.
9. A workspace where AI proposes and people decide.
10. From longitudinal records to grounded answers, with nothing hidden in between.
11. Bring your synthetic records, admitted models and questions. Leave with answers you can trace.
12. Designed to be quiet, fast and precise, because the work is the interface.
13. One Core, two surfaces: a desktop app for daily work and a CLI for automation.
14. Open, local and evidence-first, from the first import to the last audit entry.
15. The model catalog, the evidence and the record, side by side.

## Feature statements (10)

1. **Records with a history.** Longitudinal patient context, documents and coverage in one view.
2. **Answers with sources.** Insights cite the record spans they rely on and show what is missing.
3. **Models you can inspect.** Source, runtime and admission state for every model.
4. **Agents that leave receipts.** MedAgent runs are proposals, with bound context and recorded receipts.
5. **Local by default.** No default network egress. Browsing is denied until you allow it.
6. **An audit trail you can read.** Disclosures and exports are recorded by the Core.
7. **FHIR R4 in and out.** Bounded import and export, with disclosure before anything leaves.
8. **One Core, two surfaces.** The desktop app and the CLI share the same Rust authority.
9. **Keyboard first.** Command palette, search and navigation without hunting through menus.
10. **Open source.** Read the code, run the tests, open an issue.

## Social headlines (10)

1. Health AI you can inspect. MedScale is open source.
2. See the source behind every answer.
3. Your health AI. One open workspace.
4. Records, evidence and local models in one app. Open source.
5. AI proposes. You decide. Meet MedScale.
6. Local-first clinical intelligence, built in the open.
7. Every insight, with its source.
8. Stop stitching tools together. Start with one workspace.
9. Models are not authority. MedScale shows you why.
10. An open-source workspace for health data and AI. Preview now on GitHub.

## SEO title and description pairs (5)

| Page | Title | Description |
|---|---|---|
| Home | MedScale — Your health AI. One open workspace. | Open-source, local-first workspace for health records, evidence and AI models. Engineering preview for synthetic data. |
| Product | Product — MedScale | Records, insights, models, agents and governance in one source-aware desktop workspace with a shared Rust Core. |
| Models | Models — MedScale | How MedScale lists, admits and runs local AI models, and why a catalog listing is not clinical qualification. |
| Download | Download — MedScale | MedScale desktop for Windows and macOS and the CLI for Windows, macOS and Linux. Current release status and build-from-source steps. |
| Docs | Documentation — MedScale | Get started with MedScale: setup, the desktop app, the CLI, qualification boundaries, security and contributing. |

## Section messaging

**Download.** "Get MedScale." The current release status is stated plainly: no signed public installer yet, the desktop app builds from source on Windows and macOS, and the CLI runs on all three platforms. When a qualified release exists, the button resolves to the right installer for the visitor's platform.

**Developers.** "Built on a Rust Core you can read." It covers the four-command quickstart, the workspace layout (Core, CLI, desktop, FHIR, storage, packs) and a test suite that runs on every pull request.

**Community.** "Built in the open." Issues, pull requests and design discussion are welcome. Use synthetic data only: no patient information, ever.

## Words to avoid

AI-powered, revolutionary, seamless, cutting-edge, best-in-class, clinically proven, HIPAA-compliant, FDA, trusted by, enterprise-ready, magic, sparkle and star metaphors, and any claim of diagnosis or treatment.
