# Literature ledger

Search pass: 2026-09-30. See `LITERATURE_METHOD.md` for screening rules.

This ledger separates verified literature from candidate citations. Metadata is not sufficient for a strong novelty comparison; important comparison papers require primary-text review where accessible.

| Work | DOI | Year | Publication status | Relevance to MedScale | Review state / decision |
|---|---|---:|---|---|---|
| A Global Log for Medical AI | `10.48550/arxiv.2510.04033` | 2025 | arXiv preprint | Event-level logging, monitoring, agentic workflow records | INCLUDE; primary text reviewed |
| DoAtlas-1: A Causal Compilation Paradigm for Clinical AI | `10.48550/arxiv.2602.19158` | 2026 | arXiv preprint | Executable evidence/causal estimands, conflicts, auditability | INCLUDE; primary text reviewed |
| The Provenance Gap in Clinical AI | `10.48550/arxiv.2604.17114` | 2026 | arXiv preprint | Verified citations, evidence provenance, temporal knowledge graph | INCLUDE; primary text reviewed |
| Evidence-Graded Decision Authorization for Safe Clinical AI | `10.64898/2026.05.19.26353565` | 2026 | preprint | Evidence sufficiency vs permitted assertions; close authority-related conceptual prior | INCLUDE; primary text reviewed |
| Defining Operational Safety in Clinical Artificial Intelligence Systems | `10.1038/s41746-026-02450-7` | 2026 | peer-reviewed | Operational safe/gray zones, selective automation and review | INCLUDE; primary text reviewed |
| Monitoring Performance of Clinical Artificial Intelligence in Health Care | `10.11124/jbies-24-00042` | 2024 | peer-reviewed scoping review | Monitoring landscape | INCLUDE for background; deeper extraction pending |
| From Agents to Governance: Essential AI Skills for Clinicians in the LLM Era | `10.2196/86550` | 2026 | peer-reviewed | Bounded autonomy, provenance/citations, logging, abstention/escalation | INCLUDE; source text screened |
| FHIRTrustBench | `10.64898/2026.07.08.26357574` | 2026 | preprint | FHIR/interoperability as a trustworthiness dimension | INCLUDE as benchmark context; full-text extraction pending |
| Review Before Trust | `10.48550/arxiv.2608.29965` | 2026 | arXiv preprint | Direct prior for deterministic evidence gates, non-self-certification, fail-closed promotion and mutation testing | INCLUDE; primary arXiv text/abstract reviewed; novelty-narrowing prior |
| EviLedger | `10.21203/rs.3.rs-8769058/v1` | 2026 | Research Square preprint | Immutable/versioned evidence assertions, SHA-256 provenance, temporal reconstruction, rollback | INCLUDE; primary text reviewed; novelty-narrowing prior |
| Designing Portable Audit Evidence for Health Information Exchange (TrustEvidence) | `10.1080/08874417.2026.2720000` | 2026 | peer-reviewed, published online 2026-09-09 | FHIR R4/BALP-facing audit evidence, canonical signed bytes, adversarial mutation testing, retained verifier state, deterministic release reproduction | INCLUDE; primary publisher text reviewed; strong novelty-narrowing prior for provenance/audit evidence/reproducibility |
| An Auditable and Source-Verified Framework for Clinical AI Decision Support | `10.3389/frai.2026.1737532` | 2026 | peer-reviewed conceptual framework | RAG + provenance + tamper-evident logging; explicitly conceptual | INCLUDE as architecture/governance context |
| On-Premise Medical AI Agents for Reliable Clinical Decision-Making | `10.1038/s41591-026-04609-x` | 2026 | peer-reviewed, Nature Medicine | On-premise modular agents, limited-resource deployment, uncertainty/selective participation | INCLUDE; primary publisher text screened; novelty-narrowing prior |
| CLEAR: Cross-Source Evidence Adjudication for LLMs in Medicine | `10.48550/arxiv.2609.16301` | 2026 | arXiv preprint | Cross-source evidence/provenance verification and conflict adjudication | INCLUDE for model-level evidence adjudication; full-text extraction pending |
| When Silence Is Safer | `10.1038/s41746-026-02882-1` | 2026 | peer-reviewed review/framework | Uncertainty- and safety-driven abstention | INCLUDE for abstention context |
| Governing Generative AI in Healthcare | `10.3390/healthcare14081098` | 2026 | peer-reviewed conceptual framework | Epistemic authority, trust, responsibility tiers | INCLUDE for governance/authority context |
| Deterministic Integrity Gates for LLM-Assisted Clinical Manuscript Preparation | `10.48550/arxiv.2606.09500` | 2026 | arXiv preprint | Deterministic halt-on-failure integrity-gate methodology outside care delivery | INCLUDE as methodology-adjacent prior; not a clinical-system baseline |
| VeriFact | `10.48550/arxiv.2501.16672` | 2025 | arXiv preprint | Claim-level verification of LLM-generated clinical text against EHR evidence | CANDIDATE INCLUDE; abstract/metadata screened; full-text review pending |

## Screening rules

Prefer primary research and high-quality peer-reviewed frameworks/reviews for central claims. Preprints may be used when directly relevant and must be labeled as preprints. A paper is never assigned a negative feature in the novelty matrix merely because the feature was not mentioned in a search abstract. Use `NOT_REPORTED` until primary text establishes otherwise.

Retractions, corrections, and material citation-context concerns must be recorded here before final bibliography lock.
