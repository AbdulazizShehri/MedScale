//! Privacy Gate recognizers and transform application (Spec 079).
//!
//! Pure functions: no I/O, no network, no storage. The local-model
//! recognizer lives in `privacy_gate` because it needs the admitted Pack
//! store; everything here is deterministic.
//!
//! Recognition is deliberately simple and auditable (hand-written scanners,
//! no regex dependency). It can miss sensitive values; every receipt says so
//! (`ReceiptLimitation::AutomatedRecognitionIsIncomplete`).

use medscale_contracts::privacy_gate::{
    PSEUDONYM_HEX_CHARS, PSEUDONYM_PREFIX, RecognizerFamily, RecognizerIdentity, SensitiveSpan,
    SensitiveSpanKind, TransformOp, is_pseudonym,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const PATTERN_RECOGNIZER_ID: &str = "medscale.privacy.pattern";
pub const PATTERN_RECOGNIZER_VERSION: &str = "1";
pub const FHIR_RECOGNIZER_ID: &str = "medscale.privacy.fhir";
pub const FHIR_RECOGNIZER_VERSION: &str = "1";
pub const MODEL_RECOGNIZER_ID: &str = "medscale.privacy.local_model";
pub const MODEL_RECOGNIZER_VERSION: &str = "1";

#[must_use]
pub fn pattern_identity() -> RecognizerIdentity {
    RecognizerIdentity {
        recognizer_id: PATTERN_RECOGNIZER_ID.to_owned(),
        version: PATTERN_RECOGNIZER_VERSION.to_owned(),
        family: RecognizerFamily::Deterministic,
        model_pack_id: None,
    }
}

#[must_use]
pub fn fhir_identity() -> RecognizerIdentity {
    RecognizerIdentity {
        recognizer_id: FHIR_RECOGNIZER_ID.to_owned(),
        version: FHIR_RECOGNIZER_VERSION.to_owned(),
        family: RecognizerFamily::StructuredFhir,
        model_pack_id: None,
    }
}

// ---------------------------------------------------------------------------
// Character helpers
// ---------------------------------------------------------------------------

fn is_delimiter(c: char) -> bool {
    matches!(
        c,
        '"' | '\'' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | '\n' | '\r' | '\t'
    )
}

/// Byte-indexed char iterator helpers over `text`.
fn chars_from(text: &str, start: usize) -> impl Iterator<Item = (usize, char)> + '_ {
    text[start..]
        .char_indices()
        .map(move |(i, c)| (start + i, c))
}

fn prev_char(text: &str, pos: usize) -> Option<char> {
    text[..pos].chars().next_back()
}

fn span(kind: SensitiveSpanKind, start: usize, end: usize) -> SensitiveSpan {
    SensitiveSpan {
        kind,
        start,
        end,
        recognizer_id: PATTERN_RECOGNIZER_ID.to_owned(),
    }
}

/// Trims trailing sentence punctuation from a candidate end.
fn trim_trailing_punct(text: &str, start: usize, mut end: usize) -> usize {
    while end > start {
        match prev_char(text, end) {
            Some(c @ ('.' | ':' | '!' | '?')) => end -= c.len_utf8(),
            _ => break,
        }
    }
    end
}

// ---------------------------------------------------------------------------
// Word tokenization (for names, dates, addresses, labels)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Word<'a> {
    start: usize,
    end: usize,
    text: &'a str,
}

/// Splits on whitespace and delimiters; keeps internal `.`, `-`, `'`, `/`,
/// `:`, `@`, `#` so tokens like `12.03.2024` and `Dr.` stay intact.
fn words(text: &str) -> Vec<Word<'_>> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        let boundary = c.is_whitespace() || is_delimiter(c);
        match (boundary, start) {
            (true, Some(s)) => {
                out.push(Word {
                    start: s,
                    end: i,
                    text: &text[s..i],
                });
                start = None;
            }
            (false, None) => start = Some(i),
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push(Word {
            start: s,
            end: text.len(),
            text: &text[s..],
        });
    }
    out
}

fn strip_punct(word: &str) -> &str {
    word.trim_end_matches(['.', ':', ',', ';', '!', '?'])
}

fn is_capitalized_name_word(word: &str) -> bool {
    let w = strip_punct(word);
    let mut chars = w.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_uppercase() {
        return false;
    }
    let rest: Vec<char> = chars.collect();
    if rest.is_empty() {
        return false;
    }
    // "PERSON_NAME" style placeholders and acronyms are not names.
    if rest.iter().all(|c| c.is_uppercase() || *c == '_') {
        return false;
    }
    rest.iter()
        .all(|c| c.is_alphabetic() || *c == '-' || *c == '\'')
}

const NAME_PARTICLES: &[&str] = &[
    "de", "del", "della", "der", "den", "van", "von", "la", "le", "du", "da", "dos", "y",
];

const HONORIFICS: &[&str] = &[
    "mr", "mrs", "ms", "miss", "dr", "prof", "sr", "sra", "srta", "don", "doña", "dña", "m", "mme",
    "mlle", "herr", "frau", "dra", "pr",
];

const NAME_LABELS: &[&str] = &[
    "patient",
    "name",
    "patient name",
    "paciente",
    "nombre",
    "patiente",
    "nom",
    "patientin",
    "physician",
    "doctor",
    "médico",
    "médica",
    "médecin",
    "arzt",
    "ärztin",
    "contact",
    "emergency contact",
    "contacto",
    "personne à contacter",
    "angehörige",
    "angehöriger",
    "signed",
    "firmado",
    "signé",
    "unterschrieben",
];

/// Consumes capitalized name words (and inner particles) starting at `i`.
/// Returns the index one past the last name word, or `i` when none.
fn take_name_words(ws: &[Word<'_>], i: usize, text: &str) -> usize {
    let mut j = i;
    let mut last_name = i;
    while j < ws.len() && j - i < 5 {
        let w = ws[j].text;
        // Stop at a line break or delimiter between words.
        if j > i
            && text[ws[j - 1].end..ws[j].start]
                .chars()
                .any(|c| c == '\n' || is_delimiter(c))
        {
            break;
        }
        if is_capitalized_name_word(w) {
            j += 1;
            last_name = j;
            if w.ends_with(['.', ',', ';', ':']) && !is_initial(w) {
                break;
            }
        } else if NAME_PARTICLES.contains(&w.to_lowercase().as_str()) && j > i {
            j += 1;
        } else if is_initial(w) {
            j += 1;
            last_name = j;
        } else {
            break;
        }
    }
    last_name
}

fn is_initial(word: &str) -> bool {
    let mut chars = word.chars();
    matches!((chars.next(), chars.next(), chars.next()), (Some(c), Some('.'), None) if c.is_uppercase())
}

fn scan_names(text: &str, ws: &[Word<'_>], out: &mut Vec<SensitiveSpan>) {
    let mut i = 0;
    while i < ws.len() {
        let lower = strip_punct(ws[i].text).to_lowercase();
        // Honorific + name.
        if HONORIFICS.contains(&lower.as_str()) && i + 1 < ws.len() {
            let end = take_name_words(ws, i + 1, text);
            if end > i + 1 {
                let s = ws[i + 1].start;
                let e = trim_trailing_punct(text, s, ws[end - 1].end);
                out.push(span(SensitiveSpanKind::PersonName, s, e));
                i = end;
                continue;
            }
        }
        // "Label:" + name (one- or two-word labels).
        for label_len in [2_usize, 1] {
            if i + label_len > ws.len() {
                continue;
            }
            let label_end_word = &ws[i + label_len - 1];
            if !label_end_word.text.ends_with(':') {
                continue;
            }
            let label = text[ws[i].start..label_end_word.end]
                .trim_end_matches(':')
                .to_lowercase();
            if !NAME_LABELS.contains(&label.as_str()) {
                continue;
            }
            let mut first = i + label_len;
            // Allow an honorific right after the label.
            if first < ws.len()
                && HONORIFICS.contains(&strip_punct(ws[first].text).to_lowercase().as_str())
            {
                first += 1;
            }
            let end = take_name_words(ws, first, text);
            if end > first {
                let s = ws[first].start;
                let e = trim_trailing_punct(text, s, ws[end - 1].end);
                out.push(span(SensitiveSpanKind::PersonName, s, e));
            }
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Dates
// ---------------------------------------------------------------------------

const MONTHS: &[&str] = &[
    // English
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
    "jan",
    "feb",
    "mar",
    "apr",
    "jun",
    "jul",
    "aug",
    "sep",
    "sept",
    "oct",
    "nov",
    "dec", // Spanish
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "setiembre",
    "octubre",
    "noviembre",
    "diciembre", // French
    "janvier",
    "février",
    "fevrier",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "aout",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
    "decembre", // German
    "januar",
    "februar",
    "märz",
    "maerz",
    "juni",
    "juli",
    "oktober",
    "dezember",
];

fn is_month(word: &str) -> bool {
    MONTHS.contains(&strip_punct(word).to_lowercase().as_str())
}

fn digits_only(word: &str) -> Option<u32> {
    let w = strip_punct(word);
    if w.is_empty() || w.len() > 4 || !w.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    w.parse().ok()
}

fn is_day(word: &str) -> bool {
    let w = word.trim_end_matches(['.', ',']);
    let w = w
        .trim_end_matches("st")
        .trim_end_matches("nd")
        .trim_end_matches("rd")
        .trim_end_matches("th")
        .trim_end_matches("er");
    matches!(digits_only(w), Some(d) if (1..=31).contains(&d)) && w.len() <= 2
}

fn is_year(word: &str) -> bool {
    matches!(digits_only(word), Some(y) if (1800..=2199).contains(&y))
        && strip_punct(word).len() == 4
}

/// Numeric dates: `yyyy-mm-dd`, `dd/mm/yyyy`, `dd.mm.yyyy`, `dd-mm-yyyy`,
/// `mm/dd/yyyy` (same shape), with 2- or 4-digit years for separators `/`/`.`.
fn is_numeric_date(word: &str) -> bool {
    let w = strip_punct(word);
    for sep in ['-', '/', '.'] {
        let parts: Vec<&str> = w.split(sep).collect();
        if parts.len() != 3
            || !parts
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        {
            continue;
        }
        let lens: Vec<usize> = parts.iter().map(|p| p.len()).collect();
        let n: Vec<u32> = parts.iter().map(|p| p.parse().unwrap_or(0)).collect();
        if lens == [4, 2, 2] && (1..=12).contains(&n[1]) && (1..=31).contains(&n[2]) {
            return true;
        }
        if (lens[0] <= 2 && lens[1] <= 2)
            && (lens[2] == 4 || (lens[2] == 2 && sep != '-'))
            && (1..=31).contains(&n[0])
            && (1..=31).contains(&n[1])
            && (n[0] <= 12 || n[1] <= 12)
        {
            return true;
        }
    }
    false
}

fn scan_dates(text: &str, ws: &[Word<'_>], out: &mut Vec<SensitiveSpan>) {
    let mut i = 0;
    while i < ws.len() {
        if is_numeric_date(ws[i].text) {
            let e = trim_trailing_punct(text, ws[i].start, ws[i].end);
            out.push(span(SensitiveSpanKind::Date, ws[i].start, e));
            i += 1;
            continue;
        }
        if is_month(ws[i].text) {
            // Look back: [day][.]? [de]?  MONTH
            let mut start = ws[i].start;
            let mut matched = false;
            if i >= 1 && is_day(ws[i - 1].text) {
                start = ws[i - 1].start;
                matched = true;
            } else if i >= 2 && ws[i - 1].text.eq_ignore_ascii_case("de") && is_day(ws[i - 2].text)
            {
                start = ws[i - 2].start;
                matched = true;
            }
            // Look forward: [de|,]? YEAR  or  DAY[,]? YEAR
            let mut end = ws[i].end;
            let mut j = i + 1;
            if !matched && j < ws.len() && is_day(ws[j].text) {
                matched = true;
                end = ws[j].end;
                j += 1;
            }
            if j < ws.len() && ws[j].text.eq_ignore_ascii_case("de") {
                j += 1;
            }
            if j < ws.len() && is_year(ws[j].text) {
                end = ws[j].end;
                matched = true;
                j += 1;
            }
            if matched && (end > ws[i].end || start < ws[i].start) {
                let e = trim_trailing_punct(text, start, end);
                out.push(span(SensitiveSpanKind::Date, start, e));
                i = j;
                continue;
            }
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Email, URL, IP, phone
// ---------------------------------------------------------------------------

fn scan_emails(text: &str, out: &mut Vec<SensitiveSpan>) {
    let local_ok = |c: char| c.is_ascii_alphanumeric() || "._%+-".contains(c);
    let domain_ok = |c: char| c.is_ascii_alphanumeric() || c == '.' || c == '-';
    for (at, _) in text.match_indices('@') {
        let mut start = at;
        while let Some(c) = prev_char(text, start) {
            if local_ok(c) {
                start -= c.len_utf8();
            } else {
                break;
            }
        }
        let mut end = at + 1;
        for (i, c) in chars_from(text, at + 1) {
            if domain_ok(c) {
                end = i + c.len_utf8();
            } else {
                break;
            }
        }
        let end = trim_trailing_punct(text, at + 1, end);
        let domain = &text[at + 1..end];
        let tld_ok = domain
            .rsplit('.')
            .next()
            .is_some_and(|t| t.len() >= 2 && t.chars().all(|c| c.is_ascii_alphabetic()));
        if start < at && domain.contains('.') && tld_ok {
            out.push(span(SensitiveSpanKind::Email, start, end));
        }
    }
}

fn scan_urls(text: &str, out: &mut Vec<SensitiveSpan>) {
    let lower = text.to_ascii_lowercase();
    for prefix in ["https://", "http://", "www."] {
        for (start, _) in lower.match_indices(prefix) {
            if prefix == "www." && lower[..start].ends_with("//") {
                continue;
            }
            let mut end = start;
            for (i, c) in chars_from(text, start) {
                if c.is_whitespace() || is_delimiter(c) {
                    break;
                }
                end = i + c.len_utf8();
            }
            let end = trim_trailing_punct(text, start, end);
            if end > start + prefix.len() {
                out.push(span(SensitiveSpanKind::Url, start, end));
            }
        }
    }
}

fn scan_ipv4(ws: &[Word<'_>], out: &mut Vec<SensitiveSpan>) {
    for w in ws {
        let t = strip_punct(w.text);
        let parts: Vec<&str> = t.split('.').collect();
        if parts.len() == 4
            && parts.iter().all(|p| {
                !p.is_empty()
                    && p.len() <= 3
                    && p.bytes().all(|b| b.is_ascii_digit())
                    && p.parse::<u16>().is_ok_and(|n| n <= 255)
            })
        {
            out.push(span(
                SensitiveSpanKind::IpAddress,
                w.start,
                w.start + t.len(),
            ));
        }
    }
}

fn scan_phones(text: &str, out: &mut Vec<SensitiveSpan>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let starts = c == b'+' || c.is_ascii_digit() || c == b'(';
        let boundary_ok = i == 0
            || !(bytes[i - 1].is_ascii_alphanumeric()
                || bytes[i - 1] == b'.'
                || bytes[i - 1] == b'/'
                || bytes[i - 1] == b'-');
        if !(starts && boundary_ok) {
            i += 1;
            continue;
        }
        let mut j = i;
        let mut digits = 0;
        let mut separators = 0;
        while j < bytes.len() {
            let b = bytes[j];
            if b.is_ascii_digit() {
                digits += 1;
            } else if matches!(b, b' ' | b'-' | b'.' | b'(' | b')') {
                // A separator must be followed by a digit or '(' to continue.
                let next = bytes.get(j + 1).copied();
                if !matches!(next, Some(n) if n.is_ascii_digit() || n == b'(' || (b == b')' && n == b' '))
                {
                    break;
                }
                separators += 1;
            } else if b == b'+' && j == i {
            } else {
                break;
            }
            j += 1;
        }
        let end_ok = j == bytes.len() || !bytes[j].is_ascii_alphanumeric();
        let candidate = &text[i..j];
        let plus = c == b'+';
        if end_ok
            && (9..=15).contains(&digits)
            && (plus || separators >= 1)
            && !is_numeric_date(candidate)
            && !standalone_identifier(candidate.trim())
            && !candidate.contains("..")
        {
            let end = candidate.trim_end_matches([' ', '.', '-', '(']).len() + i;
            out.push(span(SensitiveSpanKind::Phone, i, end));
            i = j;
        } else {
            i += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Identifiers, postal codes, addresses
// ---------------------------------------------------------------------------

const ID_LABELS: &[&str] = &[
    "mrn",
    "id",
    "patient id",
    "record",
    "record no",
    "nhs",
    "nhs number",
    "ssn",
    "dni",
    "nie",
    "nss",
    "nhc",
    "historia",
    "ipp",
    "insee",
    "versichertennummer",
    "patientennummer",
    "fallnummer",
    "no",
    "nº",
    "number",
    "número",
    "numéro",
    "nummer",
];

fn has_digit(s: &str) -> bool {
    s.bytes().any(|b| b.is_ascii_digit())
}

fn id_like(token: &str) -> bool {
    let t = strip_punct(token);
    t.len() >= 4
        && t.bytes().filter(u8::is_ascii_digit).count() >= 4
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '/')
        && !is_pseudonym(t)
        && !is_numeric_date(t)
}

/// `AB-123456`, `MRN004512`, `123-45-6789`: letters (0-4) + optional dash +
/// at least 5 digits, or the 3-2-4 digit pattern.
fn standalone_identifier(token: &str) -> bool {
    let t = strip_punct(token);
    if is_pseudonym(t) || is_numeric_date(t) {
        return false;
    }
    let parts: Vec<&str> = t.split('-').collect();
    if parts.len() == 3
        && parts.iter().map(|p| p.len()).collect::<Vec<_>>() == [3, 2, 4]
        && parts.iter().all(|p| p.bytes().all(|b| b.is_ascii_digit()))
    {
        return true;
    }
    let letters = t.bytes().take_while(u8::is_ascii_uppercase).count();
    if letters > 4 {
        return false;
    }
    let rest = t[letters..].trim_start_matches('-');
    rest.len() >= 5 && rest.bytes().all(|b| b.is_ascii_digit()) && (letters > 0 || rest.len() >= 8)
}

fn scan_identifiers(text: &str, ws: &[Word<'_>], out: &mut Vec<SensitiveSpan>) {
    for i in 0..ws.len() {
        // Labelled: "MRN: 004512", "Patient ID 77-4411", "DNI 12345678Z".
        for label_len in [2_usize, 1] {
            if i + label_len >= ws.len() {
                continue;
            }
            let label = text[ws[i].start..ws[i + label_len - 1].end]
                .trim_end_matches([':', '#', '.'])
                .to_lowercase();
            if !ID_LABELS.contains(&label.as_str()) {
                continue;
            }
            let mut k = i + label_len;
            if k < ws.len() && (ws[k].text == "#" || ws[k].text == ":") {
                k += 1;
            }
            if k < ws.len() {
                let token = ws[k].text.trim_start_matches(['#', ':']);
                let s = ws[k].end - token.len();
                if id_like(token) {
                    let e = s + strip_punct(token).len();
                    out.push(span(SensitiveSpanKind::Identifier, s, e));
                }
            }
        }
        if standalone_identifier(ws[i].text) {
            let t = strip_punct(ws[i].text);
            out.push(span(
                SensitiveSpanKind::Identifier,
                ws[i].start,
                ws[i].start + t.len(),
            ));
        }
    }
}

type Validator = fn(&str) -> bool;

/// Checksum validators that need an adjacent label, because their shapes
/// (8-11 letters, 9-10 plain digits) are too common to flag on their own.
const LABELLED_CHECKSUMS: &[(&str, Validator)] = &[
    ("npi", super::identifier_validators::validate_npi),
    ("bic", super::identifier_validators::validate_bic),
    ("swift", super::identifier_validators::validate_bic),
    ("sin", super::identifier_validators::validate_canadian_sin),
    (
        "medicare",
        super::identifier_validators::validate_australian_medicare,
    ),
    ("tfn", super::identifier_validators::validate_australian_tfn),
];

/// `NNN NNN NNNN`, the printed grouping of a UK NHS number.
fn nhs_grouping(candidate: &str) -> bool {
    let parts: Vec<&str> = candidate.split(' ').collect();
    parts.iter().map(|p| p.len()).collect::<Vec<_>>() == [3, 3, 4]
        && parts.iter().all(|p| p.bytes().all(|b| b.is_ascii_digit()))
}

/// Structured identifiers confirmed by checksum (OpenMed validators, see
/// `identifier_validators`), over windows of up to nine words so printed
/// groupings ("4111 1111 1111 1111", "GB82 WEST 1234 …") are found.
fn scan_checksum_identifiers(text: &str, ws: &[Word<'_>], out: &mut Vec<SensitiveSpan>) {
    use super::identifier_validators::{
        validate_iban, validate_luhn, validate_uk_nhs_number, validate_uk_nino,
    };
    for i in 0..ws.len() {
        let label = if i > 0 {
            strip_punct(ws[i - 1].text).to_lowercase()
        } else {
            String::new()
        };
        for j in i..(i + 9).min(ws.len()) {
            let start = ws[i].start;
            let end = trim_trailing_punct(text, start, ws[j].end);
            if end <= start {
                continue;
            }
            let candidate = &text[start..end];
            let card = candidate
                .bytes()
                .all(|b| b.is_ascii_digit() || b == b' ' || b == b'-')
                && (13..=19).contains(&candidate.bytes().filter(u8::is_ascii_digit).count())
                && validate_luhn(candidate);
            let unlabelled = card
                || validate_iban(candidate)
                || validate_uk_nino(candidate)
                || (nhs_grouping(candidate) && validate_uk_nhs_number(candidate));
            let labelled = LABELLED_CHECKSUMS
                .iter()
                .any(|(name, valid)| label == *name && valid(candidate));
            if unlabelled || labelled {
                out.push(span(SensitiveSpanKind::Identifier, start, end));
            }
        }
    }
}

const POSTAL_LABELS: &[&str] = &[
    "zip",
    "postcode",
    "postal code",
    "cp",
    "plz",
    "code postal",
    "c.p.",
];

/// UK-style postcode: `SW1A 1AA`, `M1 1AE`, `EC1A 1BB`.
fn uk_postcode(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = strip_punct(b).as_bytes();
    let outward_ok = (2..=4).contains(&a.len())
        && a[0].is_ascii_uppercase()
        && a.iter()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && a.iter().any(u8::is_ascii_digit);
    let inward_ok = b.len() == 3
        && b[0].is_ascii_digit()
        && b[1].is_ascii_uppercase()
        && b[2].is_ascii_uppercase();
    outward_ok && inward_ok
}

fn five_digits(word: &str) -> bool {
    let w = strip_punct(word);
    w.len() == 5 && w.bytes().all(|b| b.is_ascii_digit())
}

fn scan_postal_codes(text: &str, ws: &[Word<'_>], out: &mut Vec<SensitiveSpan>) {
    for i in 0..ws.len() {
        if i + 1 < ws.len() && uk_postcode(ws[i].text, ws[i + 1].text) {
            let e = ws[i + 1].start + strip_punct(ws[i + 1].text).len();
            out.push(span(SensitiveSpanKind::PostalCode, ws[i].start, e));
            continue;
        }
        if !five_digits(ws[i].text) {
            continue;
        }
        let labelled = (1..=2).any(|n| {
            i >= n
                && POSTAL_LABELS.contains(
                    &text[ws[i - n].start..ws[i - 1].end]
                        .trim_end_matches(':')
                        .to_lowercase()
                        .as_str(),
                )
        });
        // "…, 28013 Madrid" / "75002 Paris" / "10115 Berlin": a 5-digit code
        // after a comma or line start and followed by a capitalized place.
        let after_break = i == 0
            || text[ws[i - 1].end..ws[i].start]
                .chars()
                .any(|c| c == ',' || c == '\n');
        let place_follows = i + 1 < ws.len() && is_capitalized_name_word(ws[i + 1].text);
        if labelled || (after_break && place_follows) {
            let e = ws[i].start + strip_punct(ws[i].text).len();
            out.push(span(SensitiveSpanKind::PostalCode, ws[i].start, e));
        }
    }
}

const STREET_WORDS: &[&str] = &[
    "street",
    "st",
    "avenue",
    "ave",
    "road",
    "rd",
    "lane",
    "ln",
    "drive",
    "dr",
    "boulevard",
    "blvd",
    "court",
    "way",
    "place",
    "calle",
    "avenida",
    "avda",
    "paseo",
    "plaza",
    "camino",
    "rue",
    "boulevard",
    "allée",
    "chemin",
    "place",
    "impasse",
    "straße",
    "strasse",
    "str",
    "weg",
    "platz",
    "gasse",
    "allee",
    "ring",
];

fn is_street_word(word: &str) -> bool {
    let w = strip_punct(word).to_lowercase();
    STREET_WORDS.contains(&w.as_str())
        || w.ends_with("straße")
        || w.ends_with("strasse")
        || w.ends_with("weg")
        || w.ends_with("platz")
}

/// An address is the delimiter-bounded segment (between commas, quotes,
/// line breaks) that contains a street word and a house number.
fn scan_addresses(text: &str, ws: &[Word<'_>], out: &mut Vec<SensitiveSpan>) {
    for w in ws {
        if !is_street_word(w.text) {
            continue;
        }
        let mut seg_start = w.start;
        while let Some(c) = prev_char(text, seg_start) {
            if c == ',' || c == '\n' || c == '"' || c == ';' || c == ':' || c == '(' {
                break;
            }
            seg_start -= c.len_utf8();
        }
        let mut seg_end = w.end;
        for (i, c) in chars_from(text, w.end) {
            if c == ',' || c == '\n' || c == '"' || c == ';' || c == ')' {
                break;
            }
            seg_end = i + c.len_utf8();
        }
        let raw = &text[seg_start..seg_end];
        let lead = raw.len() - raw.trim_start().len();
        let s = seg_start + lead;
        let e = trim_trailing_punct(text, s, s + raw.trim().len());
        let segment = &text[s..e];
        let word_count = segment.split_whitespace().count();
        if has_digit(segment) && (2..=8).contains(&word_count) {
            out.push(span(SensitiveSpanKind::PostalAddress, s, e));
        }
    }
}

// ---------------------------------------------------------------------------
// Pattern recognizer entry point + overlap resolution
// ---------------------------------------------------------------------------

fn kind_priority(kind: SensitiveSpanKind) -> u8 {
    match kind {
        SensitiveSpanKind::Email => 0,
        SensitiveSpanKind::Url => 1,
        SensitiveSpanKind::PostalAddress => 2,
        SensitiveSpanKind::IpAddress => 3,
        SensitiveSpanKind::Date => 4,
        SensitiveSpanKind::Phone => 5,
        SensitiveSpanKind::Identifier => 6,
        SensitiveSpanKind::PostalCode => 7,
        SensitiveSpanKind::PersonName => 8,
        SensitiveSpanKind::ModelEntity => 9,
    }
}

/// Resolves overlaps: a span is kept unless it overlaps an already-kept span
/// that is longer, or equally long with higher priority. Result is sorted
/// and non-overlapping.
#[must_use]
pub fn resolve_overlaps(mut spans: Vec<SensitiveSpan>) -> Vec<SensitiveSpan> {
    spans.retain(|s| s.start < s.end);
    spans.sort_by(|a, b| {
        (b.end - b.start)
            .cmp(&(a.end - a.start))
            .then(kind_priority(a.kind).cmp(&kind_priority(b.kind)))
            .then(a.start.cmp(&b.start))
    });
    let mut kept: Vec<SensitiveSpan> = Vec::new();
    for s in spans {
        if kept.iter().all(|k| s.end <= k.start || s.start >= k.end) {
            kept.push(s);
        }
    }
    kept.sort_by_key(|s| s.start);
    kept
}

/// Deterministic pattern recognizer over any UTF-8 text.
#[must_use]
pub fn pattern_recognize(text: &str) -> Vec<SensitiveSpan> {
    let ws = words(text);
    let mut out = Vec::new();
    scan_emails(text, &mut out);
    scan_urls(text, &mut out);
    scan_ipv4(&ws, &mut out);
    scan_dates(text, &ws, &mut out);
    scan_phones(text, &mut out);
    scan_identifiers(text, &ws, &mut out);
    scan_checksum_identifiers(text, &ws, &mut out);
    scan_postal_codes(text, &ws, &mut out);
    scan_addresses(text, &ws, &mut out);
    scan_names(text, &ws, &mut out);
    out.retain(|s| !is_transformed_value(s.kind, &text[s.start..s.end]));
    resolve_overlaps(out)
}

// ---------------------------------------------------------------------------
// FHIR-aware recognizer
// ---------------------------------------------------------------------------

/// True when `value` is already a transform output (placeholder, pseudonym
/// or generalization) and so is not itself a sensitive value.
#[must_use]
pub fn is_transformed_value(kind: SensitiveSpanKind, value: &str) -> bool {
    let v = value.trim();
    if v.is_empty() || is_pseudonym(v) || (v.starts_with('[') && v.ends_with(']')) {
        return true;
    }
    match kind {
        SensitiveSpanKind::Date => v.len() == 4 && v.bytes().all(|b| b.is_ascii_digit()),
        SensitiveSpanKind::PostalCode => v.ends_with("**"),
        SensitiveSpanKind::IpAddress => v.ends_with(".x.x.x"),
        _ => false,
    }
}

/// Collects `(kind, value)` pairs from sensitive FHIR fields.
fn collect_fhir(value: &Value, parent_key: &str, out: &mut Vec<(SensitiveSpanKind, String)>) {
    match value {
        Value::Object(map) => {
            let system = map.get("system").and_then(Value::as_str).unwrap_or("");
            for (key, child) in map {
                match (key.as_str(), child) {
                    ("family" | "given" | "prefix" | "suffix", _) if parent_key == "name" => {
                        push_strings(child, SensitiveSpanKind::PersonName, out);
                    }
                    ("text", Value::String(s)) if parent_key == "name" => {
                        out.push((SensitiveSpanKind::PersonName, s.clone()));
                    }
                    (
                        "birthDate"
                        | "deceasedDateTime"
                        | "multipleBirthDateTime"
                        | "issued"
                        | "effectiveDateTime"
                        | "recordedDate"
                        | "onsetDateTime"
                        | "authoredOn",
                        Value::String(s),
                    ) => out.push((SensitiveSpanKind::Date, s.clone())),
                    ("value", Value::String(s)) if parent_key == "identifier" => {
                        out.push((SensitiveSpanKind::Identifier, s.clone()));
                    }
                    ("value", Value::String(s)) if parent_key == "telecom" => {
                        let kind = if system == "email" || s.contains('@') {
                            SensitiveSpanKind::Email
                        } else if system == "url" {
                            SensitiveSpanKind::Url
                        } else {
                            SensitiveSpanKind::Phone
                        };
                        out.push((kind, s.clone()));
                    }
                    ("line" | "text" | "city" | "district", _) if parent_key == "address" => {
                        push_strings(child, SensitiveSpanKind::PostalAddress, out);
                    }
                    ("postalCode", Value::String(s)) if parent_key == "address" => {
                        out.push((SensitiveSpanKind::PostalCode, s.clone()));
                    }
                    ("display", Value::String(s))
                        if matches!(
                            parent_key,
                            "generalPractitioner" | "subject" | "patient" | "performer" | "author"
                        ) =>
                    {
                        out.push((SensitiveSpanKind::PersonName, s.clone()));
                    }
                    _ => collect_fhir(child, key, out),
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_fhir(item, parent_key, out);
            }
        }
        _ => {}
    }
}

fn push_strings(
    value: &Value,
    kind: SensitiveSpanKind,
    out: &mut Vec<(SensitiveSpanKind, String)>,
) {
    match value {
        Value::String(s) => out.push((kind, s.clone())),
        Value::Array(items) => {
            for item in items {
                push_strings(item, kind, out);
            }
        }
        _ => {}
    }
}

/// Outcome of the FHIR-aware recognizer.
pub enum FhirOutcome {
    /// The text is not a FHIR JSON resource; the recognizer does not apply.
    NotApplicable,
    /// The media type says FHIR but the text does not parse.
    Malformed,
    Spans(Vec<SensitiveSpan>),
}

/// Structured recognizer for FHIR JSON. Spans cover the inside of the JSON
/// string literal of each sensitive value, at every place it occurs.
#[must_use]
pub fn fhir_recognize(text: &str, declared_fhir: bool) -> FhirOutcome {
    let parsed: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) if declared_fhir => return FhirOutcome::Malformed,
        Err(_) => return FhirOutcome::NotApplicable,
    };
    let is_fhir = parsed.get("resourceType").and_then(Value::as_str).is_some();
    if !is_fhir {
        return if declared_fhir {
            FhirOutcome::Malformed
        } else {
            FhirOutcome::NotApplicable
        };
    }
    let mut values = Vec::new();
    collect_fhir(&parsed, "", &mut values);
    let mut spans = Vec::new();
    for (kind, value) in values {
        if value.trim().is_empty() || is_transformed_value(kind, &value) {
            continue;
        }
        let Ok(literal) = serde_json::to_string(&value) else {
            continue;
        };
        for (pos, _) in text.match_indices(&literal) {
            spans.push(SensitiveSpan {
                kind,
                start: pos + 1,
                end: pos + literal.len() - 1,
                recognizer_id: FHIR_RECOGNIZER_ID.to_owned(),
            });
        }
    }
    FhirOutcome::Spans(resolve_overlaps(spans))
}

// ---------------------------------------------------------------------------
// Pseudonyms (HMAC-SHA256 on the workspace sha2)
// ---------------------------------------------------------------------------

/// HMAC-SHA256 (RFC 2104).
#[must_use]
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut block_key = [0_u8; BLOCK];
    if key.len() > BLOCK {
        block_key[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block_key[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36_u8; BLOCK];
    let mut opad = [0x5c_u8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= block_key[i];
        opad[i] ^= block_key[i];
    }
    let inner = Sha256::new()
        .chain_update(ipad)
        .chain_update(message)
        .finalize();
    let outer = Sha256::new()
        .chain_update(opad)
        .chain_update(inner)
        .finalize();
    outer.into()
}

/// Normalizes a value so trivially different spellings share a pseudonym.
#[must_use]
pub fn normalize_value(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Stable keyed pseudonym for `(kind, value)` under one map key.
#[must_use]
pub fn pseudonym_for(key: &[u8], kind: SensitiveSpanKind, value: &str) -> String {
    let message = format!("{}\u{1f}{}", kind.as_str(), normalize_value(value));
    let mac = hmac_sha256(key, message.as_bytes());
    let hex: String = mac.iter().map(|b| format!("{b:02x}")).collect();
    format!("{PSEUDONYM_PREFIX}{}", &hex[..PSEUDONYM_HEX_CHARS])
}

// ---------------------------------------------------------------------------
// Transform application
// ---------------------------------------------------------------------------

/// Generalization per kind (`contracts.md` section 4).
#[must_use]
pub fn generalize(kind: SensitiveSpanKind, value: &str) -> String {
    match kind {
        SensitiveSpanKind::Date => {
            let years: Vec<&str> = value
                .split(|c: char| !c.is_ascii_digit())
                .filter(|p| p.len() == 4)
                .collect();
            match years.first() {
                Some(y) => (*y).to_owned(),
                None => redact_placeholder(kind),
            }
        }
        SensitiveSpanKind::PostalCode => {
            let prefix: String = value
                .chars()
                .filter(|c| !c.is_whitespace())
                .take(3)
                .collect();
            format!("{prefix}**")
        }
        SensitiveSpanKind::IpAddress => match value.split('.').next() {
            Some(first) if !first.is_empty() => format!("{first}.x.x.x"),
            _ => redact_placeholder(kind),
        },
        _ => redact_placeholder(kind),
    }
}

fn redact_placeholder(kind: SensitiveSpanKind) -> String {
    format!("[REDACTED:{}]", kind.placeholder_label())
}

/// One replacement decided by the transform.
pub struct Replacement {
    pub kind: SensitiveSpanKind,
    pub op: TransformOp,
    /// The original value (held in memory only, for sealing).
    pub original: String,
    pub pseudonym: Option<String>,
}

/// Applies `op_for(kind)` to every span of `text` (sorted, non-overlapping).
/// `pseudonym` is called for `Pseudonymize` spans.
pub fn apply_transform(
    text: &str,
    spans: &[SensitiveSpan],
    op_for: impl Fn(SensitiveSpanKind) -> TransformOp,
    mut pseudonym: impl FnMut(SensitiveSpanKind, &str) -> Option<String>,
) -> Result<(String, Vec<Replacement>), String> {
    let mut out = String::with_capacity(text.len());
    let mut replacements = Vec::with_capacity(spans.len());
    let mut token_numbers: Vec<(SensitiveSpanKind, String, usize)> = Vec::new();
    let mut cursor = 0;
    for s in spans {
        s.validate_in(text)?;
        if s.start < cursor {
            return Err("spans overlap".to_owned());
        }
        out.push_str(&text[cursor..s.start]);
        let original = &text[s.start..s.end];
        let op = op_for(s.kind);
        let mut pseudo = None;
        let replacement = match op {
            TransformOp::Redact => redact_placeholder(s.kind),
            TransformOp::Drop => String::new(),
            TransformOp::Generalize => generalize(s.kind, original),
            TransformOp::Tokenize => {
                let norm = normalize_value(original);
                let n = match token_numbers
                    .iter()
                    .find(|(k, v, _)| *k == s.kind && *v == norm)
                {
                    Some((_, _, n)) => *n,
                    None => {
                        let n = token_numbers
                            .iter()
                            .filter(|(k, _, _)| *k == s.kind)
                            .count()
                            + 1;
                        token_numbers.push((s.kind, norm, n));
                        n
                    }
                };
                format!("[{}-{n}]", s.kind.placeholder_label())
            }
            TransformOp::Pseudonymize => {
                let p = pseudonym(s.kind, original)
                    .ok_or_else(|| "pseudonymization requires a map key".to_owned())?;
                pseudo = Some(p.clone());
                p
            }
        };
        out.push_str(&replacement);
        replacements.push(Replacement {
            kind: s.kind,
            op,
            original: original.to_owned(),
            pseudonym: pseudo,
        });
        cursor = s.end;
    }
    out.push_str(&text[cursor..]);
    Ok((out, replacements))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<(SensitiveSpanKind, String)> {
        pattern_recognize(text)
            .into_iter()
            .map(|s| (s.kind, text[s.start..s.end].to_owned()))
            .collect()
    }

    fn has(text: &str, kind: SensitiveSpanKind, value: &str) -> bool {
        let found = kinds(text);
        let ok = found.contains(&(kind, value.to_owned()));
        if !ok {
            eprintln!("missing {kind:?} {value:?}; found {found:?}");
        }
        ok
    }

    /// Synthetic corpus benchmark (T079-03). Class-wise counts of expected
    /// values found or missed, and detections that match no expected value.
    /// The corpus is a development set written alongside the recognizers,
    /// not a held-out benchmark (`RECOGNIZER_BENCHMARK.md`).
    #[test]
    fn synthetic_corpus_class_wise_counts() {
        use std::collections::BTreeMap;
        let corpus: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/privacy_079/corpus.json"))
                .unwrap();
        // kind -> (expected, found, missed, extra)
        let mut table: BTreeMap<&'static str, (u32, u32, u32, u32)> = BTreeMap::new();
        for doc in corpus.as_array().unwrap() {
            let text = doc["text"].as_str().unwrap();
            let declared_fhir = doc["media_type"].as_str().unwrap().contains("fhir");
            let mut spans = pattern_recognize(text);
            if let FhirOutcome::Spans(found) = fhir_recognize(text, declared_fhir) {
                spans.extend(found);
            }
            let spans = resolve_overlaps(spans);
            let detected: Vec<(SensitiveSpanKind, &str)> = spans
                .iter()
                .map(|s| (s.kind, &text[s.start..s.end]))
                .collect();
            let expected: Vec<(SensitiveSpanKind, String)> = doc["expected"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| {
                    (
                        SensitiveSpanKind::parse(e["kind"].as_str().unwrap()).unwrap(),
                        e["value"].as_str().unwrap().to_owned(),
                    )
                })
                .collect();
            for (kind, value) in &expected {
                let row = table.entry(kind.as_str()).or_default();
                row.0 += 1;
                if detected.iter().any(|(k, v)| k == kind && v == value) {
                    row.1 += 1;
                } else {
                    row.2 += 1;
                    eprintln!("MISSED {} {kind:?} {value:?}", doc["id"]);
                }
            }
            for (kind, value) in &detected {
                if !expected.iter().any(|(k, v)| k == kind && v == value) {
                    table.entry(kind.as_str()).or_default().3 += 1;
                    eprintln!("EXTRA {} {kind:?} {value:?}", doc["id"]);
                }
            }
        }
        eprintln!("kind expected found missed extra");
        for (kind, (e, f, m, x)) in &table {
            eprintln!("{kind} {e} {f} {m} {x}");
        }
        let missed: u32 = table.values().map(|r| r.2).sum();
        assert_eq!(
            missed, 0,
            "regression: an expected corpus value is no longer detected"
        );
    }

    #[test]
    fn hmac_matches_rfc4231_case_2() {
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        let hex: String = mac.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn hmac_matches_rfc4231_case_6_long_key() {
        let key = [0xaa_u8; 131];
        let mac = hmac_sha256(
            &key,
            b"Test Using Larger Than Block-Size Key - Hash Key First",
        );
        let hex: String = mac.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn pseudonyms_are_stable_per_key_and_normalized() {
        let a = pseudonym_for(b"k1", SensitiveSpanKind::PersonName, "Jane  Doe");
        assert_eq!(
            a,
            pseudonym_for(b"k1", SensitiveSpanKind::PersonName, "jane doe")
        );
        assert_ne!(
            a,
            pseudonym_for(b"k2", SensitiveSpanKind::PersonName, "Jane Doe")
        );
        assert_ne!(
            a,
            pseudonym_for(b"k1", SensitiveSpanKind::Email, "Jane Doe")
        );
        assert!(is_pseudonym(&a));
    }

    #[test]
    fn recognizes_core_patterns() {
        let t = "Patient: Jane Q. Doe, MRN: 004512. Seen 2024-03-12 by Dr. Alan Smith. \
                 Call +1 555 201 7788 or jane.doe@example.org. Portal https://example.org/p/1. \
                 Host 10.0.4.12. Address: 42 Elm Street, Springfield. SSN 123-45-6789.";
        assert!(has(t, SensitiveSpanKind::PersonName, "Jane Q. Doe"));
        assert!(has(t, SensitiveSpanKind::PersonName, "Alan Smith"));
        assert!(has(t, SensitiveSpanKind::Identifier, "004512"));
        assert!(has(t, SensitiveSpanKind::Date, "2024-03-12"));
        assert!(has(t, SensitiveSpanKind::Phone, "+1 555 201 7788"));
        assert!(has(t, SensitiveSpanKind::Email, "jane.doe@example.org"));
        assert!(has(t, SensitiveSpanKind::Url, "https://example.org/p/1"));
        assert!(has(t, SensitiveSpanKind::IpAddress, "10.0.4.12"));
        assert!(has(t, SensitiveSpanKind::PostalAddress, "42 Elm Street"));
        assert!(has(t, SensitiveSpanKind::Identifier, "123-45-6789"));
    }

    #[test]
    fn recognizes_multilingual_dates() {
        assert!(has(
            "el 12 de marzo de 2024",
            SensitiveSpanKind::Date,
            "12 de marzo de 2024"
        ));
        assert!(has(
            "le 3 mars 2023.",
            SensitiveSpanKind::Date,
            "3 mars 2023"
        ));
        assert!(has(
            "am 5. März 2022",
            SensitiveSpanKind::Date,
            "5. März 2022"
        ));
        assert!(has(
            "on March 4, 2021",
            SensitiveSpanKind::Date,
            "March 4, 2021"
        ));
        assert!(has(
            "visto 03/11/2020",
            SensitiveSpanKind::Date,
            "03/11/2020"
        ));
        assert!(has("am 14.02.1975", SensitiveSpanKind::Date, "14.02.1975"));
    }

    #[test]
    fn does_not_flag_ordinary_clinical_text() {
        let t = "Blood pressure 120/80 mmHg. Metformin 500 mg twice daily. Follow up in 2 weeks.";
        assert!(kinds(t).is_empty(), "{:?}", kinds(t));
    }

    #[test]
    fn transformed_output_is_not_rescanned_as_sensitive() {
        let t =
            "Name: [REDACTED:PERSON_NAME] id PSN-0123456789ab year 1980 [DATE-1] 280** 10.x.x.x";
        assert!(kinds(t).is_empty(), "{:?}", kinds(t));
    }

    #[test]
    fn fhir_recognizer_finds_structured_values_and_flags_malformed() {
        let fhir = r#"{"resourceType":"Patient","name":[{"family":"Rivera","given":["Lucia"]}],
            "birthDate":"1980-07-01","identifier":[{"value":"MRN-778812"}],
            "telecom":[{"system":"phone","value":"+34 600 123 456"}],
            "address":[{"line":["Calle Mayor 12"],"city":"Toledo","postalCode":"45001"}]}"#;
        let FhirOutcome::Spans(spans) = fhir_recognize(fhir, true) else {
            panic!("expected spans");
        };
        let found: Vec<&str> = spans.iter().map(|s| &fhir[s.start..s.end]).collect();
        for v in [
            "Rivera",
            "Lucia",
            "1980-07-01",
            "MRN-778812",
            "+34 600 123 456",
            "Calle Mayor 12",
            "Toledo",
            "45001",
        ] {
            assert!(found.contains(&v), "{v} missing from {found:?}");
        }
        assert!(matches!(
            fhir_recognize("{not json", true),
            FhirOutcome::Malformed
        ));
        assert!(matches!(
            fhir_recognize("plain text", false),
            FhirOutcome::NotApplicable
        ));
        assert!(matches!(
            fhir_recognize("{\"a\":1}", true),
            FhirOutcome::Malformed
        ));
    }

    #[test]
    fn transform_applies_every_op_and_keeps_json_valid() {
        let text = r#"{"resourceType":"Patient","name":[{"family":"Rivera"}],"birthDate":"1980-07-01","address":[{"postalCode":"45001"}]}"#;
        let FhirOutcome::Spans(spans) = fhir_recognize(text, true) else {
            panic!()
        };
        let (out, reps) = apply_transform(
            text,
            &spans,
            |k| match k {
                SensitiveSpanKind::PersonName => TransformOp::Pseudonymize,
                SensitiveSpanKind::Date | SensitiveSpanKind::PostalCode => TransformOp::Generalize,
                _ => TransformOp::Redact,
            },
            |k, v| Some(pseudonym_for(b"key", k, v)),
        )
        .unwrap();
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["birthDate"], "1980");
        assert_eq!(parsed["address"][0]["postalCode"], "450**");
        assert!(is_pseudonym(parsed["name"][0]["family"].as_str().unwrap()));
        assert!(!out.contains("Rivera"));
        assert_eq!(reps.len(), 3);
        let FhirOutcome::Spans(residual) = fhir_recognize(&out, true) else {
            panic!()
        };
        assert!(residual.is_empty());
    }

    #[test]
    fn tokenize_numbers_distinct_values_in_order() {
        let text = "Dr. Ana Ruiz met Dr. Luis Gil and Dr. Ana Ruiz.";
        let spans = pattern_recognize(text);
        let (out, _) =
            apply_transform(text, &spans, |_| TransformOp::Tokenize, |_, _| None).unwrap();
        assert_eq!(
            out,
            "Dr. [PERSON_NAME-1] met Dr. [PERSON_NAME-2] and Dr. [PERSON_NAME-1]."
        );
    }

    #[test]
    fn overlapping_spans_are_resolved_to_the_longest() {
        let a = span(SensitiveSpanKind::Identifier, 0, 5);
        let b = span(SensitiveSpanKind::PostalAddress, 0, 12);
        let c = span(SensitiveSpanKind::PersonName, 13, 15);
        let out = resolve_overlaps(vec![a, b.clone(), c.clone()]);
        assert_eq!(out, vec![b, c]);
    }

    #[test]
    fn checksum_confirmed_identifiers_are_recognized() {
        let text = "Card 4111 1111 1111 1111 on file. IBAN GB82 WEST 1234 5698 7654 32. \
                    NINO AB 12 34 56 C. NHS 943 476 5919. NPI 1234567893.";
        for value in [
            "4111 1111 1111 1111",
            "GB82 WEST 1234 5698 7654 32",
            "AB 12 34 56 C",
            "1234567893",
        ] {
            assert!(
                has(text, SensitiveSpanKind::Identifier, value),
                "{value} recognized"
            );
        }
        // A 3-3-4 NHS number may also match the phone pattern, which has
        // priority at equal length; either way the value is detected.
        assert!(
            has(text, SensitiveSpanKind::Identifier, "943 476 5919")
                || has(text, SensitiveSpanKind::Phone, "943 476 5919")
        );
        // Failing checksums and look-alike words are not flagged by checksum.
        let negative = "Card 4111 1111 1111 1112. HOSPITAL ward. NINO BG 12 34 56 C.";
        let kinds = kinds(negative);
        assert!(
            !kinds
                .iter()
                .any(|(k, v)| *k == SensitiveSpanKind::Identifier
                    && (v.contains("4111") || v == "HOSPITAL" || v.contains("BG 12"))),
            "{kinds:?}"
        );
    }
}
