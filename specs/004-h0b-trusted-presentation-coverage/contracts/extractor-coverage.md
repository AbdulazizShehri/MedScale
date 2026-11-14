# Contract: Typed Extractors + Coverage + Units (H0-B)

**Spec**: 004-h0b-trusted-presentation-coverage  
**Status**: Interface sketch for implement  
**Scope**: Closed typed extractors; coverage vocabulary; bounded UCUM subset — **no** general FHIRPath engine

## TypedResourceExtractor (logical trait)

```text
trait TypedResourceExtractor {
  fn resource_type(&self) -> &'static str;  // "Patient" | "Observation" | "Condition"
  fn extractor_version(&self) -> &'static str; // "patient.v1" | ...

  /// Input: promoted assertion payload + verified source bytes (or structured parse artifact).
  /// Output: fields + coverage contributions. Never executes FHIRPath expressions.
  fn extract(&self, input: ExtractorInput) -> Result<ExtractorOutput, ExtractorError>;
}
```

```text
ExtractorInput {
  assertion_id: OpaqueId,
  source_id: OpaqueId,
  content_digest: DigestSha256,
  resource_type: String,
  // Structural view produced by Spec 003 lexical accept — not a FHIRPath tree.
  structural: StructuralResourceView,
}

ExtractorOutput {
  fields: [ExtractedField],
  unsupported_paths_noted: [String],  // documentation only; not dynamic eval
}
```

### Admitted extractors (H0-B)

| resource_type | extractor_version | Field keys (non-exhaustive closed set) |
|---|---|---|
| Patient | `patient.v1` | `patient.name`, `patient.birthDate`, `patient.gender`, `patient.identifier` |
| Observation | `observation.v1` | `observation.code`, `observation.valueQuantity`, `observation.valueString`, `observation.effective`, `observation.status` |
| Condition | `condition.v1` | `condition.code`, `condition.clinicalStatus`, `condition.verificationStatus`, `condition.onset` |

Any other `resource_type` → do not invent extractor; emit `UnsupportedResourceType` coverage.

## Coverage rules

```text
fn cover(concept_key, extractor_outputs) -> CoverageSlot
```

| Condition | Status |
|---|---|
| No admitted extractor for concept | `Unknown` |
| Resource type unsupported | `UnsupportedResourceType` |
| Extractor ran; zero values | `Absent` |
| Exactly one healthy value | `Present` |
| ≥2 disagreeing values for same claim_key | `Conflict` |
| Quantities present but units incomparable | `IncomparableUnits` |
| Required source quarantined/missing | `UnhealthyEvidence` |

**Conflict disagreement** (typed): same `claim_key`, normalized values unequal under extractor equality rules (codes by system+code; quantities by unit-semantic compare; strings exact).

**Forbidden**: latest-wins, confidence-wins, silent drop of minority values.

## UnitSemantics (logical trait)

```text
trait UnitSemantics {
  fn subset_id(&self) -> &str;
  fn interpret(&self, value: DecimalLexical, unit: &str) -> UnitSemanticResult;
  fn comparable(&self, a: &UnitSemanticResult, b: &UnitSemanticResult) -> Comparability;
  /// Only when both Comparable and conversion admitted in subset pin.
  fn normalize_for_compare(&self, a, b) -> Result<OrderedCompare, Incomparable>;
}
```

### Admission rules

1. Pin `ucum_subset` id + content digest in evidence / research at implement.
2. Prefer minimal hand-pinned conversions for fixture units over large UCUM crates when equivalent.
3. Unrecognized unit → `comparability = Unrecognized`; never silent numeric equality across units.
4. Do **not** import OpenMed terminology tables or licensed SNOMED/LOINC content.

## Anti-APIs (forbidden in H0-B)

```text
FhirPathEngine::evaluate(expr)
JsonPath::query(expr)
DynamicExtractor::from_user_path(path)
BundleTerminologyServer::lookup(...)   // full terminology server out of scope
```

## Test obligations (contract-level)

- Per-extractor unit tests for Patient / Observation / Condition happy paths
- Absence → `Absent`
- Unsupported resource → `UnsupportedResourceType` / `Unknown`
- Conflict fixture → `Conflict` with ≥2 members
- Unit incomparable fixture → `IncomparableUnits`
- Golden rebuild uses extractor versions pinned in `PresentationRulesVersion`
