//! Bounded UCUM subset — no silent conversion outside admitted table.

use medscale_contracts::presentation::{
    UCUM_SUBSET_DIGEST_HEX, UCUM_SUBSET_ID, UnitComparability, UnitSemanticResult,
};

/// Admitted UCUM codes for H0-B (sorted canonical list).
pub const ADMITTED_UCUM_CODES: &[&str] = &[
    "%",
    "/min",
    "Cel",
    "L",
    "[degF]",
    "g",
    "g/dL",
    "kg",
    "lb",
    "mL",
    "mg",
    "mm[Hg]",
    "mmol/L",
    "{beats}/min",
];

/// Dimension / comparability group for admitted codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnitGroup {
    Mass,
    Rate,
    Pressure,
    Temperature,
    Volume,
    ConcentrationMass,
    ConcentrationMolar,
    Fraction,
}

fn group_of(code: &str) -> Option<UnitGroup> {
    match code {
        "kg" | "g" | "mg" | "lb" => Some(UnitGroup::Mass),
        "/min" | "{beats}/min" => Some(UnitGroup::Rate),
        "mm[Hg]" => Some(UnitGroup::Pressure),
        "Cel" | "[degF]" => Some(UnitGroup::Temperature),
        "mL" | "L" => Some(UnitGroup::Volume),
        "g/dL" => Some(UnitGroup::ConcentrationMass),
        "mmol/L" => Some(UnitGroup::ConcentrationMolar),
        "%" => Some(UnitGroup::Fraction),
        _ => None,
    }
}

/// Canonical form within subset (identity for admitted codes).
fn canonical_of(code: &str) -> Option<&'static str> {
    ADMITTED_UCUM_CODES.iter().copied().find(|&c| c == code)
}

/// Evaluate a raw unit string against the admitted subset.
#[must_use]
pub fn evaluate_unit(raw_unit: &str) -> UnitSemanticResult {
    let trimmed = raw_unit.trim();
    match canonical_of(trimmed) {
        Some(canon) => UnitSemanticResult {
            raw_unit: trimmed.to_owned(),
            admitted: true,
            canonical_unit: Some(canon.to_owned()),
            comparability: UnitComparability::Comparable,
        },
        None => UnitSemanticResult {
            raw_unit: trimmed.to_owned(),
            admitted: false,
            canonical_unit: None,
            comparability: UnitComparability::Unrecognized,
        },
    }
}

/// Compare two raw units: same group → Comparable; different admitted groups → Incomparable;
/// any unrecognized → Unrecognized (fail closed, no silent convert).
#[must_use]
pub fn compare_units(a: &str, b: &str) -> UnitComparability {
    let ea = evaluate_unit(a);
    let eb = evaluate_unit(b);
    if !ea.admitted || !eb.admitted {
        return UnitComparability::Unrecognized;
    }
    let ga = group_of(ea.canonical_unit.as_deref().unwrap_or(""));
    let gb = group_of(eb.canonical_unit.as_deref().unwrap_or(""));
    match (ga, gb) {
        (Some(x), Some(y)) if x == y => UnitComparability::Comparable,
        (Some(_), Some(_)) => UnitComparability::Incomparable,
        _ => UnitComparability::Unrecognized,
    }
}

/// `value` in `code` as `(group, value in the group's base unit)`, using only
/// definitional factors: mass in grams (1 lb = 453.59237 g exactly), volume in
/// millilitres, temperature in kelvin. Nothing analyte-specific: mg/dL and
/// mmol/L stay incomparable because converting them needs a molar mass.
fn to_base(value: f64, code: &str) -> Option<(UnitGroup, f64)> {
    let group = group_of(canonical_of(code.trim())?)?;
    let base = match code.trim() {
        "kg" => value * 1000.0,
        "g" | "mL" | "%" | "/min" | "{beats}/min" | "mm[Hg]" | "g/dL" | "mmol/L" => value,
        "mg" => value / 1000.0,
        "lb" => value * 453.592_37,
        "L" => value * 1000.0,
        "Cel" => value + 273.15,
        "[degF]" => (value + 459.67) * 5.0 / 9.0,
        _ => return None,
    };
    Some((group, base))
}

/// Whether two measurements state the same quantity: `Some(true)` when the
/// units are in one group of the admitted subset and the values agree after
/// conversion (relative tolerance 1e-9, absolute 1e-9 near zero), `Some(false)`
/// when they disagree, `None` when either unit is outside the subset or the
/// groups differ. Never a clinical judgement, only arithmetic on units.
#[must_use]
pub fn same_quantity(a: f64, a_unit: &str, b: f64, b_unit: &str) -> Option<bool> {
    if !a.is_finite() || !b.is_finite() {
        return None;
    }
    let (ga, va) = to_base(a, a_unit)?;
    let (gb, vb) = to_base(b, b_unit)?;
    if ga != gb {
        return None;
    }
    let tolerance = 1e-9 * va.abs().max(vb.abs()).max(1.0);
    Some((va - vb).abs() <= tolerance)
}

/// Subset pin metadata for evidence.
#[must_use]
pub fn subset_pin() -> (&'static str, &'static str) {
    (UCUM_SUBSET_ID, UCUM_SUBSET_DIGEST_HEX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn subset_digest_matches_sorted_list() {
        let mut joined = ADMITTED_UCUM_CODES.join("\n");
        joined.push('\n');
        let digest = Sha256::digest(joined.as_bytes());
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, UCUM_SUBSET_DIGEST_HEX);
    }

    #[test]
    fn incompatible_units_not_equal() {
        assert_eq!(compare_units("kg", "Cel"), UnitComparability::Incomparable);
        assert_eq!(compare_units("kg", "g"), UnitComparability::Comparable);
    }

    #[test]
    fn same_quantity_converts_within_a_group_only() {
        assert_eq!(same_quantity(5.0, "kg", 5000.0, "g"), Some(true));
        assert_eq!(same_quantity(1.0, "lb", 453.592_37, "g"), Some(true));
        assert_eq!(same_quantity(2.0, "lb", 0.907_184_74, "kg"), Some(true));
        assert_eq!(same_quantity(37.0, "Cel", 98.6, "[degF]"), Some(true));
        assert_eq!(same_quantity(1.5, "L", 1500.0, "mL"), Some(true));
        assert_eq!(same_quantity(72.0, "/min", 72.0, "{beats}/min"), Some(true));
        assert_eq!(same_quantity(5.0, "kg", 5.0, "g"), Some(false));
        assert_eq!(same_quantity(37.0, "Cel", 37.0, "[degF]"), Some(false));
        // No analyte-specific conversion, and nothing outside the subset.
        assert_eq!(same_quantity(100.0, "g/dL", 5.5, "mmol/L"), None);
        assert_eq!(same_quantity(5.0, "kg", 5.0, "Cel"), None);
        assert_eq!(same_quantity(5.0, "kg", 5.0, "furlong"), None);
        assert_eq!(same_quantity(f64::NAN, "kg", 5.0, "kg"), None);
    }

    #[test]
    fn unrecognized_no_silent_convert() {
        let r = evaluate_unit("furlong");
        assert!(!r.admitted);
        assert_eq!(r.comparability, UnitComparability::Unrecognized);
        assert!(r.canonical_unit.is_none());
        assert_eq!(
            compare_units("kg", "furlong"),
            UnitComparability::Unrecognized
        );
    }
}
