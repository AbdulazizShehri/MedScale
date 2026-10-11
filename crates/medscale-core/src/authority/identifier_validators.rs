//! Structured-identifier validators, ported from OpenMed v3.0.0
//! `openmed/core/anonymizer/providers/clinical_ids.py` (`maziyarpanahi/openmed@ea920f36`,
//! Apache-2.0): checksum and structure rules for US SSN, US phone, Luhn
//! numbers (cards), US NPI, IBAN (ISO 13616 mod-97 with per-country length),
//! SWIFT/BIC, UK NHS number (modulus 11), UK NINO, Canadian SIN (Luhn),
//! Australian Medicare (weighted mod 10) and Australian TFN (weighted mod 11).
//!
//! Validators decide whether a candidate string is a structurally valid
//! identifier; they never decide that a value belongs to a real person.

/// ISO 13616 IBAN lengths per country (OpenMed `_IBAN_LENGTHS`).
const IBAN_LENGTHS: &[(&str, usize)] = &[
    ("AD", 24),
    ("AE", 23),
    ("AL", 28),
    ("AT", 20),
    ("AZ", 28),
    ("BA", 20),
    ("BE", 16),
    ("BG", 22),
    ("BH", 22),
    ("BR", 29),
    ("BY", 28),
    ("CH", 21),
    ("CR", 22),
    ("CY", 28),
    ("CZ", 24),
    ("DE", 22),
    ("DK", 18),
    ("DO", 28),
    ("EE", 20),
    ("EG", 29),
    ("ES", 24),
    ("FI", 18),
    ("FO", 18),
    ("FR", 27),
    ("GB", 22),
    ("GE", 22),
    ("GI", 23),
    ("GL", 18),
    ("GR", 27),
    ("GT", 28),
    ("HR", 21),
    ("HU", 28),
    ("IE", 22),
    ("IL", 23),
    ("IS", 26),
    ("IT", 27),
    ("JO", 30),
    ("KW", 30),
    ("KZ", 20),
    ("LB", 28),
    ("LC", 32),
    ("LI", 21),
    ("LT", 20),
    ("LU", 20),
    ("LV", 21),
    ("MC", 27),
    ("MD", 24),
    ("ME", 22),
    ("MK", 19),
    ("MR", 27),
    ("MT", 31),
    ("MU", 30),
    ("NL", 18),
    ("NO", 15),
    ("PK", 24),
    ("PL", 28),
    ("PS", 29),
    ("PT", 25),
    ("QA", 29),
    ("RO", 24),
    ("RS", 22),
    ("SA", 24),
    ("SC", 31),
    ("SE", 24),
    ("SI", 19),
    ("SK", 24),
    ("SM", 27),
    ("TN", 24),
    ("TR", 26),
    ("UA", 29),
    ("VA", 22),
    ("VG", 24),
    ("XK", 20),
];

/// Country codes accepted in a BIC (OpenMed `_BIC_COUNTRY_CODES`).
const BIC_COUNTRY_CODES: &[&str] = &[
    "AD", "AE", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AZ", "BA", "BB",
    "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BM", "BN", "BO", "BR", "BS", "BT", "BV", "BW", "BY",
    "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK", "CL", "CM", "CN", "CO", "CR", "CU", "CV",
    "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM", "DO", "DZ", "EE", "EG", "ER", "ES", "ET", "FI",
    "FJ", "FM", "FO", "FR", "GA", "GB", "GD", "GE", "GF", "GH", "GI", "GL", "GM", "GN", "GP", "GQ",
    "GR", "GT", "GW", "GY", "HK", "HM", "HR", "HU", "ID", "IE", "IL", "IN", "IO", "IQ", "IR", "IS",
    "IT", "JM", "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN", "KP", "KR", "KW", "KZ", "LA", "LB",
    "LC", "LI", "LK", "LR", "LS", "LT", "LU", "LV", "LY", "MA", "MC", "MD", "ME", "MG", "MH", "MK",
    "ML", "MM", "MN", "MO", "MP", "MQ", "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ", "NA",
    "NC", "NE", "NF", "NG", "NI", "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG",
    "PH", "PK", "PL", "PM", "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW",
    "SA", "SB", "SC", "SD", "SE", "SG", "SH", "SI", "SK", "SL", "SM", "SN", "SO", "SR", "SS", "ST",
    "SV", "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TJ", "TK", "TM", "TN", "TO", "TR", "TT", "TV",
    "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI", "VN", "VU", "WF",
    "WS", "XK", "YE", "YT", "ZA", "ZM", "ZW",
];

fn digits_only(text: &str) -> Vec<u32> {
    text.chars().filter_map(|c| c.to_digit(10)).collect()
}

fn luhn_check_digit(digits: &[u32]) -> u32 {
    let total: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| {
            if i.is_multiple_of(2) {
                let doubled = d * 2;
                if doubled > 9 { doubled - 9 } else { doubled }
            } else {
                d
            }
        })
        .sum();
    (10 - total % 10) % 10
}

/// Removes whitespace (and hyphens when `hyphens`) and upper-cases ASCII.
fn compact(text: &str, hyphens: bool) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace() && !(hyphens && *c == '-'))
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

fn all_ascii(s: &str, pred: fn(&u8) -> bool) -> bool {
    s.bytes().all(|b| pred(&b))
}

/// US Social Security number: 9 digits; no 000/666/9xx area, 00 group or 0000 serial.
#[must_use]
#[allow(dead_code)] // Ported for parity; the recognizer already flags these shapes.
pub fn validate_ssn(text: &str) -> bool {
    let d = digits_only(text);
    if d.len() != 9 {
        return false;
    }
    let area = &d[0..3];
    if area == [0, 0, 0] || area == [6, 6, 6] || area[0] == 9 {
        return false;
    }
    d[3..5] != [0, 0] && d[5..9] != [0, 0, 0, 0]
}

/// US phone: 10 digits (or 11 with a leading 1); area and exchange rules.
#[must_use]
#[allow(dead_code)] // Ported for parity; the recognizer already flags these shapes.
pub fn validate_phone_us(text: &str) -> bool {
    let mut d = digits_only(text);
    if d.len() == 11 && d[0] == 1 {
        d.remove(0);
    }
    d.len() == 10 && d[0] > 1 && d[3] != 0
}

/// Luhn checksum over at least 13 digits (payment cards and similar).
#[must_use]
pub fn validate_luhn(text: &str) -> bool {
    let d = digits_only(text);
    d.len() >= 13 && luhn_check_digit(&d[..d.len() - 1]) == d[d.len() - 1]
}

/// US National Provider Identifier: Luhn over `80840` + the first 9 digits.
#[must_use]
pub fn validate_npi(text: &str) -> bool {
    let d = digits_only(text);
    if d.len() != 10 {
        return false;
    }
    let mut prefixed = vec![8, 0, 8, 4, 0];
    prefixed.extend_from_slice(&d[..9]);
    luhn_check_digit(&prefixed) == d[9]
}

fn mod97(value: &str) -> u32 {
    let mut remainder = 0u32;
    for c in value.chars() {
        let digits = if let Some(d) = c.to_digit(10) {
            d.to_string()
        } else {
            (c as u32 - 55).to_string()
        };
        for digit in digits.chars().filter_map(|x| x.to_digit(10)) {
            remainder = (remainder * 10 + digit) % 97;
        }
    }
    remainder
}

/// IBAN: structure, per-country length and ISO 13616 mod-97 = 1.
#[must_use]
pub fn validate_iban(text: &str) -> bool {
    let c = compact(text, true);
    let b = c.as_bytes();
    if !c.is_ascii()
        || !(15..=34).contains(&c.len())
        || !b[..2].iter().all(u8::is_ascii_uppercase)
        || !b[2..4].iter().all(u8::is_ascii_digit)
        || !b[4..]
            .iter()
            .all(|x| x.is_ascii_uppercase() || x.is_ascii_digit())
    {
        return false;
    }
    if IBAN_LENGTHS
        .iter()
        .find(|(cc, _)| *cc == &c[..2])
        .is_some_and(|(_, length)| c.len() != *length)
    {
        return false;
    }
    let rearranged = format!("{}{}", &c[4..], &c[..4]);
    mod97(&rearranged) == 1
}

/// SWIFT/BIC: 8 or 11 characters with a known country code.
#[must_use]
pub fn validate_bic(text: &str) -> bool {
    let c = compact(text, false);
    if !c.is_ascii() || (c.len() != 8 && c.len() != 11) {
        return false;
    }
    let ok = all_ascii(&c[..6], u8::is_ascii_uppercase)
        && all_ascii(&c[6..], |x| x.is_ascii_uppercase() || x.is_ascii_digit());
    ok && BIC_COUNTRY_CODES.contains(&&c[4..6])
}

/// UK NHS number: 10 digits with the modulus-11 check digit.
#[must_use]
pub fn validate_uk_nhs_number(text: &str) -> bool {
    let d = digits_only(text);
    if d.len() != 10 {
        return false;
    }
    let total: u32 = (2..=10).rev().zip(&d[..9]).map(|(w, v)| w * v).sum();
    let check = match 11 - total % 11 {
        11 => 0,
        10 => return false,
        c => c,
    };
    d[9] == check
}

/// UK National Insurance number: two letters, six digits, A-D; prefix rules.
#[must_use]
pub fn validate_uk_nino(text: &str) -> bool {
    let c = compact(text, true);
    let b = c.as_bytes();
    if !c.is_ascii()
        || c.len() != 9
        || !b[..2].iter().all(u8::is_ascii_uppercase)
        || !b[2..8].iter().all(u8::is_ascii_digit)
        || !matches!(b[8], b'A'..=b'D')
    {
        return false;
    }
    let prefix = &c[..2];
    !["BG", "GB", "KN", "NK", "NT", "TN", "ZZ"].contains(&prefix)
        && !"DFIQUV".contains(char::from(b[0]))
        && !"DFIQUVO".contains(char::from(b[1]))
}

/// `NNNNNNNNN` or `NNN?NNN?NNN` with one repeated separator (space or hyphen).
fn grouped_nine(text: &str) -> bool {
    let t = text.trim();
    let b = t.as_bytes();
    match b.len() {
        9 => b.iter().all(u8::is_ascii_digit),
        11 => {
            let sep = b[3];
            (sep == b' ' || sep == b'-')
                && b[7] == sep
                && b.iter()
                    .enumerate()
                    .all(|(i, x)| i == 3 || i == 7 || x.is_ascii_digit())
        }
        _ => false,
    }
}

/// Canadian Social Insurance number: 9 digits, Luhn, not starting with 0.
#[must_use]
pub fn validate_canadian_sin(text: &str) -> bool {
    if !grouped_nine(text) {
        return false;
    }
    let d = digits_only(text);
    d[0] != 0 && luhn_check_digit(&d[..8]) == d[8]
}

/// Australian Medicare card: leading 2-6, 10 digits (optionally 4-5-1),
/// optional IRN; weighted mod-10 check on the 9th digit.
#[must_use]
pub fn validate_australian_medicare(text: &str) -> bool {
    let t = text.trim();
    if !t.is_ascii() {
        return false;
    }
    let (card, rest) = if t.len() >= 12 && t.as_bytes()[4] == b' ' && t.as_bytes()[10] == b' ' {
        (&t[..12], &t[12..])
    } else if t.len() >= 10 {
        (&t[..10], &t[10..])
    } else {
        return false;
    };
    let card_ok = if card.len() == 12 {
        card.bytes().enumerate().all(|(i, x)| {
            if i == 4 || i == 10 {
                x == b' '
            } else {
                x.is_ascii_digit()
            }
        })
    } else {
        card.bytes().all(|x| x.is_ascii_digit())
    };
    if !card_ok || !matches!(card.as_bytes()[0], b'2'..=b'6') {
        return false;
    }
    // Optional IRN: `/` (with optional spaces), one space or hyphen, or nothing, then 1-9.
    let irn = rest.trim_start_matches(' ');
    let irn_ok = if rest.is_empty() {
        true
    } else {
        let tail = if let Some(r) = irn.strip_prefix('/') {
            r.trim_start_matches(' ')
        } else if rest.len() == 2 && (rest.starts_with(' ') || rest.starts_with('-')) {
            &rest[1..]
        } else {
            rest
        };
        tail.len() == 1 && matches!(tail.as_bytes()[0], b'1'..=b'9')
    };
    if !irn_ok {
        return false;
    }
    let d = digits_only(card);
    let total: u32 = [1, 3, 7, 9, 1, 3, 7, 9]
        .iter()
        .zip(&d[..8])
        .map(|(w, v)| w * v)
        .sum();
    total % 10 == d[8]
}

/// Australian Tax File number: 9 digits (optionally 3-3-3 spaced), weighted mod 11.
#[must_use]
pub fn validate_australian_tfn(text: &str) -> bool {
    let t = text.trim();
    let b = t.as_bytes();
    let shape = match b.len() {
        9 => b.iter().all(u8::is_ascii_digit),
        11 => {
            b[3] == b' '
                && b[7] == b' '
                && b.iter()
                    .enumerate()
                    .all(|(i, x)| i == 3 || i == 7 || x.is_ascii_digit())
        }
        _ => false,
    };
    if !shape {
        return false;
    }
    let d = digits_only(t);
    if d.iter().all(|x| *x == 0) {
        return false;
    }
    let total: u32 = [1, 4, 3, 7, 5, 8, 6, 9, 10]
        .iter()
        .zip(&d)
        .map(|(w, v)| w * v)
        .sum();
    total.is_multiple_of(11)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Expected values computed with a line-by-line Python mirror of the
    /// OpenMed reference functions.
    #[test]
    fn validators_agree_with_the_openmed_reference() {
        assert!(validate_ssn("123-45-6789"), "ssn 123-45-6789");
        assert!(validate_ssn("078-05-1120"), "ssn 078-05-1120");
        assert!(!validate_ssn("000-12-3456"), "ssn 000-12-3456");
        assert!(!validate_ssn("666-12-3456"), "ssn 666-12-3456");
        assert!(!validate_ssn("912-34-5678"), "ssn 912-34-5678");
        assert!(!validate_ssn("123-00-4567"), "ssn 123-00-4567");
        assert!(!validate_ssn("123-45-0000"), "ssn 123-45-0000");
        assert!(!validate_ssn("12-345-678"), "ssn 12-345-678");
        assert_eq!(
            validate_phone_us("(415) 555-2671"),
            true,
            "phone_us (415) 555-2671"
        );
        assert_eq!(
            validate_phone_us("+1 415 555 2671"),
            true,
            "phone_us +1 415 555 2671"
        );
        assert_eq!(
            validate_phone_us("015-555-2671"),
            false,
            "phone_us 015-555-2671"
        );
        assert_eq!(
            validate_phone_us("415-055-2671"),
            false,
            "phone_us 415-055-2671"
        );
        assert!(!validate_phone_us("555-2671"), "phone_us 555-2671");
        assert_eq!(
            validate_phone_us("12-415-555-2671"),
            false,
            "phone_us 12-415-555-2671"
        );
        assert_eq!(
            validate_luhn("4111 1111 1111 1111"),
            true,
            "luhn 4111 1111 1111 1111"
        );
        assert_eq!(
            validate_luhn("4111-1111-1111-1112"),
            false,
            "luhn 4111-1111-1111-1112"
        );
        assert!(!validate_luhn("79927398713"), "luhn 79927398713");
        assert_eq!(
            validate_luhn("5500 0000 0000 0004"),
            true,
            "luhn 5500 0000 0000 0004"
        );
        assert_eq!(
            validate_luhn("1234567812345670"),
            true,
            "luhn 1234567812345670"
        );
        assert!(validate_npi("1234567893"), "npi 1234567893");
        assert!(!validate_npi("1234567890"), "npi 1234567890");
        assert!(!validate_npi("12345678"), "npi 12345678");
        assert!(validate_npi("1245319599"), "npi 1245319599");
        assert_eq!(
            validate_iban("GB82 WEST 1234 5698 7654 32"),
            true,
            "iban GB82 WEST 1234 5698 7654 32"
        );
        assert_eq!(
            validate_iban("DE89 3704 0044 0532 0130 00"),
            true,
            "iban DE89 3704 0044 0532 0130 00"
        );
        assert_eq!(
            validate_iban("GB82 WEST 1234 5698 7654 33"),
            false,
            "iban GB82 WEST 1234 5698 7654 33"
        );
        assert_eq!(
            validate_iban("DE89 3704 0044 0532 0130"),
            false,
            "iban DE89 3704 0044 0532 0130"
        );
        assert_eq!(
            validate_iban("XX12345678901234"),
            false,
            "iban XX12345678901234"
        );
        assert!(validate_bic("DEUTDEFF"), "bic DEUTDEFF");
        assert!(validate_bic("DEUTDEFF500"), "bic DEUTDEFF500");
        assert!(validate_bic("deut de ff"), "bic deut de ff");
        assert!(!validate_bic("DEUTXXFF"), "bic DEUTXXFF");
        assert!(!validate_bic("DEUT1EFF"), "bic DEUT1EFF");
        assert!(!validate_bic("DEUTDEF"), "bic DEUTDEF");
        assert_eq!(
            validate_uk_nhs_number("943 476 5919"),
            true,
            "uk_nhs_number 943 476 5919"
        );
        assert_eq!(
            validate_uk_nhs_number("943 476 5918"),
            false,
            "uk_nhs_number 943 476 5918"
        );
        assert_eq!(
            validate_uk_nhs_number("401 023 2137"),
            true,
            "uk_nhs_number 401 023 2137"
        );
        assert_eq!(
            validate_uk_nhs_number("12345"),
            false,
            "uk_nhs_number 12345"
        );
        assert!(validate_uk_nino("AB123456C"), "uk_nino AB123456C");
        assert_eq!(
            validate_uk_nino("ab 12 34 56 c"),
            true,
            "uk_nino ab 12 34 56 c"
        );
        assert!(!validate_uk_nino("BG123456C"), "uk_nino BG123456C");
        assert!(!validate_uk_nino("DA123456C"), "uk_nino DA123456C");
        assert!(!validate_uk_nino("AO123456C"), "uk_nino AO123456C");
        assert!(!validate_uk_nino("AB123456E"), "uk_nino AB123456E");
        assert_eq!(
            validate_canadian_sin("130 692 544"),
            true,
            "canadian_sin 130 692 544"
        );
        assert_eq!(
            validate_canadian_sin("130-692-544"),
            true,
            "canadian_sin 130-692-544"
        );
        assert_eq!(
            validate_canadian_sin("130692545"),
            false,
            "canadian_sin 130692545"
        );
        assert_eq!(
            validate_canadian_sin("046 454 286"),
            false,
            "canadian_sin 046 454 286"
        );
        assert_eq!(
            validate_canadian_sin("130 692-544"),
            false,
            "canadian_sin 130 692-544"
        );
        assert_eq!(
            validate_australian_medicare("2123 45670 1"),
            true,
            "australian_medicare 2123 45670 1"
        );
        assert_eq!(
            validate_australian_medicare("2123456701"),
            true,
            "australian_medicare 2123456701"
        );
        assert_eq!(
            validate_australian_medicare("2123 45670 1 2"),
            true,
            "australian_medicare 2123 45670 1 2"
        );
        assert_eq!(
            validate_australian_medicare("2123 45671 1"),
            false,
            "australian_medicare 2123 45671 1"
        );
        assert_eq!(
            validate_australian_medicare("7123456701"),
            false,
            "australian_medicare 7123456701"
        );
        assert_eq!(
            validate_australian_tfn("123 456 782"),
            true,
            "australian_tfn 123 456 782"
        );
        assert_eq!(
            validate_australian_tfn("123456782"),
            true,
            "australian_tfn 123456782"
        );
        assert_eq!(
            validate_australian_tfn("123 456 783"),
            false,
            "australian_tfn 123 456 783"
        );
        assert_eq!(
            validate_australian_tfn("000000000"),
            false,
            "australian_tfn 000000000"
        );
        assert_eq!(
            validate_australian_tfn("12345678"),
            false,
            "australian_tfn 12345678"
        );
    }

    #[test]
    fn non_ascii_input_is_rejected_without_panicking() {
        for f in [
            validate_iban,
            validate_bic,
            validate_uk_nino,
            validate_australian_medicare,
            validate_australian_tfn,
            validate_canadian_sin,
        ] {
            assert!(!f("ÉÉ12 3456 7890 1234 5"));
            assert!(!f("２１２３４５６７０１"));
        }
    }

    #[test]
    fn tables_match_the_reference_sizes() {
        assert_eq!(IBAN_LENGTHS.len(), 73);
        assert_eq!(BIC_COUNTRY_CODES.len(), 231);
    }
}
