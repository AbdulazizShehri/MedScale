# Spec 004 H0-B evidence

## Scope delivered

- Typed extractors: Patient / Observation / Condition (`medscale-fhir`)
- Bounded UCUM subset admission (`docs/engineering/admissions/004-ucum-subset.md`)
- Coverage Present/Absent/Unknown/Conflict/IncomparableUnits/UnhealthyEvidence/UnsupportedResourceType
- Deterministic SubjectTimelineV1 / SubjectBriefV1 / SubjectCoverageV1
- Facade: GetTimeline / GetBrief / GetCoverage / DrillDownPresentation + RebuildProjection kinds
- Golden rebuild + absence/conflict + FHIRPath ban lockfile check

## Commands

```text
cargo test --workspace
cargo test -p medscale-core --test h0b_presentation_coverage
cargo clippy --workspace --all-targets -- -D warnings
```

## Exit gates

| Gate | Status |
|---|---|
| No FHIRPath engine dependency | PASS (`no_fhirpath_engine_dependency`) |
| UCUM subset pin + fail-closed | PASS (unit suite + admission) |
| Timeline deterministic order | PASS |
| Coverage absence/conflict | PASS |
| Brief LLM-free | PASS (structured sections only) |
| Drill-down | PASS |
| Projection rebuild equality | PASS |
| Synthetic-only / DEFAULT_DENY | unchanged |

## Notes

- Conflict resolution remains `Unresolved` in H0-B
- Spec 005 encryption and Spec 006 product UI not in scope
