# MedScale research paper

Working title: **MedScale: A Local-First, Provenance-Centered Architecture for Auditable Clinical and Research AI**

This directory contains the research manuscript and reproducibility materials for the MedScale paper. The scientific snapshot is frozen at commit `1e2b7d94e970256b38bda15fa91f62bc397e825a` unless a later snapshot is explicitly adopted and recorded in `research/SNAPSHOT.md`.

## Claim discipline

This paper evaluates a systems architecture. It does **not** claim clinical validation, regulatory validation, production PHI readiness, or platform qualification. The frozen repository records `RELEASE_READY=false`, `PRIVATE_DATA_READY=false`, `PLATFORM_QUALIFIED=false`, `CLINICAL_VALIDATION_STATUS=NOT_PERFORMED`, and `REGULATORY_STATUS=NOT_PERFORMED`.

All quantitative manuscript claims must be regenerated from versioned experiment artifacts or bound to exact repository/CI evidence. Synthetic data only is authorized for the paper evaluation unless governance changes explicitly.

## Build

The manuscript intentionally uses a small arXiv-compatible LaTeX dependency surface.

```bash
cd paper
pdflatex main.tex
bibtex main
pdflatex main.tex
pdflatex main.tex
```

## Research workflow

1. Freeze repository evidence and map candidate claims to exact evidence.
2. Maintain the literature ledger and source-backed novelty matrix.
3. Pre-specify paper experiments before interpreting results.
4. Store raw results separately from generated tables/figures.
5. Draft manuscript claims only after evidence exists.
6. Run an adversarial claim/reproducibility review before arXiv packaging.

See `research/CLAIM_EVIDENCE_MAP.md`, `research/LITERATURE_LEDGER.md`, `research/RELATED_WORK_MATRIX.md`, and `research/EVALUATION_PLAN.md`.