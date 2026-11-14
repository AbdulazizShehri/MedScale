# Spec 089 — Research Packs (Clinical Research first)

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promotion date:** 2026-09-27
**Canonical base:** `1ac03622cb28046fc6386adab945578c82920f38` (Spec 088 closure merge)
**Target branch:** `spec/089-research-packs`

## Authority

Dependency proof: `RESEARCH_OS_V2_SPEC_IMPLEMENTATION_CONTRACTS.md` and
`RESEARCH_OS_EXECUTION_ROADMAP.md` number Research Packs **089**, hard
dependency **074 + 075 + 077 + 082 + 083** (085/087 only for executable
Pack features, which this slice does not have). All are
`CLOSED_CANONICAL`. This spec admits no dependency.

## Scope

Research Packs are declarative domain semantics, distinct from Extension
Packs (integration) and model/runtime Packs (weights). The first domain in
the authority's proof order is Clinical Research. This slice ships one
first-party Pack, `medscale.clinical-research`, in two versions compiled
into MedScale as canonical manifests, and proves:

- artifact schemas (study protocol, adverse event report, evidence claim)
  with closed field kinds, required fields and explicit `unknown` values;
- workflow state machines Core enforces (only declared transitions;
  compare-and-set revisions);
- evidence assessments whose every axis is explicit and may be `unknown`:
  citation exists, supports / contradicts / neutral, applicability,
  year, jurisdiction, guideline version, retraction, quality (GRADE-style),
  trial criteria; a missing citation cannot support, a retracted one is
  not recorded as support, and the summary verdict is `supported` only
  when existence, non-retraction, support and applicability are all
  recorded;
- install per Project (digest-pinned to the shipped manifest), upgrade
  applying the next version's declared non-destructive migrations to every
  artifact in one transaction, and uninstall that disables the Pack while
  keeping its data read-only; reinstall re-enables at the recorded
  version;
- no alternate authority plane: every object is Core-owned, in the
  Project, with receipts; installing a Pack grants no capability and
  installs no Extension.

Storage v18; CLI `medscale pack act|catalog|show|list`.

## Explicitly not authorized

- Third-party or downloadable Research Packs, Pack signing/distribution,
  executable Pack features, UI plugins.
- The other domains (AI Research, Systematic Review, Imaging, Omics, Wet
  Lab) — later Packs.
- Any claim that an assessment is clinically correct; real PHI.

Recorded residuals: citations are recorded as given, not resolved or
verified against a bibliographic source (no network); destructive
migrations (field removal or rename) are not supported; no Desktop surface.

## Completion rule

`CLOSED_CANONICAL` only after merge on a green exact head and recorded
post-main verification.
