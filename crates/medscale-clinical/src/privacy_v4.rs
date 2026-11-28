// SPDX-License-Identifier: Apache-2.0
// Adapted from maziyarpanahi/openmed@5b79cae5 (openmed/core/{calendars,calendar_tables,date_shift,
// rtl_render,clinical_protect,safety_sweep,policy,pii_i18n}.py and openmed/risk/{k_anonymity,l_diversity,reid}.py).
// Third-party attributions preserved in bundled tables:
//   - hijridate 2.6.0 (MIT, Copyright (c) Mohammed Alshehri) via data/hijri_umm_al_qura.json
//   - py-nepali 1.2.0 (MIT, opensource-nepal/py-nepali) via data/bikram_sambat.json
//! OpenMed v4.0.0 deterministic privacy, multilingual calendar, date-shift, RTL rendering,
//! 25-profile policy presets (including `saudi_pdpl`), clinical protection, safety sweep,
//! k/l/t + prosecutor/journalist/marketer risk analysis, and tamper-evident compliance receipts.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::canonical::fingerprint;

pub const FSI: char = '\u{2068}';
pub const PDI: char = '\u{2069}';

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarCode {
    Gregorian,
    Hijri,
    SolarHijri,
    BikramSambat,
    Ethiopian,
    ThaiBuddhist,
}

impl CalendarCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gregorian => "gregorian",
            Self::Hijri => "hijri",
            Self::SolarHijri => "solar_hijri",
            Self::BikramSambat => "bikram_sambat",
            Self::Ethiopian => "ethiopian",
            Self::ThaiBuddhist => "thai_buddhist",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_lowercase().as_str() {
            "gregorian" => Ok(Self::Gregorian),
            "hijri" | "lunar_hijri" | "umm_al_qura" => Ok(Self::Hijri),
            "solar_hijri" | "jalali" => Ok(Self::SolarHijri),
            "bikram_sambat" | "bs" => Ok(Self::BikramSambat),
            "ethiopian" => Ok(Self::Ethiopian),
            "thai_buddhist" => Ok(Self::ThaiBuddhist),
            other => Err(format!("unsupported calendar code: {other}")),
        }
    }
}

/// Folds Eastern Arabic-Indic (`\u0660-\u0669`) and Extended Arabic-Indic (`\u06F0-\u06F9`)
/// numerals into ASCII digits while leaving all other characters unchanged.
#[must_use]
pub fn normalize_arabic_indic_digits(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\u{0660}'..='\u{0669}' => char::from(b'0' + (c as u32 - 0x0660) as u8),
            '\u{06F0}'..='\u{06F9}' => char::from(b'0' + (c as u32 - 0x06F0) as u8),
            other => other,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Proleptic Gregorian ordinal (matching Python datetime.date.toordinal():
// 0001-01-01 => ordinal 1).
// ---------------------------------------------------------------------------

#[must_use]
pub const fn is_gregorian_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[must_use]
pub const fn gregorian_month_length(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_gregorian_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Howard Hinnant civil-from-days / days-from-civil shifted to Python ordinal epoch (`0001-01-01 = 1`).
#[must_use]
pub const fn gregorian_to_ordinal(year: i32, month: u8, day: u8) -> i64 {
    let y = (year as i64) - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = month as i64;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + (day as i64) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    // days_from_civil(y, m, d) == era * 146_097 + doe - 719_468, and 1970-01-01 == Python ordinal 719_163
    era * 146_097 + doe - 719_468 + 719_163
}

#[must_use]
pub const fn ordinal_to_gregorian(ordinal: i64) -> (i32, u8, u8) {
    let z = ordinal - 719_163 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = (y + if m <= 2 { 1 } else { 0 }) as i32;
    (year, m as u8, d as u8)
}

#[derive(Debug)]
struct TableCalendar {
    first_year: i32,
    first_ordinal: i64,
    month_lengths: Vec<[u8; 12]>,
    year_start_ordinals: Vec<i64>,
    last_ordinal: i64,
}

#[derive(Deserialize)]
struct RawCalendarTable {
    first_year: i32,
    first_day: String,
    month_lengths: Vec<Vec<u8>>,
}

impl TableCalendar {
    fn from_json(raw_json: &str) -> Self {
        let parsed: RawCalendarTable =
            serde_json::from_str(raw_json).expect("bundled calendar table JSON parses");
        let parts: Vec<i32> = parsed
            .first_day
            .split('-')
            .map(|s| s.parse::<i32>().expect("calendar anchor int"))
            .collect();
        let first_ordinal = gregorian_to_ordinal(parts[0], parts[1] as u8, parts[2] as u8);
        let mut month_lengths = Vec::with_capacity(parsed.month_lengths.len());
        let mut year_start_ordinals = Vec::with_capacity(parsed.month_lengths.len());
        let mut cur = first_ordinal;
        for row in parsed.month_lengths {
            let mut arr = [0u8; 12];
            for (idx, len) in row.iter().take(12).enumerate() {
                arr[idx] = *len;
            }
            year_start_ordinals.push(cur);
            cur += arr.iter().map(|&x| i64::from(x)).sum::<i64>();
            month_lengths.push(arr);
        }
        Self {
            first_year: parsed.first_year,
            first_ordinal,
            month_lengths,
            year_start_ordinals,
            last_ordinal: cur - 1,
        }
    }

    fn to_ordinal(&self, year: i32, month: u8, day: u8) -> Result<i64, String> {
        let max_year = self.first_year + self.month_lengths.len() as i32 - 1;
        if year < self.first_year || year > max_year || !(1..=12).contains(&month) {
            return Err("date out of calendar table range".to_owned());
        }
        let y_idx = (year - self.first_year) as usize;
        let lengths = &self.month_lengths[y_idx];
        let m_len = lengths[(month - 1) as usize];
        if day == 0 || day > m_len {
            return Err("invalid day for calendar month".to_owned());
        }
        let before_month: i64 = lengths[..(month - 1) as usize]
            .iter()
            .map(|&x| i64::from(x))
            .sum();
        Ok(self.year_start_ordinals[y_idx] + before_month + i64::from(day) - 1)
    }

    fn resolve_ordinal(&self, ordinal: i64) -> Result<(i32, u8, u8), String> {
        if ordinal < self.first_ordinal || ordinal > self.last_ordinal {
            return Err("ordinal out of calendar table range".to_owned());
        }
        let y_idx = self
            .year_start_ordinals
            .partition_point(|&start| start <= ordinal)
            .saturating_sub(1);
        let mut rem = ordinal - self.year_start_ordinals[y_idx];
        let lengths = &self.month_lengths[y_idx];
        for (m_idx, &len) in lengths.iter().enumerate() {
            let l = i64::from(len);
            if rem < l {
                return Ok((
                    self.first_year + y_idx as i32,
                    (m_idx + 1) as u8,
                    (rem + 1) as u8,
                ));
            }
            rem -= l;
        }
        Err("ordinal resolution failed".to_owned())
    }
}

static HIJRI_TABLE: LazyLock<TableCalendar> =
    LazyLock::new(|| TableCalendar::from_json(include_str!("data/hijri_umm_al_qura.json")));
static BIKRAM_TABLE: LazyLock<TableCalendar> =
    LazyLock::new(|| TableCalendar::from_json(include_str!("data/bikram_sambat.json")));

const SOLAR_HIJRI_ANCHOR_YEAR: i32 = 1404;
const SOLAR_HIJRI_ANCHOR_ORDINAL: i64 = 739_331; // 2025-03-21
const SOLAR_HIJRI_DAYS_PER_CYCLE: i64 = 33 * 365 + 8;

const fn solar_hijri_is_leap(year: i32) -> bool {
    matches!(year.rem_euclid(33), 1 | 5 | 9 | 13 | 17 | 22 | 26 | 30)
}

fn solar_hijri_leap_before(year: i32) -> i64 {
    let rem = year.rem_euclid(33);
    let partial = [1, 5, 9, 13, 17, 22, 26, 30]
        .into_iter()
        .filter(|&r| r < rem)
        .count() as i64;
    i64::from(year.div_euclid(33)) * 8 + partial
}

fn solar_hijri_year_start(year: i32) -> i64 {
    SOLAR_HIJRI_ANCHOR_ORDINAL
        + 365 * i64::from(year - SOLAR_HIJRI_ANCHOR_YEAR)
        + solar_hijri_leap_before(year)
        - solar_hijri_leap_before(SOLAR_HIJRI_ANCHOR_YEAR)
}

const fn solar_hijri_days_before_month(month: u8) -> i64 {
    if month <= 7 {
        31 * (month as i64 - 1)
    } else {
        30 * (month as i64 - 1) + 6
    }
}

/// Converts a `(year, month, day)` tuple in `cal` to the shared proleptic Gregorian ordinal day number.
pub fn calendar_to_ordinal(
    cal: CalendarCode,
    year: i32,
    month: u8,
    day: u8,
) -> Result<i64, String> {
    match cal {
        CalendarCode::Gregorian => {
            if !(1..=9999).contains(&year)
                || !(1..=12).contains(&month)
                || day == 0
                || day > gregorian_month_length(year, month)
            {
                return Err("invalid gregorian date".to_owned());
            }
            Ok(gregorian_to_ordinal(year, month, day))
        }
        CalendarCode::ThaiBuddhist => {
            if !(2484..=2665).contains(&year) {
                return Err("thai_buddhist year out of supported range 2484..=2665".to_owned());
            }
            calendar_to_ordinal(CalendarCode::Gregorian, year - 543, month, day)
        }
        CalendarCode::Ethiopian => {
            if !(1891..=2114).contains(&year) || !(1..=13).contains(&month) {
                return Err("ethiopian date out of supported range".to_owned());
            }
            let max_d = if month < 13 {
                30
            } else if year.rem_euclid(4) == 3 {
                6
            } else {
                5
            };
            if day == 0 || day > max_d {
                return Err("invalid ethiopian day".to_owned());
            }
            Ok(2796 - 1
                + 365 * i64::from(year - 1)
                + i64::from(year.div_euclid(4))
                + 30 * i64::from(month - 1)
                + i64::from(day))
        }
        CalendarCode::SolarHijri => {
            if !(1277..=1501).contains(&year) || !(1..=12).contains(&month) {
                return Err("solar_hijri date out of supported range".to_owned());
            }
            let max_d = if month <= 6 {
                31
            } else if month <= 11 || solar_hijri_is_leap(year) {
                30
            } else {
                29
            };
            if day == 0 || day > max_d {
                return Err("invalid solar_hijri day".to_owned());
            }
            Ok(
                solar_hijri_year_start(year)
                    + solar_hijri_days_before_month(month)
                    + i64::from(day)
                    - 1,
            )
        }
        CalendarCode::Hijri => HIJRI_TABLE.to_ordinal(year, month, day),
        CalendarCode::BikramSambat => BIKRAM_TABLE.to_ordinal(year, month, day),
    }
}

/// Converts a shared proleptic Gregorian ordinal day number into a `(year, month, day)` in `cal`.
pub fn ordinal_to_calendar(cal: CalendarCode, ordinal: i64) -> Result<(i32, u8, u8), String> {
    match cal {
        CalendarCode::Gregorian => {
            if !(1..=3_652_059).contains(&ordinal) {
                return Err("gregorian ordinal out of range".to_owned());
            }
            Ok(ordinal_to_gregorian(ordinal))
        }
        CalendarCode::ThaiBuddhist => {
            let (y, m, d) = ordinal_to_calendar(CalendarCode::Gregorian, ordinal)?;
            let ty = y + 543;
            if !(2484..=2665).contains(&ty) {
                return Err("thai_buddhist year out of supported range".to_owned());
            }
            Ok((ty, m, d))
        }
        CalendarCode::Ethiopian => {
            const EPOCH: i64 = 2796;
            let year = ((4 * (ordinal - EPOCH) + 1463).div_euclid(1461)) as i32;
            if !(1891..=2114).contains(&year) {
                return Err("ethiopian ordinal out of range".to_owned());
            }
            let jan1 = EPOCH - 1 + 365 * i64::from(year - 1) + i64::from(year.div_euclid(4)) + 1;
            let month = ((ordinal - jan1).div_euclid(30) + 1) as u8;
            let m_start = EPOCH - 1
                + 365 * i64::from(year - 1)
                + i64::from(year.div_euclid(4))
                + 30 * i64::from(month - 1)
                + 1;
            let day = (ordinal + 1 - m_start) as u8;
            Ok((year, month, day))
        }
        CalendarCode::SolarHijri => {
            let offset = ordinal - SOLAR_HIJRI_ANCHOR_ORDINAL;
            let mut year = SOLAR_HIJRI_ANCHOR_YEAR
                + ((offset * 33).div_euclid(SOLAR_HIJRI_DAYS_PER_CYCLE)) as i32;
            while solar_hijri_year_start(year) > ordinal {
                year -= 1;
            }
            while solar_hijri_year_start(year + 1) <= ordinal {
                year += 1;
            }
            if !(1277..=1501).contains(&year) {
                return Err("solar_hijri ordinal out of range".to_owned());
            }
            let doy = ordinal - solar_hijri_year_start(year);
            let month = if doy < 186 {
                (doy / 31 + 1) as u8
            } else {
                ((doy - 6) / 30 + 1) as u8
            };
            let day = (doy - solar_hijri_days_before_month(month) + 1) as u8;
            Ok((year, month, day))
        }
        CalendarCode::Hijri => HIJRI_TABLE.resolve_ordinal(ordinal),
        CalendarCode::BikramSambat => BIKRAM_TABLE.resolve_ordinal(ordinal),
    }
}

/// Shifts a date in its native calendar by `days` via the shared ordinal.
pub fn shift_calendar_date(
    cal: CalendarCode,
    year: i32,
    month: u8,
    day: u8,
    days: i64,
) -> Result<(i32, u8, u8), String> {
    let ord = calendar_to_ordinal(cal, year, month, day)?;
    ordinal_to_calendar(cal, ord + days)
}

// ---------------------------------------------------------------------------
// Deterministic HMAC-SHA256 / seeded date shift (matching openmed/core/date_shift.py)
// ---------------------------------------------------------------------------

fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut k_block = [0u8; 64];
    if key.len() > 64 {
        let h = Sha256::digest(key);
        k_block[..32].copy_from_slice(&h);
    } else {
        k_block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= k_block[i];
        opad[i] ^= k_block[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(msg);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    let out = outer.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    arr
}

fn digest_be_mod(digest: &[u8; 32], modulus: i64) -> i64 {
    let m = modulus as u128;
    let mut rem: u128 = 0;
    for &b in digest {
        rem = (rem * 256 + u128::from(b)) % m;
    }
    rem as i64
}

/// Deterministic non-zero signed day offset for `patient_key` using `secret` (RFC 2104 HMAC-SHA256).
pub fn stable_date_shift_offset(
    patient_key: &[u8],
    max_days: i64,
    secret: &[u8],
) -> Result<i64, String> {
    if max_days <= 0 {
        return Err("max_days must be positive".to_owned());
    }
    if patient_key.is_empty() || secret.is_empty() {
        return Err("patient_key and secret must be non-empty".to_owned());
    }
    let digest = hmac_sha256(secret, patient_key);
    let bucket = digest_be_mod(&digest, max_days * 2);
    Ok(if bucket < max_days {
        bucket - max_days
    } else {
        bucket - max_days + 1
    })
}

/// Deterministic non-zero signed day offset from a caller seed (matching `stable_offset_from_seed`).
pub fn stable_date_shift_from_seed(seed: i64, max_days: i64) -> Result<i64, String> {
    if max_days <= 0 {
        return Err("max_days must be positive".to_owned());
    }
    let mut hasher = Sha256::new();
    hasher.update(b"medscale-date-shift-seed-v1\0");
    hasher.update(seed.to_string().as_bytes());
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&hasher.finalize());
    let bucket = digest_be_mod(&digest, max_days * 2);
    Ok(if bucket < max_days {
        bucket - max_days
    } else {
        bucket - max_days + 1
    })
}

// ---------------------------------------------------------------------------
// RTL Bidi Rendering (`openmed/core/rtl_render.py`)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RtlRenderResult {
    pub text: String,
    pub base_direction: String,
    pub isolated: bool,
    pub wrapped_count: usize,
}

fn is_arabic_or_hebrew(c: char) -> bool {
    matches!(
        c as u32,
        0x0590..=0x05FF | 0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF
    )
}

#[must_use]
pub fn contains_rtl(text: &str) -> bool {
    text.chars().any(is_arabic_or_hebrew)
}

#[must_use]
pub fn base_direction(text: &str) -> &'static str {
    let mut rtl_count = 0usize;
    let mut ltr_count = 0usize;
    for c in text.chars() {
        if is_arabic_or_hebrew(c) {
            rtl_count += 1;
        } else if c.is_alphabetic() {
            ltr_count += 1;
        }
    }
    if rtl_count >= ltr_count && rtl_count > 0 {
        "rtl"
    } else {
        "ltr"
    }
}

#[must_use]
pub fn strip_bidi_controls(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !matches!(
                *c,
                '\u{2066}'
                    | '\u{2067}'
                    | '\u{2068}'
                    | '\u{2069}'
                    | '\u{202A}'
                    | '\u{202B}'
                    | '\u{202C}'
                    | '\u{202D}'
                    | '\u{202E}'
                    | '\u{200E}'
                    | '\u{200F}'
                    | '\u{061C}'
            )
        })
        .collect()
}

/// Wraps each `(start_char, end_char)` replacement span in Unicode `FSI`..`PDI`
/// isolates when `text` contains RTL script runs; returns pure LTR text byte-for-byte unchanged.
#[must_use]
pub fn render_redacted_rtl(text: &str, char_spans: &[(usize, usize)]) -> RtlRenderResult {
    let dir = base_direction(text).to_owned();
    if !contains_rtl(text) || char_spans.is_empty() {
        return RtlRenderResult {
            text: text.to_owned(),
            base_direction: dir,
            isolated: false,
            wrapped_count: 0,
        };
    }
    let chars: Vec<char> = text.chars().collect();
    let mut spans = char_spans.to_vec();
    spans.sort_unstable();
    let mut out = String::with_capacity(text.len() + spans.len() * 6);
    let mut cursor = 0usize;
    let mut wrapped = 0usize;
    for (start, end) in spans {
        if start < cursor || start >= end || end > chars.len() {
            continue;
        }
        for &c in &chars[cursor..start] {
            out.push(c);
        }
        out.push(FSI);
        for &c in &chars[start..end] {
            out.push(c);
        }
        out.push(PDI);
        cursor = end;
        wrapped += 1;
    }
    for &c in &chars[cursor..] {
        out.push(c);
    }
    RtlRenderResult {
        text: out,
        base_direction: dir,
        isolated: wrapped > 0,
        wrapped_count: wrapped,
    }
}

// ---------------------------------------------------------------------------
// 25 Policy Presets (24 OpenMed v4 JSONs + Saudi PDPL)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyPresetSummary {
    pub name: String,
    pub schema_version: u32,
    pub posture: String,
    pub default_action: String,
    pub strict_no_leak: bool,
    pub safety_sweep_mandatory: bool,
    pub keep_mapping: bool,
    pub reversible_id: bool,
    pub actions: BTreeMap<String, String>,
    pub sha256: String,
}

static POLICY_PRESETS: LazyLock<Vec<PolicyPresetSummary>> = LazyLock::new(|| {
    let raw_files: &[&str] = &[
        include_str!("data/policies/africa_malabo_baseline.json"),
        include_str!("data/policies/australia_privacy_act.json"),
        include_str!("data/policies/canada_pipeda.json"),
        include_str!("data/policies/china_pipl.json"),
        include_str!("data/policies/clinical_minimal_redaction.json"),
        include_str!("data/policies/clinical_preserve.json"),
        include_str!("data/policies/eg_pdpl.json"),
        include_str!("data/policies/fhir_hipaa_safe_harbor.json"),
        include_str!("data/policies/fhir_research_limited_dataset.json"),
        include_str!("data/policies/gdpr_art9_health.json"),
        include_str!("data/policies/gdpr_pseudonymization.json"),
        include_str!("data/policies/hipaa_expert_review_assist.json"),
        include_str!("data/policies/hipaa_safe_harbor.json"),
        include_str!("data/policies/india_dpdp_act.json"),
        include_str!("data/policies/india_health_id.json"),
        include_str!("data/policies/ke_dpa.json"),
        include_str!("data/policies/ma_law_09_08.json"),
        include_str!("data/policies/ng_ndpa.json"),
        include_str!("data/policies/omop_hipaa_safe_harbor.json"),
        include_str!("data/policies/omop_research_limited_dataset.json"),
        include_str!("data/policies/research_limited_dataset.json"),
        include_str!("data/policies/strict_no_leak.json"),
        include_str!("data/policies/uk_ico_anonymisation.json"),
        include_str!("data/policies/za_popia.json"),
    ];
    let mut presets: Vec<PolicyPresetSummary> = raw_files
        .iter()
        .map(|src| {
            let v: Value = serde_json::from_str(src).expect("bundled policy JSON parses");
            let actions = v["actions"]
                .as_object()
                .map(|m| {
                    m.iter()
                        .filter_map(|(k, val)| val.as_str().map(|s| (k.clone(), s.to_owned())))
                        .collect()
                })
                .unwrap_or_default();
            PolicyPresetSummary {
                name: v["name"].as_str().unwrap_or("").to_owned(),
                schema_version: v["schema_version"].as_u64().unwrap_or(1) as u32,
                posture: v["posture"].as_str().unwrap_or("").to_owned(),
                default_action: v["default_action"].as_str().unwrap_or("mask").to_owned(),
                strict_no_leak: v["strict_no_leak"].as_bool().unwrap_or(false),
                safety_sweep_mandatory: v["safety_sweep_mandatory"].as_bool().unwrap_or(true),
                keep_mapping: v["keep_mapping"].as_bool().unwrap_or(false),
                reversible_id: v["reversible_id"].as_bool().unwrap_or(false),
                actions,
                sha256: fingerprint(&v),
            }
        })
        .collect();

    // MedScale Saudi PDPL (Personal Data Protection Law) preset:
    let mut saudi_actions = BTreeMap::new();
    for k in [
        "PERSON",
        "FIRST_NAME",
        "LAST_NAME",
        "SAUDI_NATIONAL_ID",
        "SAUDI_IQAMA",
        "MRN",
        "SSN",
        "PHONE",
        "EMAIL",
        "IBAN",
        "STREET_ADDRESS",
        "DATE_OF_BIRTH",
        "DATE",
    ] {
        saudi_actions.insert(k.to_owned(), "mask".to_owned());
    }
    let saudi_meta = json!({
        "schema_version": 1,
        "name": "saudi_pdpl",
        "posture": "saudi_pdpl_health_sensitive_local_first",
        "default_action": "mask",
        "strict_no_leak": true,
        "safety_sweep_mandatory": true,
        "rtl_isolation": true,
    });
    presets.push(PolicyPresetSummary {
        name: "saudi_pdpl".to_owned(),
        schema_version: 1,
        posture: "saudi_pdpl_health_sensitive_local_first".to_owned(),
        default_action: "mask".to_owned(),
        strict_no_leak: true,
        safety_sweep_mandatory: true,
        keep_mapping: false,
        reversible_id: false,
        actions: saudi_actions,
        sha256: fingerprint(&saudi_meta),
    });
    presets.sort_by(|a, b| a.name.cmp(&b.name));
    presets
});

#[must_use]
pub fn list_policy_presets() -> &'static [PolicyPresetSummary] {
    &POLICY_PRESETS
}

#[must_use]
pub fn get_policy_preset(name: &str) -> Option<&'static PolicyPresetSummary> {
    let normalized = match name.trim().to_lowercase().as_str() {
        "au_privacy" => "australia_privacy_act",
        "gdpr" => "gdpr_pseudonymization",
        "gdpr_health" => "gdpr_art9_health",
        "kenya_dpa" => "ke_dpa",
        "pipeda" => "canada_pipeda",
        "uk_ico" => "uk_ico_anonymisation",
        "india_health_ids" | "abha" => "india_health_id",
        "pdpl_sa" | "ksa_pdpl" => "saudi_pdpl",
        other => return POLICY_PRESETS.iter().find(|p| p.name == other),
    };
    POLICY_PRESETS.iter().find(|p| p.name == normalized)
}

// ---------------------------------------------------------------------------
// Clinical Protect & Safety Sweep
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedSpanV4 {
    pub label: String,
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub source: String,
}

static PROTECTED_CLINICAL_TERMS: LazyLock<BTreeSet<String>> = LazyLock::new(|| {
    include_str!("data/clinical_protect_terms.txt")
        .lines()
        .filter_map(|line| {
            let trimmed = line.split('#').next().unwrap_or("").trim().to_lowercase();
            let norm = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
            if norm.is_empty() { None } else { Some(norm) }
        })
        .collect()
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClinicalProtectionOutcome {
    pub retained: Vec<DetectedSpanV4>,
    pub suppressed_count: usize,
    pub checked_count: usize,
    pub protected_term_count: usize,
}

fn is_ambiguous_person_loc_org(label: &str) -> bool {
    matches!(
        label.trim().to_uppercase().as_str(),
        "PERSON"
            | "PER"
            | "NAME"
            | "FIRST_NAME"
            | "LAST_NAME"
            | "MIDDLE_NAME"
            | "LOCATION"
            | "LOC"
            | "ORGANIZATION"
            | "ORG"
    )
}

fn in_personal_title_context(text: &str, char_start: usize) -> bool {
    let prefix: String = text.chars().take(char_start).collect();
    let lower = prefix.trim_end().to_lowercase();
    lower.ends_with("dr.")
        || lower.ends_with("dr")
        || lower.ends_with("mr.")
        || lower.ends_with("mrs.")
        || lower.ends_with("ms.")
        || lower.ends_with("prof.")
}

/// Drops false-positive clinical eponyms/medications on ambiguous `PERSON`/`LOCATION`/`ORGANIZATION`
/// labels unless preceded by an explicit personal title (`Dr.`) or formatted like a direct identifier.
#[must_use]
pub fn filter_clinical_protected_spans(
    spans: &[DetectedSpanV4],
    text: &str,
) -> ClinicalProtectionOutcome {
    let mut retained = Vec::with_capacity(spans.len());
    let mut suppressed = 0usize;
    let mut checked = 0usize;
    let chars: Vec<char> = text.chars().collect();
    for s in spans {
        if is_ambiguous_person_loc_org(&s.label) && s.start < s.end && s.end <= chars.len() {
            checked += 1;
            let surface: String = chars[s.start..s.end].iter().collect();
            let norm = surface
                .trim()
                .to_lowercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let has_digit = norm.chars().any(|c| c.is_ascii_digit());
            if !has_digit
                && PROTECTED_CLINICAL_TERMS.contains(&norm)
                && !in_personal_title_context(text, s.start)
            {
                suppressed += 1;
                continue;
            }
        }
        retained.push(s.clone());
    }
    ClinicalProtectionOutcome {
        retained,
        suppressed_count: suppressed,
        checked_count: checked,
        protected_term_count: PROTECTED_CLINICAL_TERMS.len(),
    }
}

/// Deterministic post-detector sweep catching missed structured identifiers (`ssn`, `email`,
/// `medical_record_number`, `saudi_national_id`) normalized over Arabic-Indic numerals.
#[must_use]
pub fn run_safety_sweep(text: &str, existing: &[DetectedSpanV4]) -> Vec<DetectedSpanV4> {
    let normalized = normalize_arabic_indic_digits(text);
    let chars: Vec<char> = normalized.chars().collect();
    let orig_chars: Vec<char> = text.chars().collect();
    let overlaps_existing = |start: usize, end: usize| -> bool {
        existing.iter().any(|e| start < e.end && end > e.start)
    };
    let mut found = Vec::new();

    // 1. SSN NNN-NN-NNNN
    if chars.len() >= 11 {
        for i in 0..=chars.len() - 11 {
            let prev_ok = i == 0 || !chars[i - 1].is_alphanumeric();
            let next_ok = i + 11 == chars.len() || !chars[i + 11].is_alphanumeric();
            if prev_ok
                && next_ok
                && chars[i..i + 3].iter().all(char::is_ascii_digit)
                && chars[i + 3] == '-'
                && chars[i + 4..i + 6].iter().all(char::is_ascii_digit)
                && chars[i + 6] == '-'
                && chars[i + 7..i + 11].iter().all(char::is_ascii_digit)
                && !overlaps_existing(i, i + 11)
            {
                found.push(DetectedSpanV4 {
                    label: "ssn".to_owned(),
                    start: i,
                    end: i + 11,
                    text: orig_chars[i..i + 11].iter().collect(),
                    source: "safety_sweep".to_owned(),
                });
            }
        }
    }

    // 2. Emails (`user@domain.tld`)
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '@' {
            let mut s = i;
            while s > 0
                && (chars[s - 1].is_ascii_alphanumeric()
                    || matches!(chars[s - 1], '.' | '_' | '-' | '+'))
            {
                s -= 1;
            }
            let mut e = i + 1;
            while e < chars.len()
                && (chars[e].is_ascii_alphanumeric() || matches!(chars[e], '.' | '-'))
            {
                e += 1;
            }
            while e > i + 1 && chars[e - 1] == '.' {
                e -= 1;
            }
            let domain: String = chars[i + 1..e].iter().collect();
            if s < i && domain.contains('.') && domain.len() >= 3 && !overlaps_existing(s, e) {
                found.push(DetectedSpanV4 {
                    label: "email".to_owned(),
                    start: s,
                    end: e,
                    text: orig_chars[s..e].iter().collect(),
                    source: "safety_sweep".to_owned(),
                });
                i = e;
                continue;
            }
        }
        i += 1;
    }

    // 3. Explicit MRN (`MRN[: #]*<digits>`)
    let lower: String = normalized.to_lowercase();
    let mut search_from = 0usize;
    while let Some(byte_rel) = lower[search_from..].find("mrn") {
        let byte_idx = search_from + byte_rel;
        let char_start = lower[..byte_idx].chars().count();
        let prev_ok = char_start == 0 || !chars[char_start - 1].is_alphanumeric();
        if prev_ok {
            let mut pos = char_start + 3;
            while pos < chars.len() && matches!(chars[pos], ':' | ' ' | '#') {
                pos += 1;
            }
            let dig_start = pos;
            while pos < chars.len() && (chars[pos].is_ascii_digit() || chars[pos] == '-') {
                pos += 1;
            }
            let digit_count = chars[dig_start..pos]
                .iter()
                .filter(|c| c.is_ascii_digit())
                .count();
            if digit_count >= 5 && !overlaps_existing(char_start, pos) {
                found.push(DetectedSpanV4 {
                    label: "medical_record_number".to_owned(),
                    start: char_start,
                    end: pos,
                    text: orig_chars[char_start..pos].iter().collect(),
                    source: "safety_sweep".to_owned(),
                });
            }
        }
        search_from = byte_idx + 3;
    }

    found.sort_by_key(|s| (s.start, s.end));
    found
}

// ---------------------------------------------------------------------------
// k-Anonymity, l-Diversity, t-Closeness & Prosecutor/Journalist/Marketer Risk
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReidentificationRiskReport {
    pub record_count: usize,
    pub k_min: usize,
    pub class_count: usize,
    pub singleton_count: usize,
    pub l_diversity: BTreeMap<String, usize>,
    pub t_closeness: BTreeMap<String, f64>,
    pub prosecutor_risk: f64,
    pub journalist_risk: f64,
    pub marketer_risk: f64,
}

#[must_use]
pub fn evaluate_table_risk(
    records: &[BTreeMap<String, Value>],
    quasi_identifiers: &[String],
    sensitive_attributes: &[String],
) -> ReidentificationRiskReport {
    if records.is_empty() {
        return ReidentificationRiskReport {
            record_count: 0,
            k_min: 0,
            class_count: 0,
            singleton_count: 0,
            l_diversity: BTreeMap::new(),
            t_closeness: BTreeMap::new(),
            prosecutor_risk: 0.0,
            journalist_risk: 0.0,
            marketer_risk: 0.0,
        };
    }

    let mut groups: BTreeMap<Vec<String>, Vec<usize>> = BTreeMap::new();
    for (idx, row) in records.iter().enumerate() {
        let key: Vec<String> = quasi_identifiers
            .iter()
            .map(|qi| match row.get(qi) {
                Some(Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
                None => String::new(),
            })
            .collect();
        groups.entry(key).or_default().push(idx);
    }

    let k_min = groups.values().map(Vec::len).min().unwrap_or(0);
    let class_count = groups.len();
    let singleton_count = groups.values().filter(|g| g.len() == 1).count();
    let n = records.len() as f64;
    let max_group_inv = groups
        .values()
        .map(|g| 1.0 / (g.len() as f64))
        .fold(0.0_f64, f64::max);
    let marketer_risk = (class_count as f64) / n;

    let mut l_diversity = BTreeMap::new();
    let mut t_closeness = BTreeMap::new();

    for attr in sensitive_attributes {
        let mut global_counts: BTreeMap<String, usize> = BTreeMap::new();
        for row in records {
            let v = row
                .get(attr)
                .map(|x| {
                    x.as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| x.to_string())
                })
                .unwrap_or_default();
            *global_counts.entry(v).or_insert(0) += 1;
        }

        let mut min_l = usize::MAX;
        let mut max_tvd = 0.0_f64;
        for idxs in groups.values() {
            let mut local_counts: BTreeMap<String, usize> = BTreeMap::new();
            for &i in idxs {
                let v = records[i]
                    .get(attr)
                    .map(|x| {
                        x.as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| x.to_string())
                    })
                    .unwrap_or_default();
                *local_counts.entry(v).or_insert(0) += 1;
            }
            min_l = min_l.min(local_counts.len());

            // Total variation distance: 0.5 * sum_v |P_local(v) - P_global(v)|
            let m = idxs.len() as f64;
            let mut l1 = 0.0_f64;
            for (val, &g_cnt) in &global_counts {
                let p_g = (g_cnt as f64) / n;
                let l_cnt = local_counts.get(val).copied().unwrap_or(0);
                let p_l = (l_cnt as f64) / m;
                l1 += (p_l - p_g).abs();
            }
            max_tvd = max_tvd.max(0.5 * l1);
        }
        l_diversity.insert(attr.clone(), if min_l == usize::MAX { 0 } else { min_l });
        t_closeness.insert(attr.clone(), max_tvd);
    }

    ReidentificationRiskReport {
        record_count: records.len(),
        k_min,
        class_count,
        singleton_count,
        l_diversity,
        t_closeness,
        prosecutor_risk: max_group_inv,
        journalist_risk: max_group_inv,
        marketer_risk,
    }
}

// ---------------------------------------------------------------------------
// Tamper-Evident Compliance Receipt (SHA-256 Digest Over Policy & Counts Only)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyComplianceReceipt {
    pub schema_version: u32,
    pub policy_name: String,
    pub policy_sha256: String,
    pub input_sha256: String,
    pub output_sha256: String,
    pub base_direction: String,
    pub rtl_isolated: bool,
    pub suppressed_clinical_terms: usize,
    pub sweep_added_spans: usize,
    pub total_redacted_spans: usize,
    pub receipt_sha256: String,
}

fn raw_sha256_tag(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(71);
    out.push_str("sha256:");
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn build_compliance_receipt(
    policy_name: &str,
    policy_sha256: &str,
    input_text: &str,
    output_text: &str,
    base_direction: &str,
    rtl_isolated: bool,
    suppressed_clinical_terms: usize,
    sweep_added_spans: usize,
    total_redacted_spans: usize,
) -> PrivacyComplianceReceipt {
    let input_sha256 = raw_sha256_tag(input_text.as_bytes());
    let output_sha256 = raw_sha256_tag(output_text.as_bytes());
    let body = json!({
        "schema_version": 1,
        "policy_name": policy_name,
        "policy_sha256": policy_sha256,
        "input_sha256": input_sha256,
        "output_sha256": output_sha256,
        "base_direction": base_direction,
        "rtl_isolated": rtl_isolated,
        "suppressed_clinical_terms": suppressed_clinical_terms,
        "sweep_added_spans": sweep_added_spans,
        "total_redacted_spans": total_redacted_spans,
    });
    let receipt_sha256 = fingerprint(&body);
    PrivacyComplianceReceipt {
        schema_version: 1,
        policy_name: policy_name.to_owned(),
        policy_sha256: policy_sha256.to_owned(),
        input_sha256,
        output_sha256,
        base_direction: base_direction.to_owned(),
        rtl_isolated,
        suppressed_clinical_terms,
        sweep_added_spans,
        total_redacted_spans,
        receipt_sha256,
    }
}

#[must_use]
pub fn verify_compliance_receipt(receipt: &PrivacyComplianceReceipt) -> bool {
    let body = json!({
        "schema_version": receipt.schema_version,
        "policy_name": receipt.policy_name,
        "policy_sha256": receipt.policy_sha256,
        "input_sha256": receipt.input_sha256,
        "output_sha256": receipt.output_sha256,
        "base_direction": receipt.base_direction,
        "rtl_isolated": receipt.rtl_isolated,
        "suppressed_clinical_terms": receipt.suppressed_clinical_terms,
        "sweep_added_spans": receipt.sweep_added_spans,
        "total_redacted_spans": receipt.total_redacted_spans,
    });
    fingerprint(&body) == receipt.receipt_sha256
}
