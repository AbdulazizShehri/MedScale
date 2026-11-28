// SPDX-License-Identifier: Apache-2.0
// Adapted from maziyarpanahi/openmed@5b79cae5 (openmed/clinical/{sections,context,negation_scope,
// experiencer,sig_parser,med_reconciliation,dosing_check,sdoh,radiology_report,pathology_profile,
// discharge_summary,nli_gate,claim_conflicts}.py).
// Third-party attribution: Assertion, negation, temporality, and experiencer rules include patterns
// adapted from medspaCy ConText (https://github.com/medspacy/medspacy, Apache-2.0), retrieved via
// OpenMed@5b79cae5 and ported into MedScale on 2026-11-28.
// Local modifications: pure-Rust zero-dependency implementation backed by conformance vectors.
//! OpenMed v4.0.0 clinical NLP, structured medication sig, reconciliation, guarded dosing check,
//! relations/events, SDOH (without bundling DUA SHAC text), specialty report templates, NLI gate,
//! and claim-conflict detection.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CLINICIAN_REVIEW_DISCLAIMER: &str = "Assistive decision support for clinician review only. This output is not a diagnosis, \
     treatment decision, or autonomous clinical decision. A qualified clinician must \
     independently review the cited source basis and confirm before any clinical action.";

pub const SHAC_NON_BUNDLING_NOTICE: &str = "SDOH extraction uses deterministic synthetic/public rules; the DUA-gated SHAC corpus is never bundled.";

// ---------------------------------------------------------------------------
// 1. Clinical Sections + Local LOINC Crosswalk
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClinicalSectionSpan {
    pub label: String,
    pub header: String,
    pub start: usize,
    pub end: usize,
    pub content_start: usize,
    pub loinc_code: Option<String>,
}

static SECTION_HEADER_ALIASES: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    let raw: Value = serde_json::from_str(include_str!("data/section_headers.yaml.json"))
        .expect("bundled section_headers JSON parses");
    let mut map = BTreeMap::new();
    if let Some(secs) = raw["sections"].as_object() {
        for (canon, aliases) in secs {
            map.insert(canon.replace('_', " "), canon.clone());
            if let Some(arr) = aliases.as_array() {
                for a in arr {
                    if let Some(s) = a.as_str() {
                        map.insert(s.trim().to_lowercase(), canon.clone());
                    }
                }
            }
        }
    }
    map.entry("discharge diagnosis".to_owned())
        .or_insert_with(|| "discharge_diagnoses".to_owned());
    map.entry("discharge diagnoses".to_owned())
        .or_insert_with(|| "discharge_diagnoses".to_owned());
    map.entry("discharge medications".to_owned())
        .or_insert_with(|| "discharge_medications".to_owned());
    map
});

#[must_use]
pub fn section_loinc_code(label: &str) -> Option<&'static str> {
    match label {
        "allergies" => Some("48765-2"),
        "assessment" => Some("51848-0"),
        "assessment_and_plan" => Some("51847-2"),
        "chief_complaint" => Some("8661-1"),
        "family_history" => Some("10157-6"),
        "findings" => Some("30954-2"),
        "history" | "history_of_present_illness" => Some("10164-2"),
        "impression" => Some("19005-8"),
        "medications" | "discharge_medications" => Some("10160-0"),
        "past_medical_history" => Some("11348-0"),
        "plan" => Some("18776-5"),
        "problem_list" => Some("11450-4"),
        "review_of_systems" => Some("10187-3"),
        "social_history" => Some("29762-2"),
        _ => None,
    }
}

/// Detects canonical clinical note sections using byte/char offsets matching `sections.detect_sections`.
#[must_use]
pub fn detect_sections(text: &str) -> Vec<ClinicalSectionSpan> {
    let mut headers: Vec<(usize, usize, String, String)> = Vec::new();
    let mut line_char_start = 0usize;
    for raw_line in text.split_inclusive('\n') {
        let stripped = raw_line.trim_end_matches(['\r', '\n']);
        let trimmed = stripped.trim();
        if let Some(colon_pos) = trimmed.find(':') {
            let head = trimmed[..colon_pos].trim();
            let norm = head.to_lowercase();
            let rest = trimmed[colon_pos + 1..].trim();
            if rest.is_empty()
                && let Some(canon) = SECTION_HEADER_ALIASES.get(&norm)
            {
                headers.push((
                    line_char_start,
                    line_char_start + raw_line.chars().count(),
                    canon.clone(),
                    head.to_owned(),
                ));
            }
        }
        line_char_start += raw_line.chars().count();
    }
    if !text.ends_with('\n') && line_char_start < text.chars().count() {
        let tail: String = text.chars().skip(line_char_start).collect();
        let trimmed = tail.trim();
        if let Some(colon_pos) = trimmed.find(':') {
            let head = trimmed[..colon_pos].trim();
            let norm = head.to_lowercase();
            if trimmed[colon_pos + 1..].trim().is_empty()
                && let Some(canon) = SECTION_HEADER_ALIASES.get(&norm)
            {
                headers.push((
                    line_char_start,
                    text.chars().count(),
                    canon.clone(),
                    head.to_owned(),
                ));
            }
        }
    }

    let total_chars = text.chars().count();
    let mut out = Vec::with_capacity(headers.len());
    for i in 0..headers.len() {
        let (start, content_start, ref label, ref header) = headers[i];
        let end = if i + 1 < headers.len() {
            headers[i + 1].0
        } else {
            total_chars
        };
        out.push(ClinicalSectionSpan {
            label: label.clone(),
            header: header.clone(),
            start,
            end,
            content_start,
            loinc_code: section_loinc_code(label).map(str::to_owned),
        });
    }
    out
}

// ---------------------------------------------------------------------------
// 2. medspaCy ConText: Negation Scope, Experiencer, Temporality, Certainty
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanInput {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NegationScopeHit {
    pub cue: String,
    pub direction: String,
    pub scope_start: usize,
    pub scope_end: usize,
    pub governed: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperiencerResult {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub experiencer: String,
    pub cue: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClinicalContextAssertion {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub negation: String,
    pub temporality: String,
    pub certainty: String,
    pub experiencer: String,
}

#[derive(Debug, Deserialize)]
struct RawContextRulesFile {
    rules: Vec<RawContextRule>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawContextRule {
    category: String,
    cue: String,
    direction: String,
    #[serde(default)]
    max_scope_tokens: usize,
}

static CONTEXT_RULES: LazyLock<Vec<RawContextRule>> = LazyLock::new(|| {
    let parsed: RawContextRulesFile =
        serde_json::from_str(include_str!("data/context_rules.yaml.json"))
            .expect("bundled context_rules JSON parses");
    parsed.rules
});

fn is_word_boundary(chars: &[char], idx: usize) -> bool {
    idx == 0
        || idx == chars.len()
        || !(chars[idx - 1].is_alphanumeric() && chars[idx].is_alphanumeric())
}

fn find_cue_spans(text_lower_chars: &[char], cue: &str) -> Vec<(usize, usize)> {
    let cue_chars: Vec<char> = cue.to_lowercase().chars().collect();
    if cue_chars.is_empty() || text_lower_chars.len() < cue_chars.len() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for i in 0..=text_lower_chars.len() - cue_chars.len() {
        if text_lower_chars[i..i + cue_chars.len()] == cue_chars[..] {
            let start_ok = i == 0 || !text_lower_chars[i - 1].is_alphanumeric();
            let end = i + cue_chars.len();
            let end_ok = end == text_lower_chars.len() || !text_lower_chars[end].is_alphanumeric();
            if start_ok && end_ok {
                hits.push((i, end));
            }
        }
    }
    hits
}

const PSEUDO_NEGATIONS: &[&str] = &[
    "cannot be excluded",
    "not ruled out",
    "no change",
    "no significant change",
    "without difficulty",
];

const TERMINATORS: &[&str] = &[
    "and",
    "but",
    "however",
    "or",
    "although",
    "though",
    "nevertheless",
    "yet",
    "still",
    "except",
    "aside from",
    "apart from",
    "secondary to",
    "which",
    "who",
    "because",
];

fn next_terminator_pos(lower_chars: &[char], from: usize) -> usize {
    let mut best = lower_chars.len();
    for (idx, &c) in lower_chars.iter().enumerate().skip(from) {
        if matches!(c, '.' | '!' | '?' | ';') {
            best = best.min(idx);
            break;
        }
    }
    for term in TERMINATORS {
        if let Some((start, _)) = find_cue_spans(lower_chars, term)
            .into_iter()
            .find(|&(s, _)| s >= from && is_word_boundary(lower_chars, s))
        {
            best = best.min(start);
        }
    }
    best
}

fn prev_terminator_pos(lower_chars: &[char], before: usize) -> usize {
    let mut best = 0usize;
    for idx in (0..before).rev() {
        if matches!(lower_chars[idx], '.' | '!' | '?' | ';') {
            best = idx + 1;
            break;
        }
    }
    for term in TERMINATORS {
        if let Some((_, end)) = find_cue_spans(lower_chars, term)
            .into_iter()
            .rfind(|&(_, e)| e <= before)
        {
            best = best.max(end);
        }
    }
    best
}

/// Computes exact forward/backward negation scope spans terminating at punctuation and
/// NegEx/ConText conjunction terminators (`but`, `however`, `and`, `or`, `;`, `.`).
#[must_use]
pub fn detect_negation_scopes(text: &str, spans: &[SpanInput]) -> Vec<NegationScopeHit> {
    let orig_chars: Vec<char> = text.chars().collect();
    let lower_chars: Vec<char> = text.to_lowercase().chars().collect();

    let pseudo_ranges: Vec<(usize, usize)> = PSEUDO_NEGATIONS
        .iter()
        .flat_map(|p| find_cue_spans(&lower_chars, p))
        .collect();

    let mut cues: Vec<&RawContextRule> = CONTEXT_RULES
        .iter()
        .filter(|r| r.category == "negation")
        .collect();
    // Add compact fallback cues if absent from JSON rules:
    let extra_neg = [
        ("no", "forward"),
        ("denies", "forward"),
        ("denied", "forward"),
        ("without", "forward"),
        ("negative for", "forward"),
        ("ruled out", "backward"),
        ("absent", "backward"),
        ("not present", "backward"),
        ("resolved", "backward"),
    ];

    let mut candidate_cues: Vec<(String, String, usize)> = cues
        .drain(..)
        .map(|r| (r.cue.clone(), r.direction.clone(), r.max_scope_tokens))
        .collect();
    for (cue, dir) in extra_neg {
        if !candidate_cues
            .iter()
            .any(|(c, _, _)| c.eq_ignore_ascii_case(cue))
        {
            candidate_cues.push((cue.to_owned(), dir.to_owned(), 40));
        }
    }
    candidate_cues.sort_by_key(|b| std::cmp::Reverse(b.0.len()));

    let mut claimed_cue_ranges: Vec<(usize, usize)> = Vec::new();
    let mut results = Vec::new();
    for (cue_str, direction, _max_tok) in candidate_cues {
        for (c_start, c_end) in find_cue_spans(&lower_chars, &cue_str) {
            if pseudo_ranges
                .iter()
                .any(|&(ps, pe)| c_start < pe && c_end > ps)
                || claimed_cue_ranges
                    .iter()
                    .any(|&(cs, ce)| c_start < ce && c_end > cs)
            {
                continue;
            }
            claimed_cue_ranges.push((c_start, c_end));
            let (scope_start, scope_end) = if direction == "backward" {
                (prev_terminator_pos(&lower_chars, c_start), c_start)
            } else {
                // Note: comma is NOT a terminator so coordinated negation ("no fever, chills, or chest pain")
                // keeps initial clauses, while `or` / `but` terminates as matched by Python `negation_scope.py`
                (c_end, next_terminator_pos(&lower_chars, c_end))
            };
            let mut governed = Vec::new();
            for sp in spans {
                if sp.start >= scope_start && sp.end <= scope_end {
                    governed.push((sp.start, sp.end));
                }
            }
            results.push(NegationScopeHit {
                cue: orig_chars[c_start..c_end].iter().collect(),
                direction: direction.clone(),
                scope_start,
                scope_end,
                governed,
            });
        }
    }
    results.sort_by_key(|r| r.scope_start);
    results
}

const FAMILY_CUES: &[&str] = &[
    "family history",
    "fhx",
    "familial",
    "maternal",
    "paternal",
    "grandmother",
    "grandfather",
    "grandparent",
    "mother",
    "father",
    "sister",
    "brother",
    "sibling",
    "parent",
    "parents",
    "aunt",
    "uncle",
];

const OTHER_CUES: &[&str] = &["donor", "roommate", "caregiver", "spouse", "partner"];

fn clause_bounds(lower_chars: &[char], span_start: usize, span_end: usize) -> (usize, usize) {
    let right_bound = span_end.min(lower_chars.len());
    let mut left = span_start.min(right_bound);
    while left > 0 && !matches!(lower_chars[left - 1], '.' | '!' | '?' | ';' | '\n') {
        left -= 1;
    }
    let mut right = right_bound;
    while right < lower_chars.len() && !matches!(lower_chars[right], '.' | '!' | '?' | ';' | '\n') {
        right += 1;
    }
    (left, right)
}

/// Refines experiencer attribution per span within its local clause (`patient` / `family` / `other`).
#[must_use]
pub fn refine_experiencer(text: &str, spans: &[SpanInput]) -> Vec<ExperiencerResult> {
    let lower_chars: Vec<char> = text.to_lowercase().chars().collect();
    spans
        .iter()
        .map(|sp| {
            let (c_left, c_right) = clause_bounds(&lower_chars, sp.start, sp.end);
            let clause = &lower_chars[c_left..c_right];
            let mut assigned = "patient".to_owned();
            let mut matched_cue = None;
            for &cue in FAMILY_CUES {
                if !find_cue_spans(clause, cue).is_empty() {
                    assigned = "family".to_owned();
                    matched_cue = Some(cue.to_owned());
                    break;
                }
            }
            if matched_cue.is_none() {
                for &cue in OTHER_CUES {
                    if !find_cue_spans(clause, cue).is_empty() {
                        assigned = "other".to_owned();
                        matched_cue = Some(cue.to_owned());
                        break;
                    }
                }
            }
            ExperiencerResult {
                text: sp.text.clone(),
                start: sp.start,
                end: sp.end,
                experiencer: assigned,
                cue: matched_cue,
            }
        })
        .collect()
}

/// Evaluates full ConText assertion axes (`negation`, `temporality`, `certainty`, `experiencer`)
/// over all candidate spans inside `text`.
#[must_use]
pub fn assert_context_axes(text: &str, spans: &[SpanInput]) -> Vec<ClinicalContextAssertion> {
    let negs = detect_negation_scopes(text, spans);
    let exps = refine_experiencer(text, spans);
    let lower_chars: Vec<char> = text.to_lowercase().chars().collect();
    let hist_cues = [
        "history of",
        "hx of",
        "past history",
        "prior",
        "previous",
        "s/p",
        "status post",
    ];
    let hypo_cues = ["if ", "should ", "risk of", "in case of", "prophylaxis"];
    let uncert_cues = [
        "rule out",
        "r/o",
        "possible",
        "probable",
        "suspected",
        "questionable",
        "may have",
    ];

    spans
        .iter()
        .enumerate()
        .map(|(i, sp)| {
            let is_neg = negs.iter().any(|n| {
                n.governed
                    .iter()
                    .any(|&(gs, ge)| gs == sp.start && ge == sp.end)
            });
            let (c_left, c_right) = clause_bounds(&lower_chars, sp.start, sp.end);
            let clause = &lower_chars[c_left..c_right];
            let temporality = if hypo_cues
                .iter()
                .any(|c| !find_cue_spans(clause, c.trim()).is_empty())
            {
                "hypothetical"
            } else if hist_cues
                .iter()
                .any(|c| !find_cue_spans(clause, c).is_empty())
            {
                "historical"
            } else {
                "recent"
            };
            let certainty = if uncert_cues
                .iter()
                .any(|c| !find_cue_spans(clause, c).is_empty())
            {
                "uncertain"
            } else {
                "certain"
            };
            ClinicalContextAssertion {
                text: sp.text.clone(),
                start: sp.start,
                end: sp.end,
                negation: if is_neg { "negated" } else { "affirmed" }.to_owned(),
                temporality: temporality.to_owned(),
                certainty: certainty.to_owned(),
                experiencer: exps[i].experiencer.clone(),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 3. Structured Medication Sig Parser (`sig_parser.py`)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedSig {
    pub raw: String,
    pub dose: Option<f64>,
    pub unit: Option<String>,
    pub form: Option<String>,
    pub route: Option<String>,
    pub frequency_per_day: Option<f64>,
    pub frequency_period: Option<u32>,
    pub frequency_period_unit: Option<String>,
    pub as_needed: bool,
    pub condition: Option<String>,
    pub duration_days: Option<u32>,
    pub missing: Vec<String>,
}

fn token_at_boundary(lower: &str, pat: &str) -> Option<(usize, usize)> {
    let mut from = 0usize;
    while let Some(rel) = lower[from..].find(pat) {
        let s = from + rel;
        let e = s + pat.len();
        let left_ok = s == 0 || !lower.as_bytes()[s - 1].is_ascii_alphanumeric();
        let right_ok = e == lower.len() || !lower.as_bytes()[e].is_ascii_alphanumeric();
        if left_ok && right_ok {
            return Some((s, e));
        }
        from = s + 1;
    }
    None
}

#[must_use]
pub fn parse_sig(raw: &str) -> ParsedSig {
    let lower = raw.to_lowercase();
    let route_lex: &[(&str, &str)] = &[
        ("by mouth", "oral"),
        ("per os", "oral"),
        ("po", "oral"),
        ("intravenous", "intravenous"),
        ("iv", "intravenous"),
        ("intramuscular", "intramuscular"),
        ("im", "intramuscular"),
        ("subcutaneous", "subcutaneous"),
        ("subcut", "subcutaneous"),
        ("subq", "subcutaneous"),
        ("sq", "subcutaneous"),
        ("sc", "subcutaneous"),
        ("sublingual", "sublingual"),
        ("sl", "sublingual"),
        ("per rectum", "rectal"),
        ("rectally", "rectal"),
        ("pr", "rectal"),
        ("topically", "topical"),
        ("topical", "topical"),
        ("inhaled", "inhaled"),
        ("nebulized", "inhaled"),
        ("inh", "inhaled"),
        ("intranasal", "nasal"),
        ("nasally", "nasal"),
        ("ophthalmic", "ophthalmic"),
    ];
    let form_lex: &[(&str, &str)] = &[
        ("tablets", "tablet"),
        ("tablet", "tablet"),
        ("tabs", "tablet"),
        ("tab", "tablet"),
        ("capsules", "capsule"),
        ("capsule", "capsule"),
        ("caps", "capsule"),
        ("cap", "capsule"),
        ("puffs", "puff"),
        ("puff", "puff"),
        ("drops", "drop"),
        ("drop", "drop"),
        ("gtt", "drop"),
        ("sprays", "spray"),
        ("spray", "spray"),
        ("patches", "patch"),
        ("patch", "patch"),
        ("suppository", "suppository"),
        ("supp", "suppository"),
        ("lozenge", "lozenge"),
    ];
    let dose_units = [
        "mg", "mcg", "g", "kg", "ml", "l", "unit", "units", "meq", "iu", "%",
    ];

    let route = route_lex
        .iter()
        .find_map(|&(cue, norm)| token_at_boundary(&lower, cue).map(|_| norm.to_owned()));

    let mut form: Option<String> = None;
    for &(cue, norm) in form_lex {
        if token_at_boundary(&lower, cue).is_some() {
            form = Some(norm.to_owned());
            break;
        }
    }

    // Parse first `<number> <unit_or_form>` token outside `x7 days` / `q4h`
    let mut dose: Option<f64> = None;
    let mut unit: Option<String> = None;
    let words: Vec<&str> = lower.split_whitespace().collect();
    for i in 0..words.len() {
        let w = words[i].trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '%');
        if let Ok(num) = w.parse::<f64>()
            && let Some(next_raw) = words.get(i + 1)
        {
            let next_w = next_raw.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '%');
            if dose_units.contains(&next_w) {
                dose = Some(num);
                unit = Some(next_w.to_owned());
                break;
            }
            if let Some((_, norm_form)) = form_lex.iter().find(|&&(f, _)| f == next_w) {
                dose = Some(num);
                form = Some((*norm_form).to_owned());
                unit = Some((*norm_form).to_owned());
                break;
            }
        }
    }

    // Frequency:
    let mut freq_per_day: Option<f64> = None;
    let mut freq_period: Option<u32> = None;
    let mut freq_period_unit: Option<String> = None;
    for tok in &words {
        let clean = tok.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        if let Some(rest) = clean.strip_prefix('q')
            && let Some(num_str) = rest.strip_suffix('h')
            && let Ok(hours) = num_str.parse::<u32>()
            && hours > 0
        {
            freq_period = Some(hours);
            freq_period_unit = Some("h".to_owned());
            freq_per_day = Some(24.0 / f64::from(hours));
            break;
        }
    }
    if freq_per_day.is_none() {
        freq_per_day = if token_at_boundary(&lower, "qid").is_some()
            || token_at_boundary(&lower, "four times daily").is_some()
            || token_at_boundary(&lower, "four times a day").is_some()
        {
            Some(4.0)
        } else if token_at_boundary(&lower, "tid").is_some()
            || token_at_boundary(&lower, "three times daily").is_some()
            || token_at_boundary(&lower, "three times a day").is_some()
        {
            Some(3.0)
        } else if token_at_boundary(&lower, "bid").is_some()
            || token_at_boundary(&lower, "twice daily").is_some()
            || token_at_boundary(&lower, "twice a day").is_some()
            || token_at_boundary(&lower, "two times daily").is_some()
        {
            Some(2.0)
        } else if token_at_boundary(&lower, "qd").is_some()
            || token_at_boundary(&lower, "daily").is_some()
            || token_at_boundary(&lower, "once daily").is_some()
        {
            Some(1.0)
        } else {
            None
        };
    }

    // Duration (`x7 days`, `for 5 days`, `x2 weeks`):
    let mut duration_days: Option<u32> = None;
    for i in 0..words.len() {
        let w = words[i].trim_matches(|c: char| !c.is_ascii_alphanumeric());
        let candidate_days = if let Some(rest) = w.strip_prefix('x') {
            rest.parse::<u32>().ok()
        } else if i > 0 && words[i - 1] == "for" {
            w.parse::<u32>().ok()
        } else {
            None
        };
        if let Some(n) = candidate_days
            && let Some(unit_tok) = words.get(i + 1)
        {
            let u = unit_tok.trim_matches(|c: char| !c.is_ascii_alphabetic());
            if u.starts_with("day") || u == "d" {
                duration_days = Some(n);
                break;
            } else if u.starts_with("week") || u == "wk" {
                duration_days = Some(n * 7);
                break;
            }
        }
    }

    // PRN condition:
    let mut as_needed = false;
    let mut condition = None;
    for cue in ["prn", "as needed"] {
        if let Some((_, cue_end)) = token_at_boundary(&lower, cue) {
            as_needed = true;
            let tail = raw[cue_end..].trim();
            let cleaned = tail
                .strip_prefix("for ")
                .unwrap_or(tail)
                .trim_matches(|c: char| matches!(c, '.' | ',' | ';'))
                .trim();
            if !cleaned.is_empty() {
                condition = Some(cleaned.to_owned());
            }
            break;
        }
    }

    let mut missing = Vec::new();
    if dose.is_none() {
        missing.push("dose".to_owned());
    }
    if route.is_none() {
        missing.push("route".to_owned());
    }
    if freq_per_day.is_none() {
        missing.push("frequency".to_owned());
    }

    ParsedSig {
        raw: raw.to_owned(),
        dose,
        unit,
        form,
        route,
        frequency_per_day: freq_per_day,
        frequency_period: freq_period,
        frequency_period_unit: freq_period_unit,
        as_needed,
        condition,
        duration_days,
        missing,
    }
}

// ---------------------------------------------------------------------------
// 4. Medication Reconciliation & Guarded Dose-Range Advisory
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MedicationMentionInput {
    pub medication: String,
    #[serde(default)]
    pub dose: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciledMedicationRecord {
    pub ingredient: String,
    pub current_status: String,
    pub current_dose: Option<String>,
    pub current_route: Option<String>,
    pub history_len: usize,
    pub has_conflict: bool,
}

fn canonical_med_status(raw: Option<&str>) -> String {
    match raw.unwrap_or("continued").trim().to_lowercase().as_str() {
        "start" | "started" | "initiate" | "initiated" | "add" | "added" => "started",
        "hold" | "held" | "withhold" | "withheld" | "on hold" => "held",
        "change" | "changed" | "increase" | "increased" | "decrease" | "decreased" | "titrated" => {
            "changed"
        }
        "stop" | "stopped" | "discontinue" | "discontinued" | "ceased" => "stopped",
        _ => "continued",
    }
    .to_owned()
}

#[must_use]
pub fn reconcile_medications(
    mentions: &[MedicationMentionInput],
) -> Vec<ReconciledMedicationRecord> {
    let mut order: Vec<String> = Vec::new();
    let mut grouped: BTreeMap<String, Vec<MedicationMentionInput>> = BTreeMap::new();
    for m in mentions {
        let key = m.medication.trim().to_lowercase();
        if !grouped.contains_key(&key) {
            order.push(key.clone());
        }
        grouped.entry(key).or_default().push(m.clone());
    }

    order
        .into_iter()
        .filter_map(|key| {
            let mut items = grouped.remove(&key)?;
            items.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
            let last = items.last()?.clone();
            let distinct_doses: BTreeSet<String> = items
                .iter()
                .filter_map(|x| x.dose.as_ref().map(|s| s.trim().to_lowercase()))
                .collect();
            let current_status = canonical_med_status(last.status.as_deref());
            let has_conflict = distinct_doses.len() > 1 && current_status == "continued";
            Some(ReconciledMedicationRecord {
                ingredient: last.medication.trim().to_owned(),
                current_status,
                current_dose: last.dose,
                current_route: last.route,
                history_len: items.len(),
                has_conflict,
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoseCheckInput {
    pub drug: String,
    pub route: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoseRangeSpec {
    pub drug: String,
    pub route: String,
    pub low: Option<f64>,
    pub high: Option<f64>,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoseCheckAdvisory {
    pub drug: String,
    pub route: String,
    pub status: String,
    pub requires_clinician_review: bool,
    pub autonomous_decision: bool,
    pub disclaimer: String,
}

fn mass_unit_mg_factor(unit: &str) -> Option<f64> {
    match unit.trim().to_lowercase().as_str() {
        "kg" => Some(1_000_000.0),
        "g" => Some(1_000.0),
        "mg" => Some(1.0),
        "mcg" | "ug" => Some(0.001),
        _ => None,
    }
}

#[must_use]
pub fn check_dose_ranges(
    doses: &[DoseCheckInput],
    reference_ranges: &[DoseRangeSpec],
) -> Vec<DoseCheckAdvisory> {
    doses
        .iter()
        .map(|d| {
            let matching = reference_ranges.iter().find(|r| {
                r.drug.eq_ignore_ascii_case(&d.drug) && r.route.eq_ignore_ascii_case(&d.route)
            });
            let status = if let Some(r) = matching {
                if let (Some(obs_f), Some(ref_f)) =
                    (mass_unit_mg_factor(&d.unit), mass_unit_mg_factor(&r.unit))
                {
                    let obs_mg = d.value * obs_f;
                    if r.high.is_some_and(|hi| obs_mg > hi * ref_f) {
                        "above_range"
                    } else if r.low.is_some_and(|lo| obs_mg < lo * ref_f) {
                        "below_range"
                    } else {
                        "within_range"
                    }
                } else if d.unit.eq_ignore_ascii_case(&r.unit) {
                    if r.high.is_some_and(|hi| d.value > hi) {
                        "above_range"
                    } else if r.low.is_some_and(|lo| d.value < lo) {
                        "below_range"
                    } else {
                        "within_range"
                    }
                } else {
                    "not_checked_unit_mismatch"
                }
            } else {
                "not_checked_missing_reference"
            };
            DoseCheckAdvisory {
                drug: d.drug.clone(),
                route: d.route.clone(),
                status: status.to_owned(),
                requires_clinician_review: true,
                autonomous_decision: false,
                disclaimer: CLINICIAN_REVIEW_DISCLAIMER.to_owned(),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 5. Clinical Relations & Events + SDOH + Specialty Templates
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClinicalRelation {
    pub relation_type: String,
    pub source_text: String,
    pub target_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClinicalEvent {
    pub event_type: String,
    pub subject: String,
    pub temporality: String,
}

#[must_use]
pub fn extract_relations_and_events(text: &str) -> (Vec<ClinicalRelation>, Vec<ClinicalEvent>) {
    let mut relations = Vec::new();
    let mut events = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let sig = parse_sig(trimmed);
        if sig.dose.is_some() || sig.route.is_some() {
            let drug_name = trimmed
                .split_whitespace()
                .next()
                .unwrap_or("medication")
                .to_owned();
            if let Some(d) = sig.dose {
                let u = sig.unit.clone().unwrap_or_default();
                relations.push(ClinicalRelation {
                    relation_type: "medication_has_dose".to_owned(),
                    source_text: drug_name.clone(),
                    target_text: format!("{d} {u}").trim().to_owned(),
                });
            }
            if let Some(r) = sig.route {
                relations.push(ClinicalRelation {
                    relation_type: "medication_has_route".to_owned(),
                    source_text: drug_name.clone(),
                    target_text: r,
                });
            }
            events.push(ClinicalEvent {
                event_type: "medication_administration".to_owned(),
                subject: drug_name,
                temporality: if sig.as_needed {
                    "hypothetical"
                } else {
                    "recent"
                }
                .to_owned(),
            });
        }
    }
    (relations, events)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdohFinding {
    pub category: String,
    pub value: String,
    pub status: String,
    pub temporality: String,
}

#[must_use]
pub fn extract_sdoh(text: &str) -> Vec<SdohFinding> {
    let lower = text.to_lowercase();
    let mut out = Vec::new();
    if lower.contains("unemployed") || lower.contains("lost job") {
        out.push(SdohFinding {
            category: "employment".to_owned(),
            value: "unemployed".to_owned(),
            status: "unemployed".to_owned(),
            temporality: "recent".to_owned(),
        });
    } else if lower.contains("employed") || lower.contains("works full time") {
        out.push(SdohFinding {
            category: "employment".to_owned(),
            value: "employed".to_owned(),
            status: "employed".to_owned(),
            temporality: "recent".to_owned(),
        });
    }
    if lower.contains("lives alone") {
        out.push(SdohFinding {
            category: "living_status".to_owned(),
            value: "lives_alone".to_owned(),
            status: "lives_alone".to_owned(),
            temporality: "recent".to_owned(),
        });
    } else if lower.contains("homeless") || lower.contains("unhoused") {
        out.push(SdohFinding {
            category: "living_status".to_owned(),
            value: "homeless".to_owned(),
            status: "homeless".to_owned(),
            temporality: "recent".to_owned(),
        });
    }
    if lower.contains("former smoker") || lower.contains("quit smoking") {
        out.push(SdohFinding {
            category: "tobacco".to_owned(),
            value: "smoker".to_owned(),
            status: "past".to_owned(),
            temporality: "historical".to_owned(),
        });
    } else if lower.contains("current smoker") || lower.contains("smokes daily") {
        out.push(SdohFinding {
            category: "tobacco".to_owned(),
            value: "smoker".to_owned(),
            status: "current".to_owned(),
            temporality: "recent".to_owned(),
        });
    } else if lower.contains("never smoked") || lower.contains("non-smoker") {
        out.push(SdohFinding {
            category: "tobacco".to_owned(),
            value: "smoker".to_owned(),
            status: "none".to_owned(),
            temporality: "recent".to_owned(),
        });
    }
    if lower.contains("food insecurity") || lower.contains("runs out of food") {
        out.push(SdohFinding {
            category: "food_insecurity".to_owned(),
            value: "food_insecurity".to_owned(),
            status: "current".to_owned(),
            temporality: "recent".to_owned(),
        });
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadiologyTemplateResult {
    pub findings_text: String,
    pub impression_text: String,
    pub recommendation_text: String,
}

#[must_use]
pub fn parse_radiology_report(text: &str) -> RadiologyTemplateResult {
    let mut findings = String::new();
    let mut impression = String::new();
    let mut rec = String::new();
    let mut active = "";
    for line in text.lines() {
        let trim = line.trim();
        let upper = trim.to_uppercase();
        if let Some(rest) = upper
            .strip_prefix("FINDINGS:")
            .map(|_| trim["FINDINGS:".len()..].trim())
        {
            active = "findings";
            findings.push_str(rest);
        } else if let Some(rest) = upper
            .strip_prefix("IMPRESSION:")
            .map(|_| trim["IMPRESSION:".len()..].trim())
        {
            active = "impression";
            impression.push_str(rest);
        } else if let Some(rest) = upper
            .strip_prefix("RECOMMENDATION:")
            .map(|_| trim["RECOMMENDATION:".len()..].trim())
        {
            active = "recommendation";
            rec.push_str(rest);
        } else if upper.starts_with("EXAM:") || upper.starts_with("INDICATION:") {
            active = "";
        } else if !trim.is_empty() {
            match active {
                "findings" => {
                    if !findings.is_empty() {
                        findings.push(' ');
                    }
                    findings.push_str(trim);
                }
                "impression" => {
                    if !impression.is_empty() {
                        impression.push(' ');
                    }
                    impression.push_str(trim);
                }
                "recommendation" => {
                    if !rec.is_empty() {
                        rec.push(' ');
                    }
                    rec.push_str(trim);
                }
                _ => {}
            }
        }
    }
    RadiologyTemplateResult {
        findings_text: findings,
        impression_text: impression,
        recommendation_text: rec,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathologyProfileResult {
    pub specimens: Vec<String>,
    pub diagnoses: Vec<String>,
}

#[must_use]
pub fn extract_pathology_profile(text: &str) -> PathologyProfileResult {
    let mut specimens = Vec::new();
    let mut diagnoses = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        let upper = t.to_uppercase();
        if upper.starts_with("SPECIMEN:") {
            let val = t["SPECIMEN:".len()..].trim();
            if !val.is_empty() {
                specimens.push(val.to_owned());
            }
        } else if upper.starts_with("DIAGNOSIS:") {
            let val = t["DIAGNOSIS:".len()..].trim();
            if !val.is_empty() {
                diagnoses.push(val.to_owned());
            }
        }
    }
    PathologyProfileResult {
        specimens,
        diagnoses,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DischargeSummaryResult {
    pub discharge_diagnoses: Vec<String>,
    pub discharge_medications: Vec<ParsedSig>,
}

#[must_use]
pub fn extract_discharge_summary(text: &str) -> DischargeSummaryResult {
    let secs = detect_sections(text);
    let chars: Vec<char> = text.chars().collect();
    let mut diagnoses = Vec::new();
    let mut meds = Vec::new();
    for s in secs {
        let body: String = chars[s.content_start..s.end].iter().collect();
        match s.label.as_str() {
            "discharge_diagnoses" | "diagnosis" => {
                for l in body.lines() {
                    let t = l.trim();
                    if !t.is_empty() {
                        diagnoses.push(t.to_owned());
                    }
                }
            }
            "discharge_medications" | "medications" => {
                for l in body.lines() {
                    let t = l.trim();
                    if !t.is_empty() {
                        meds.push(parse_sig(t));
                    }
                }
            }
            _ => {}
        }
    }
    DischargeSummaryResult {
        discharge_diagnoses: diagnoses,
        discharge_medications: meds,
    }
}

// ---------------------------------------------------------------------------
// 6. Selective NLI Abstention Gate & Claim Conflict Check
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NliGateDecision {
    pub outcome: String,
    pub selected_probability: f64,
    pub margin: f64,
    pub requires_human_review: bool,
}

#[must_use]
pub fn evaluate_nli_gate(entailment: f64, contradiction: f64, neutral: f64) -> NliGateDecision {
    const THRESHOLD: f64 = 0.90;
    const MIN_MARGIN: f64 = 0.05;
    let max_non_e = contradiction.max(neutral);
    let max_non_c = entailment.max(neutral);
    if entailment >= THRESHOLD && (entailment - max_non_e) > MIN_MARGIN {
        NliGateDecision {
            outcome: "entailment".to_owned(),
            selected_probability: entailment,
            margin: entailment - max_non_e,
            requires_human_review: false,
        }
    } else if contradiction >= THRESHOLD && (contradiction - max_non_c) > MIN_MARGIN {
        NliGateDecision {
            outcome: "contradiction".to_owned(),
            selected_probability: contradiction,
            margin: contradiction - max_non_c,
            requires_human_review: false,
        }
    } else {
        let selected = entailment.max(contradiction).max(neutral);
        let mut vals = [entailment, contradiction, neutral];
        vals.sort_by(|a, b| b.total_cmp(a));
        NliGateDecision {
            outcome: "abstain".to_owned(),
            selected_probability: selected,
            margin: vals[0] - vals[1],
            requires_human_review: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimInput {
    pub claim_id: String,
    pub claim_key: String,
    pub value: String,
}

#[must_use]
pub fn count_claim_conflicts(claims: &[ClaimInput]) -> usize {
    let mut by_key: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for c in claims {
        by_key
            .entry(c.claim_key.trim().to_lowercase())
            .or_default()
            .insert(c.value.trim().to_lowercase());
    }
    by_key.values().filter(|vals| vals.len() > 1).count()
}
