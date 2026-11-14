# STANDARD admission: UCUM subset (Spec 004)

## medscale.ucum.subset.v1 (hand table — no crate dependency)

| Field | Value |
|---|---|
| Component | Bounded UCUM code table in `medscale-fhir::units` |
| Version / revision pin | `medscale.ucum.subset.v1` |
| Content digest (SHA-256) | `947866ded10e36783aa1f15d5eb8e90b79513f1fec1a9e27528bd1c7b1904303` |
| Canonical list | newline-joined sorted codes + trailing newline: `%`, `/min`, `Cel`, `L`, `[degF]`, `g`, `g/dL`, `kg`, `lb`, `mL`, `mg`, `mm[Hg]`, `mmol/L`, `{beats}/min` |
| Owning Spec | 004 |
| Purpose | Honest quantity comparability for Observation.valueQuantity in H0-B |
| Alternatives considered | Full UCUM crates (`ucum`, `fhir` unit packs); deferred — smallest reversible subset |
| License / NOTICE | Codes are UCUM public terminology references; table is MedScale-authored |
| Security / advisory review | No native dependency; compile-time static table |
| Placement | `medscale-fhir` units module behind `evaluate_unit` / `compare_units` |
| Tests required | Digest pin test; incompatible units; unrecognized fail-closed |
| Update strategy | Bump subset id + digest on any code add/remove; regenerate golden presentation |
| Exit strategy | Replace table with admitted UCUM library behind same trait |
| FHIRPath | **Not admitted** |

No model crates admitted.
