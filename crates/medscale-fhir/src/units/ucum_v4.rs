// SPDX-License-Identifier: Apache-2.0
// Adapted from maziyarpanahi/openmed@5b79cae5 (openmed/clinical/units/{ucum.py,convert.py});
// extended with explicit analyte molar-mass table for clinical concentration conversion.
//! Compositional UCUM-subset parser and dimension-checked unit converter (Wave 2).
//!
//! Coexists with the H0-B bounded table in [`super`]; all conversions are
//! strictly deterministic and return explicit `incommensurable`, `ambiguous`,
//! `unknown`, or `analyte_context_required` statuses instead of silently guessing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const MEASUREMENT_NORMALIZATION_ADVISORY: &str = "Measurement normalization is deterministic assistance for comparison and \
     is not a substitute for the originating laboratory's own formal diagnostic flagging.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UcumV4Status {
    Ok,
    Unknown,
    Ambiguous,
    Incommensurable,
    AnalyteContextRequired,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedUcumUnit {
    pub input: String,
    pub status: UcumV4Status,
    pub normalized_unit: Option<String>,
    pub canonical_unit: Option<String>,
    pub dimension: BTreeMap<String, i32>,
    pub scale: Option<f64>,
    pub offset: f64,
    pub is_affine: bool,
    pub reason: String,
    pub advisory: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UcumConversionResult {
    pub value: f64,
    pub from_unit: String,
    pub to_unit: String,
    pub status: UcumV4Status,
    pub magnitude: Option<f64>,
    pub canonical_magnitude: Option<f64>,
    pub canonical_unit: Option<String>,
    pub dimension: BTreeMap<String, i32>,
    pub analyte: Option<String>,
    pub reason: String,
    pub advisory: String,
}

#[derive(Debug, Clone)]
struct UnitAtom {
    dimension: BTreeMap<String, i32>,
    scale: f64,
    offset: f64,
    is_affine: bool,
}

#[derive(Debug, Clone)]
struct UnitExpr {
    normalized_unit: String,
    dimension: BTreeMap<String, i32>,
    scale: f64,
    offset: f64,
    is_affine: bool,
}

fn dim(pairs: &[(&str, i32)]) -> BTreeMap<String, i32> {
    let mut map = BTreeMap::new();
    for &(k, v) in pairs {
        if v != 0 {
            map.insert(k.to_owned(), v);
        }
    }
    map
}

fn canonical_unit_for(dimension: &BTreeMap<String, i32>) -> String {
    if dimension.is_empty() {
        return "1".to_owned();
    }
    let lookup: &[(&[(&str, i32)], &str)] = &[
        (&[("mass", 1)], "g"),
        (&[("amount", 1)], "mol"),
        (&[("equivalent_amount", 1)], "Eq"),
        (&[("volume", 1)], "L"),
        (&[("length", 1)], "m"),
        (&[("time", 1)], "s"),
        (&[("pressure", 1)], "Pa"),
        (&[("temperature", 1)], "K"),
        (&[("catalytic_activity", 1)], "U"),
        (&[("entity", 1)], "count"),
        (&[("mass", 1), ("volume", -1)], "g/L"),
        (&[("amount", 1), ("volume", -1)], "mol/L"),
        (&[("equivalent_amount", 1), ("volume", -1)], "Eq/L"),
        (&[("catalytic_activity", 1), ("volume", -1)], "U/L"),
        (&[("entity", 1), ("volume", -1)], "count/L"),
        (&[("entity", 1), ("time", -1)], "count/s"),
        (&[("volume", 1), ("time", -1)], "L/s"),
        (&[("mass", 1), ("length", -2)], "g/m2"),
    ];
    for (spec, canon) in lookup {
        if &dim(spec) == dimension {
            return (*canon).to_owned();
        }
    }
    let mut num = Vec::new();
    let mut den = Vec::new();
    for (name, &exp) in dimension {
        let p = exp.unsigned_abs();
        let tok = if p == 1 {
            name.clone()
        } else {
            format!("{name}^{p}")
        };
        if exp > 0 {
            num.push(tok);
        } else {
            den.push(tok);
        }
    }
    if den.is_empty() {
        num.join("*")
    } else if num.is_empty() {
        format!("/{}", den.join("*"))
    } else {
        format!("{}/{}", num.join("*"), den.join("*"))
    }
}

fn clean_unit_text(raw: &str) -> String {
    let mut s = raw
        .trim()
        .replace(['\u{03bc}', '\u{00b5}'], "u")
        .replace('\u{00b0}', "")
        .replace("**", "^")
        .replace(" x ", " ")
        .replace(" X ", " ")
        .replace(" per ", "/");
    while s.contains(" /") || s.contains("/ ") || s.contains(" ^") || s.contains("^ ") {
        s = s
            .replace(" /", "/")
            .replace("/ ", "/")
            .replace(" ^", "^")
            .replace("^ ", "^");
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn resolve_alias(raw: &str) -> Result<String, (UcumV4Status, String, Option<String>)> {
    let cleaned = clean_unit_text(raw);
    let key = cleaned.to_lowercase();
    if key.is_empty() {
        return Err((UcumV4Status::Unknown, "unit is empty".to_owned(), None));
    }
    if matches!(
        key.as_str(),
        "u" | "unit" | "units" | "international unit" | "international units"
    ) {
        return Err((
            UcumV4Status::Ambiguous,
            "unit string is clinically ambiguous without local context".to_owned(),
            Some(key),
        ));
    }
    let mapped = match key.as_str() {
        "percent" => "%",
        "mg/dl" | "milligram/deciliter" | "milligrams/deciliter" => "mg/dL",
        "g/dl" | "gram/deciliter" | "grams/deciliter" => "g/dL",
        "g/l" => "g/L",
        "mg/l" => "mg/L",
        "ug/ml" | "mcg/ml" => "ug/mL",
        "ng/ml" => "ng/mL",
        "mmol/l" => "mmol/L",
        "umol/l" => "umol/L",
        "meq/l" => "mEq/L",
        "ueq/l" => "uEq/L",
        "u/l" | "iu/l" | "units/l" => "U/L",
        "mm hg" | "mmhg" => "mmHg",
        "c" | "degc" | "celsius" => "Cel",
        "f" | "degf" | "fahrenheit" => "[degF]",
        "/min" | "per minute" => "1/min",
        "bpm" | "beats/min" | "beats per minute" => "beat/min",
        "breaths/min" | "breaths per minute" => "breath/min",
        "cells/ul" | "cell/ul" | "cells/mm3" | "cell/mm3" => "cell/uL",
        "k/ul" | "k/mm3" | "x10^3/ul" | "10^3/ul" | "10*3/ul" => "10*3/uL",
        "10^9/l" | "10*9/l" => "10*9/L",
        _ => return Ok(cleaned),
    };
    Ok(mapped.to_owned())
}

fn base_unit(symbol: &str) -> Option<UnitAtom> {
    let atom = match symbol {
        "1" => UnitAtom {
            dimension: BTreeMap::new(),
            scale: 1.0,
            offset: 0.0,
            is_affine: false,
        },
        "%" => UnitAtom {
            dimension: BTreeMap::new(),
            scale: 0.01,
            offset: 0.0,
            is_affine: false,
        },
        "g" => UnitAtom {
            dimension: dim(&[("mass", 1)]),
            scale: 1.0,
            offset: 0.0,
            is_affine: false,
        },
        "mcg" => UnitAtom {
            dimension: dim(&[("mass", 1)]),
            scale: 1e-6,
            offset: 0.0,
            is_affine: false,
        },
        "mol" => UnitAtom {
            dimension: dim(&[("amount", 1)]),
            scale: 1.0,
            offset: 0.0,
            is_affine: false,
        },
        "Eq" => UnitAtom {
            dimension: dim(&[("equivalent_amount", 1)]),
            scale: 1.0,
            offset: 0.0,
            is_affine: false,
        },
        "L" | "l" => UnitAtom {
            dimension: dim(&[("volume", 1)]),
            scale: 1.0,
            offset: 0.0,
            is_affine: false,
        },
        "m" => UnitAtom {
            dimension: dim(&[("length", 1)]),
            scale: 1.0,
            offset: 0.0,
            is_affine: false,
        },
        "s" => UnitAtom {
            dimension: dim(&[("time", 1)]),
            scale: 1.0,
            offset: 0.0,
            is_affine: false,
        },
        "min" => UnitAtom {
            dimension: dim(&[("time", 1)]),
            scale: 60.0,
            offset: 0.0,
            is_affine: false,
        },
        "h" => UnitAtom {
            dimension: dim(&[("time", 1)]),
            scale: 3600.0,
            offset: 0.0,
            is_affine: false,
        },
        "d" => UnitAtom {
            dimension: dim(&[("time", 1)]),
            scale: 86400.0,
            offset: 0.0,
            is_affine: false,
        },
        "Pa" => UnitAtom {
            dimension: dim(&[("pressure", 1)]),
            scale: 1.0,
            offset: 0.0,
            is_affine: false,
        },
        "kPa" => UnitAtom {
            dimension: dim(&[("pressure", 1)]),
            scale: 1000.0,
            offset: 0.0,
            is_affine: false,
        },
        "mmHg" => UnitAtom {
            dimension: dim(&[("pressure", 1)]),
            scale: 133.322_387_415,
            offset: 0.0,
            is_affine: false,
        },
        "K" => UnitAtom {
            dimension: dim(&[("temperature", 1)]),
            scale: 1.0,
            offset: 0.0,
            is_affine: false,
        },
        "Cel" => UnitAtom {
            dimension: dim(&[("temperature", 1)]),
            scale: 1.0,
            offset: 273.15,
            is_affine: true,
        },
        "[degF]" => UnitAtom {
            dimension: dim(&[("temperature", 1)]),
            scale: 5.0 / 9.0,
            offset: 255.372_222_222_222_2,
            is_affine: true,
        },
        "U" => UnitAtom {
            dimension: dim(&[("catalytic_activity", 1)]),
            scale: 1.0,
            offset: 0.0,
            is_affine: false,
        },
        "cell" | "cells" | "beat" | "breath" => UnitAtom {
            dimension: dim(&[("entity", 1)]),
            scale: 1.0,
            offset: 0.0,
            is_affine: false,
        },
        _ => return None,
    };
    Some(atom)
}

fn lookup_atom(symbol: &str) -> Option<UnitAtom> {
    if let Some(a) = base_unit(symbol) {
        return Some(a);
    }
    if let Some(exp_str) = symbol
        .strip_prefix("10*")
        .or_else(|| symbol.strip_prefix("10^"))
        && let Ok(exp) = exp_str.parse::<i32>()
    {
        return Some(UnitAtom {
            dimension: BTreeMap::new(),
            scale: 10f64.powi(exp),
            offset: 0.0,
            is_affine: false,
        });
    }
    const PREFIXES: &[(&str, f64)] = &[
        ("da", 10.0),
        ("k", 1e3),
        ("d", 1e-1),
        ("c", 1e-2),
        ("m", 1e-3),
        ("u", 1e-6),
        ("n", 1e-9),
        ("p", 1e-12),
    ];
    for &(pref, factor) in PREFIXES {
        if let Some(rest) = symbol.strip_prefix(pref)
            && matches!(rest, "g" | "mol" | "Eq" | "L" | "l" | "m" | "s")
            && let Some(mut base) = base_unit(rest)
        {
            base.scale *= factor;
            return Some(base);
        }
    }
    None
}

fn parse_factor(token: &str) -> Option<(UnitAtom, i32)> {
    let tok = token.trim();
    if tok.is_empty() {
        return None;
    }
    if let Some(exp_part) = tok.strip_prefix("10*").or_else(|| tok.strip_prefix("10^"))
        && exp_part.chars().all(|c| c == '-' || c.is_ascii_digit())
    {
        return lookup_atom(tok).map(|a| (a, 1));
    }
    let clean = tok.replace('^', "");
    let split_idx = clean
        .char_indices()
        .rev()
        .take_while(|&(_, c)| c.is_ascii_digit() || c == '-')
        .last()
        .map(|(idx, _)| idx);

    let (sym, exp) = match split_idx {
        Some(idx) if idx > 0 && !clean[..idx].ends_with('*') => {
            let e = clean[idx..].parse::<i32>().ok()?;
            (&clean[..idx], e)
        }
        _ => (clean.as_str(), 1),
    };
    let atom = lookup_atom(sym)?;
    Some((atom, exp))
}

fn parse_expression(raw: &str) -> Result<UnitExpr, (UcumV4Status, String, Option<String>)> {
    let normalized = resolve_alias(raw)?;
    let mut terms: Vec<(bool, &str)> = Vec::new();
    let mut cur_start = 0usize;
    let mut is_denom = false;
    let bytes = normalized.as_bytes();
    for (idx, &b) in bytes.iter().enumerate() {
        if b == b'/' || b == b'*' {
            let part = normalized[cur_start..idx].trim();
            if part.is_empty() && !(idx == 0 && b == b'/') {
                return Err((
                    UcumV4Status::Unknown,
                    "malformed unit separator".to_owned(),
                    Some(normalized),
                ));
            }
            if !part.is_empty() {
                terms.push((is_denom, part));
            }
            if b == b'/' {
                is_denom = true;
            }
            cur_start = idx + 1;
        }
    }
    let tail = normalized[cur_start..].trim();
    if tail.is_empty() {
        return Err((
            UcumV4Status::Unknown,
            "trailing unit operator".to_owned(),
            Some(normalized),
        ));
    }
    terms.push((is_denom, tail));

    let mut total_dim: BTreeMap<String, i32> = BTreeMap::new();
    let mut total_scale = 1.0_f64;
    let mut affine_offset = 0.0_f64;
    let mut has_affine = false;

    for (idx, (denom, tok)) in terms.iter().enumerate() {
        let Some((atom, raw_exp)) = parse_factor(tok) else {
            return Err((
                UcumV4Status::Unknown,
                format!("unknown UCUM token `{tok}`"),
                Some(normalized),
            ));
        };
        let signed_exp = if *denom {
            raw_exp.saturating_neg()
        } else {
            raw_exp
        };
        if signed_exp.abs() > 100 {
            return Err((
                UcumV4Status::Unknown,
                format!("unit exponent out of range in `{tok}`"),
                Some(normalized),
            ));
        }
        if atom.is_affine {
            if terms.len() != 1 || idx != 0 || signed_exp != 1 {
                return Err((
                    UcumV4Status::Unknown,
                    "affine temperature units cannot appear in compound expressions".to_owned(),
                    Some(normalized),
                ));
            }
            has_affine = true;
            affine_offset = atom.offset;
        }
        total_scale *= atom.scale.powi(signed_exp);
        if !total_scale.is_finite() || total_scale <= 0.0 {
            return Err((
                UcumV4Status::Unknown,
                "unit scale overflow or underflow".to_owned(),
                Some(normalized),
            ));
        }
        for (dim_name, dim_exp) in atom.dimension {
            let next = total_dim
                .get(&dim_name)
                .copied()
                .unwrap_or(0)
                .saturating_add(dim_exp.saturating_mul(signed_exp));
            if next == 0 {
                total_dim.remove(&dim_name);
            } else {
                total_dim.insert(dim_name, next);
            }
        }
    }

    Ok(UnitExpr {
        normalized_unit: normalized,
        dimension: total_dim,
        scale: total_scale,
        offset: affine_offset,
        is_affine: has_affine,
    })
}

/// Parses a unit string under the compositional Wave 2 UCUM rules.
#[must_use]
pub fn parse_ucum_v4(raw_unit: &str) -> ParsedUcumUnit {
    match parse_expression(raw_unit) {
        Ok(expr) => {
            let canon = canonical_unit_for(&expr.dimension);
            ParsedUcumUnit {
                input: raw_unit.to_owned(),
                status: UcumV4Status::Ok,
                normalized_unit: Some(expr.normalized_unit),
                canonical_unit: Some(canon),
                dimension: expr.dimension,
                scale: Some(expr.scale),
                offset: expr.offset,
                is_affine: expr.is_affine,
                reason: "parsed".to_owned(),
                advisory: MEASUREMENT_NORMALIZATION_ADVISORY.to_owned(),
            }
        }
        Err((status, reason, norm)) => ParsedUcumUnit {
            input: raw_unit.to_owned(),
            status,
            normalized_unit: norm,
            canonical_unit: None,
            dimension: BTreeMap::new(),
            scale: None,
            offset: 0.0,
            is_affine: false,
            reason,
            advisory: MEASUREMENT_NORMALIZATION_ADVISORY.to_owned(),
        },
    }
}

fn requires_analyte_context(src: &BTreeMap<String, i32>, dst: &BTreeMap<String, i32>) -> bool {
    let mut s = src.clone();
    let mut d = dst.clone();
    let sm = s.remove("mass").unwrap_or(0);
    let sa = s.remove("amount").unwrap_or(0);
    let se = s.remove("equivalent_amount").unwrap_or(0);
    let dm = d.remove("mass").unwrap_or(0);
    let da = d.remove("amount").unwrap_or(0);
    let de = d.remove("equivalent_amount").unwrap_or(0);
    s == d && (sm != dm || sa != da || se != de) && (sm + sa + se == dm + da + de)
}

/// Definitional dimension-checked conversion matching `medscale_clinical.clinical.units.convert.normalize_to`.
#[must_use]
pub fn normalize_ucum_v4(value: f64, from_unit: &str, to_unit: &str) -> UcumConversionResult {
    if !value.is_finite() {
        return UcumConversionResult {
            value,
            from_unit: from_unit.to_owned(),
            to_unit: to_unit.to_owned(),
            status: UcumV4Status::Unknown,
            magnitude: None,
            canonical_magnitude: None,
            canonical_unit: None,
            dimension: BTreeMap::new(),
            analyte: None,
            reason: "measurement value must be finite".to_owned(),
            advisory: MEASUREMENT_NORMALIZATION_ADVISORY.to_owned(),
        };
    }
    let src = match parse_expression(from_unit) {
        Ok(e) => e,
        Err((status, reason, _)) => {
            return UcumConversionResult {
                value,
                from_unit: from_unit.to_owned(),
                to_unit: to_unit.to_owned(),
                status,
                magnitude: None,
                canonical_magnitude: None,
                canonical_unit: None,
                dimension: BTreeMap::new(),
                analyte: None,
                reason,
                advisory: MEASUREMENT_NORMALIZATION_ADVISORY.to_owned(),
            };
        }
    };
    let dst = match parse_expression(to_unit) {
        Ok(e) => e,
        Err((status, reason, _)) => {
            return UcumConversionResult {
                value,
                from_unit: from_unit.to_owned(),
                to_unit: to_unit.to_owned(),
                status,
                magnitude: None,
                canonical_magnitude: None,
                canonical_unit: None,
                dimension: BTreeMap::new(),
                analyte: None,
                reason,
                advisory: MEASUREMENT_NORMALIZATION_ADVISORY.to_owned(),
            };
        }
    };
    if src.dimension != dst.dimension {
        let status = if requires_analyte_context(&src.dimension, &dst.dimension) {
            UcumV4Status::AnalyteContextRequired
        } else {
            UcumV4Status::Incommensurable
        };
        let reason = if status == UcumV4Status::AnalyteContextRequired {
            "converting between mass, molar, and equivalent units requires analyte molar mass or valence".to_owned()
        } else {
            "source and target units have incompatible physical dimensions".to_owned()
        };
        return UcumConversionResult {
            value,
            from_unit: src.normalized_unit,
            to_unit: dst.normalized_unit,
            status,
            magnitude: None,
            canonical_magnitude: None,
            canonical_unit: None,
            dimension: BTreeMap::new(),
            analyte: None,
            reason,
            advisory: MEASUREMENT_NORMALIZATION_ADVISORY.to_owned(),
        };
    }
    let canonical_mag = (value * src.scale) + src.offset;
    let out_mag = (canonical_mag - dst.offset) / dst.scale;
    if !canonical_mag.is_finite() || !out_mag.is_finite() {
        return UcumConversionResult {
            value,
            from_unit: src.normalized_unit,
            to_unit: dst.normalized_unit,
            status: UcumV4Status::Unknown,
            magnitude: None,
            canonical_magnitude: None,
            canonical_unit: None,
            dimension: BTreeMap::new(),
            analyte: None,
            reason: "conversion magnitude overflow".to_owned(),
            advisory: MEASUREMENT_NORMALIZATION_ADVISORY.to_owned(),
        };
    }
    let canon_unit = canonical_unit_for(&src.dimension);
    UcumConversionResult {
        value,
        from_unit: src.normalized_unit,
        to_unit: dst.normalized_unit,
        status: UcumV4Status::Ok,
        magnitude: Some(out_mag),
        canonical_magnitude: Some(canonical_mag),
        canonical_unit: Some(canon_unit),
        dimension: src.dimension,
        analyte: None,
        reason: "converted".to_owned(),
        advisory: MEASUREMENT_NORMALIZATION_ADVISORY.to_owned(),
    }
}

/// Transparent molar-mass table (g/mol) for explicit analyte-aware mass <-> molar conversion.
#[must_use]
pub fn analyte_molar_mass_g_per_mol(analyte: &str) -> Option<f64> {
    match analyte.trim().to_lowercase().as_str() {
        "glucose" => Some(180.156),
        "creatinine" => Some(113.12),
        "cholesterol" | "ldl" | "hdl" => Some(386.65),
        "triglycerides" => Some(885.43),
        "bilirubin" => Some(584.66),
        "calcium" => Some(40.078),
        "magnesium" => Some(24.305),
        "urea_bun" | "bun" | "urea" => Some(28.014),
        "uric_acid" => Some(168.11),
        _ => None,
    }
}

/// Converts clinical concentration between mass and molar units when the caller explicitly
/// names an analyte (or supplies its molar mass in g/mol).
#[must_use]
pub fn convert_analyte_concentration(
    value: f64,
    from_unit: &str,
    to_unit: &str,
    analyte: &str,
    explicit_molar_mass_g_per_mol: Option<f64>,
) -> UcumConversionResult {
    let direct = normalize_ucum_v4(value, from_unit, to_unit);
    if direct.status != UcumV4Status::AnalyteContextRequired {
        return direct;
    }
    let Some(molar_mass) = explicit_molar_mass_g_per_mol
        .filter(|m| m.is_finite() && *m > 0.0)
        .or_else(|| analyte_molar_mass_g_per_mol(analyte))
    else {
        return UcumConversionResult {
            analyte: Some(analyte.to_owned()),
            reason: format!("unknown analyte `{analyte}` and no positive molar mass provided"),
            ..direct
        };
    };

    let Ok(src) = parse_expression(from_unit) else {
        return direct;
    };
    let Ok(dst) = parse_expression(to_unit) else {
        return direct;
    };
    let sm = src.dimension.get("mass").copied().unwrap_or(0);
    let sa = src.dimension.get("amount").copied().unwrap_or(0);
    let dm = dst.dimension.get("mass").copied().unwrap_or(0);
    let da = dst.dimension.get("amount").copied().unwrap_or(0);
    if !((sm == 1 && sa == 0 && dm == 0 && da == 1) || (sm == 0 && sa == 1 && dm == 1 && da == 0)) {
        return direct;
    }

    // Source value in canonical base (either g/... or mol/...):
    let canonical_src = value * src.scale;
    let canonical_dst = if sm == 1 && da == 1 {
        canonical_src / molar_mass
    } else {
        canonical_src * molar_mass
    };
    let out_mag = canonical_dst / dst.scale;
    UcumConversionResult {
        value,
        from_unit: src.normalized_unit,
        to_unit: dst.normalized_unit,
        status: UcumV4Status::Ok,
        magnitude: Some(out_mag),
        canonical_magnitude: Some(canonical_dst),
        canonical_unit: Some(canonical_unit_for(&dst.dimension)),
        dimension: dst.dimension,
        analyte: Some(analyte.trim().to_lowercase()),
        reason: format!("converted using analyte molar mass {molar_mass} g/mol"),
        advisory: MEASUREMENT_NORMALIZATION_ADVISORY.to_owned(),
    }
}
