# Spec 083 — Knowledge + Research Canvas

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promoted:** 2026-09-24
**Base SHA:** see `docs/planning/SPEC_083_PROMOTION.md`
**Target branch:** `spec/083-knowledge-canvas`
**Dependency:** 074 + 075 + 077 + 079 (all closed). Integration gates 080,
081 and 082 are closed, so web evidence, transcript spans and analytics
derivatives may be indexed.
**Promotion authority:** `docs/planning/SPEC_083_PROMOTION.md`

## 1. Problem

A research Project collects evidence in several governed places: dataset
snapshots, transcripts, captured web pages and analytics results. Finding
"where did we see myalgia?" means opening each one. A search index that
copies text and forgets where it came from would turn search results into a
second, unpinned truth. Results from outdated or archived sources would be
served as if current.

## 2. Goal

One Core-owned, per-Project knowledge path:
- a rebuildable lexical index over exact source revisions, where every
  match resolves to an exact object, digest and span;
- freshness computed live, so superseded and archived sources are labelled
  and archived text is never shown;
- a receipt for every retrieval, with "insufficient evidence" as a
  first-class answer;
- a Research Canvas whose evidence nodes are live references, with
  inspection of unsupported notes, contradictions and missing evidence.

## 3. Scenarios

1. A researcher builds the index and searches "myalgia". Matches come from a
   snapshot cell, a corrected transcript segment, a captured page excerpt
   and an analytics result. Each names its exact revision and span.
2. The dataset is re-imported. Status reports the indexed snapshot as
   superseded and the new one as not yet indexed. A default search does not
   serve the superseded text; `include_stale` lists it, labelled.
3. The data source is archived. Its spans become tombstoned: listed only
   with `include_stale`, never with text. A rebuild drops them.
4. A search with no match is `insufficient_evidence`, not an empty success.
5. A researcher adds a note and cites a hit on a Canvas, links the evidence
   as `supports`, and later sees the evidence marked missing after the
   source is archived.
6. Project B cannot search, cite or read anything of Project A. Another
   scope reads nothing.

## 4. Requirements

- FR-01 `IndexManifest` binding exact source revisions (kind, object,
  content digest, lineage), tokenizer identity and a chunk-list digest.
- FR-02 Lexical retrieval only (`lexical-v1`: deterministic integer
  scoring). Vector retrieval is not admitted (Q28/Q29).
- FR-03 Freshness (`current`, `superseded`, `tombstoned`) computed per
  request; authorization before disclosure; no retrieval cache (Q31).
- FR-04 `RetrievalReceipt` for every request that reaches a Project,
  including refusals; `insufficient_evidence` outcome.
- FR-05 `EvidenceSpanRef` for snapshot cells, transcript segments (with time
  spans), web evidence excerpts and derived-table cells.
- FR-06 Research Canvas: immutable contiguous revisions, notes, evidence
  references and typed links; optimistic concurrency.
- FR-07 Canvas inspection: live resolution, unsupported notes,
  contradictions, superseded and missing evidence.
- FR-08 Storage v12, backup/restore and consistency checks.
- FR-09 CLI and Desktop over Core.

## 5. Out of scope

Vector or embedding retrieval, model-generated answers or summaries, a
literature library with citation metadata or PDF/OCR import (no admitted
parser), Project Graph expansion in retrieval plans, graphical canvas
layout, and real PHI.

## 6. Success criteria

The frozen acceptance requirements in `SPEC_083_PROMOTION.md`.
