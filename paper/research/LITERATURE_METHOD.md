# Literature review method

## Status

This is a structured, living literature review for the MedScale systems paper. It is **not yet claimed to be an exhaustive systematic review**.

Search date for the current pass: **2026-09-30**.

## Sources

- Scite literature search, metadata, citation context, and available full text;
- primary arXiv records for preprints;
- primary publisher pages / full text where available;
- DOI metadata for bibliography verification.

Search-result titles alone are not sufficient evidence for a novelty claim. Direct comparison rows require primary-source review or are marked `UNDER_REVIEW` / `NOT_REPORTED`.

## Search themes

The current search pass covers combinations of:

1. clinical / medical AI architecture;
2. clinical LLM and medical foundation-model evaluation;
3. evidence-grounded generation and clinical factuality verification;
4. provenance, lineage, evidence ledgers, and auditability;
5. authority, trust promotion, evidence gating, and human review;
6. uncertainty, calibration, selective prediction, and abstention;
7. monitoring, logging, and post-deployment governance;
8. FHIR/interoperability-aware trustworthiness;
9. local/private/on-premise medical AI;
10. agentic clinical systems and bounded autonomy;
11. recovery, rollback, reproducibility, and tamper-evident clinical AI state.

## Inclusion criteria

A work is prioritized when it materially informs at least one MedScale research claim, experimental design choice, novelty boundary, or limitation. Central novelty comparisons prioritize primary technical/research papers and peer-reviewed frameworks/reviews; directly relevant preprints are retained but labeled as preprints.

## Screening and decision rules

1. Screen title/abstract for direct relevance.
2. Review primary text for papers used in the novelty matrix where accessible.
3. Record DOI, year, publication status, relevance, and the exact claim/dimension it informs.
4. Record corrections/retractions or material citation-context concerns when found.
5. Use `NOT_REPORTED` rather than `NO` when a source does not establish a property.
6. Do not infer absence of an implementation feature from absence in an abstract.
7. Do not claim MedScale originated provenance, logging, evidence gating, local inference, uncertainty, or abstention; these have clear prior art.
8. Refine or remove novelty claims when a closer prior work is found.

## Current novelty-screen consequence

The September 2026 search materially narrowed the paper's novelty hypothesis. In particular:

- *Review Before Trust* directly demonstrates deterministic source-grounded trust promotion, generator/verifier authority separation, fail-closed admission, and mutation testing in a clinical-record path.
- *EviLedger* directly addresses immutable/versioned clinical-AI evidence assertions, cryptographic provenance, temporal state, and rollback.
- *On-Premise Medical AI Agents for Reliable Clinical Decision-Making* directly addresses local/on-premise medical AI and uncertainty-aware selective participation.

Accordingly, MedScale must **not** claim novelty for those ideas in isolation. The remaining hypothesis is systems-level and must be demonstrated empirically: whether authority separation across source/derived/model/human/action/effect planes, provenance-bound artifacts, explicit subsystem failure states, fail-closed external boundaries, and recovery qualification can be enforced together and audited at an exact system revision.

## Bibliography lock

Before arXiv packaging, every cited item will receive a final metadata and status check. Author lists, DOI, venue, year, volume, pages, and publication status will not be guessed when authoritative metadata is unavailable.
