//! Medical time, effect states, and placement classes (Spec 002 + Spec 019 Q06).

use serde::{Deserialize, Serialize};

/// Precision of a medical/recorded time value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimePrecision {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    Instant,
    Unknown,
}

impl TimePrecision {
    /// Rank for upgrade checks: higher means finer chronological claim.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::Year => 1,
            Self::Month => 2,
            Self::Day => 3,
            Self::Hour => 4,
            Self::Minute => 5,
            Self::Second => 6,
            Self::Instant => 7,
        }
    }
}

/// Role of a MedicalTime relative to a clinical or custody event (Spec 019).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeRole {
    Effective,
    Recorded,
    Acquired,
    ValidStart,
    ValidEnd,
}

/// Structured time with explicit precision. Never invent Instant from coarser input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MedicalTime {
    /// Partial or full timestamp string matching `precision` (logical; validated by helpers).
    pub value: String,
    pub precision: TimePrecision,
    pub approximate: bool,
    /// Optional offset from UTC in minutes (east positive). Absent = unspecified, not UTC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone_offset_minutes: Option<i16>,
}

impl MedicalTime {
    /// Construct without inventing timezone or Instant precision.
    #[must_use]
    pub fn new(value: impl Into<String>, precision: TimePrecision, approximate: bool) -> Self {
        Self {
            value: value.into(),
            precision,
            approximate,
            timezone_offset_minutes: None,
        }
    }

    /// Parse/validate a MedicalTime that already declares precision.
    ///
    /// Refuses values whose lexical shape implies finer precision than declared,
    /// and refuses claiming Instant/Second/… when the value only carries year/month/day.
    pub fn parse_validated(
        value: impl Into<String>,
        precision: TimePrecision,
        approximate: bool,
        timezone_offset_minutes: Option<i16>,
    ) -> Result<Self, MedicalTimeError> {
        let value = value.into();
        if let Some(offset) = timezone_offset_minutes
            && !(-14 * 60..=14 * 60).contains(&offset)
        {
            return Err(MedicalTimeError::InvalidTimezoneOffset { offset });
        }
        validate_value_matches_precision(&value, precision)?;
        Ok(Self {
            value,
            precision,
            approximate,
            timezone_offset_minutes,
        })
    }

    /// Refuse upgrading chronological precision (e.g. Year → Instant).
    pub fn try_with_precision(self, target: TimePrecision) -> Result<Self, MedicalTimeError> {
        if target.rank() > self.precision.rank() {
            return Err(MedicalTimeError::FalsePrecisionUpgrade {
                from: self.precision,
                to: target,
            });
        }
        validate_value_matches_precision(&self.value, target)?;
        Ok(Self {
            precision: target,
            ..self
        })
    }

    /// Inclusive lexical bounds for ordering without collapsing to a false Instant.
    ///
    /// Year-only values sort as a calendar-year range, not as `YYYY-01-01T00:00:00Z` Instant.
    #[must_use]
    pub fn precision_bounds(&self) -> (String, String) {
        match self.precision {
            TimePrecision::Unknown => ("~".to_owned(), "~".to_owned()),
            TimePrecision::Year => {
                let y = year_prefix(&self.value);
                (format!("{y}-01-01"), format!("{y}-12-31"))
            }
            TimePrecision::Month => {
                let ym = month_prefix(&self.value);
                let end_day = days_in_month_prefix(&ym);
                (format!("{ym}-01"), format!("{ym}-{end_day:02}"))
            }
            TimePrecision::Day => {
                let d = day_prefix(&self.value);
                (d.clone(), d)
            }
            TimePrecision::Hour
            | TimePrecision::Minute
            | TimePrecision::Second
            | TimePrecision::Instant => (self.value.clone(), self.value.clone()),
        }
    }

    /// Deterministic timeline sort key: start|precision_rank|end|value|tz.
    /// Does not treat Unknown/partial as Instant.
    #[must_use]
    pub fn timeline_sort_key(&self) -> String {
        let (start, end) = self.precision_bounds();
        let tz = self
            .timezone_offset_minutes
            .map(|o| o.to_string())
            .unwrap_or_else(|| "_".to_owned());
        format!(
            "{start}|p{:02}|{end}|{}|{tz}",
            self.precision.rank(),
            self.value
        )
    }
}

/// Errors from MedicalTime parse / precision helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MedicalTimeError {
    FalsePrecisionUpgrade {
        from: TimePrecision,
        to: TimePrecision,
    },
    ValuePrecisionMismatch {
        value: String,
        precision: TimePrecision,
    },
    InvalidTimezoneOffset {
        offset: i16,
    },
}

impl std::fmt::Display for MedicalTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FalsePrecisionUpgrade { from, to } => {
                write!(
                    f,
                    "refusing false MedicalTime precision upgrade {from:?} -> {to:?}"
                )
            }
            Self::ValuePrecisionMismatch { value, precision } => {
                write!(
                    f,
                    "value {value:?} does not match declared precision {precision:?}"
                )
            }
            Self::InvalidTimezoneOffset { offset } => {
                write!(f, "timezone_offset_minutes {offset} out of range")
            }
        }
    }
}

impl std::error::Error for MedicalTimeError {}

fn year_prefix(value: &str) -> String {
    value.chars().take(4).collect()
}

fn month_prefix(value: &str) -> String {
    let chars: String = value.chars().take(7).collect();
    if chars.len() >= 7 {
        chars
    } else {
        format!("{}-01", year_prefix(value))
    }
}

fn day_prefix(value: &str) -> String {
    let chars: String = value.chars().take(10).collect();
    if chars.len() >= 10 {
        chars
    } else {
        format!("{}-01", month_prefix(value))
    }
}

fn days_in_month_prefix(ym: &str) -> u8 {
    let parts: Vec<_> = ym.split('-').collect();
    if parts.len() < 2 {
        return 31;
    }
    let year: i32 = parts[0].parse().unwrap_or(1970);
    let month: u8 = parts[1].parse().unwrap_or(1);
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

fn validate_value_matches_precision(
    value: &str,
    precision: TimePrecision,
) -> Result<(), MedicalTimeError> {
    let ok = match precision {
        TimePrecision::Unknown => !value.is_empty(),
        TimePrecision::Year => value.len() == 4 && value.chars().all(|c| c.is_ascii_digit()),
        TimePrecision::Month => {
            value.len() == 7
                && value.as_bytes().get(4) == Some(&b'-')
                && value[..4].chars().all(|c| c.is_ascii_digit())
                && value[5..7].chars().all(|c| c.is_ascii_digit())
        }
        TimePrecision::Day => {
            value.len() == 10
                && value.as_bytes().get(4) == Some(&b'-')
                && value.as_bytes().get(7) == Some(&b'-')
                && value[..4].chars().all(|c| c.is_ascii_digit())
                && value[5..7].chars().all(|c| c.is_ascii_digit())
                && value[8..10].chars().all(|c| c.is_ascii_digit())
        }
        TimePrecision::Hour => value.len() >= 13 && value.contains('T'),
        TimePrecision::Minute => value.len() >= 16 && value.contains('T'),
        TimePrecision::Second | TimePrecision::Instant => value.len() >= 19 && value.contains('T'),
    };
    if ok {
        Ok(())
    } else {
        Err(MedicalTimeError::ValuePrecisionMismatch {
            value: value.to_owned(),
            precision,
        })
    }
}

/// External-action / effect state machine vocabulary (Spec 014 hardens durability).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectState {
    Pending,
    Sent,
    Confirmed,
    Failed,
    Unknown,
}

/// OSS matrix placement class for native/FFI components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementClass {
    P0,
    P1,
    P2,
    P3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn year_precision_preserved_and_bounds_are_range() {
        let t = MedicalTime::parse_validated("2020", TimePrecision::Year, false, None).unwrap();
        assert_eq!(t.precision, TimePrecision::Year);
        let (start, end) = t.precision_bounds();
        assert_eq!(start, "2020-01-01");
        assert_eq!(end, "2020-12-31");
        assert!(t.timeline_sort_key().contains("|p01|"));
    }

    #[test]
    fn refuse_year_to_instant_upgrade() {
        let t = MedicalTime::new("2020", TimePrecision::Year, false);
        let err = t.try_with_precision(TimePrecision::Instant).unwrap_err();
        assert!(matches!(
            err,
            MedicalTimeError::FalsePrecisionUpgrade {
                from: TimePrecision::Year,
                to: TimePrecision::Instant
            }
        ));
    }

    #[test]
    fn timezone_offset_roundtrip() {
        let t = MedicalTime::parse_validated("2020-06-15", TimePrecision::Day, false, Some(180))
            .unwrap();
        let json = serde_json::to_value(&t).unwrap();
        let back: MedicalTime = serde_json::from_value(json).unwrap();
        assert_eq!(back.timezone_offset_minutes, Some(180));
    }
}
