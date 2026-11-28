//! OpenMed v4.0.0 Wave 3 native pure-Rust DICOM Part 10 parser & PS3.15
//! Basic Application Level Confidentiality Profile de-identifier
//! (header tags, HMAC `2.25.<u128>` UID remap, uncompressed pixel blackout,
//! Structured Report redaction & residual verification).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::multimodal_v4::{MultimodalError, MultimodalErrorKind, sha256_prefixed};

const MAX_DICOM_BYTES: usize = 32 * 1024 * 1024;
const MAX_DICOM_ELEMENTS: usize = 50_000;
const MAX_SQ_DEPTH: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DicomTag {
    pub group: u16,
    pub element: u16,
}

impl DicomTag {
    #[must_use]
    pub const fn new(group: u16, element: u16) -> Self {
        Self { group, element }
    }

    #[must_use]
    pub fn code(&self) -> String {
        format!("({:04X},{:04X})", self.group, self.element)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixelBlackoutRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DicomDeidentifyOptions {
    pub uid_hmac_secret: String,
    pub date_shift_days: i64,
    #[serde(default)]
    pub blackout_rects: Vec<PixelBlackoutRect>,
}

impl Default for DicomDeidentifyOptions {
    fn default() -> Self {
        Self {
            uid_hmac_secret: "medscale-local-dicom-uid-v1".to_owned(),
            date_shift_days: 0,
            blackout_rects: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DicomDeidentifyReport {
    pub transfer_syntax_uid: String,
    pub explicit_vr_dataset: bool,
    pub tags_scrubbed: Vec<String>,
    pub uids_remapped: usize,
    pub sr_items_scrubbed: usize,
    pub blackout_regions_applied: usize,
    pub residual_clean: bool,
    pub input_sha256: String,
    pub output_sha256: String,
    pub scrubbed_bytes: Vec<u8>,
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        let digest = Sha256::digest(key);
        key_block[..32].copy_from_slice(&digest);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5Cu8; 64];
    for i in 0..64 {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(msg);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    let out = outer.finalize();
    let mut res = [0u8; 32];
    res.copy_from_slice(&out);
    res
}

/// Maps any DICOM UID deterministically into `2.25.<u128>` (PS3.5 / RFC 9562 derived UUID-as-OID).
#[must_use]
pub fn remap_dicom_uid(secret: &[u8], original_uid: &str) -> String {
    let mac = hmac_sha256(
        secret,
        original_uid.trim_end_matches(['\0', ' ']).as_bytes(),
    );
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&mac[..16]);
    let val = u128::from_be_bytes(bytes);
    format!("2.25.{val}")
}

fn is_long_explicit_vr(vr: [u8; 2]) -> bool {
    matches!(
        &vr,
        b"OB" | b"OD" | b"OF" | b"OL" | b"OW" | b"SQ" | b"UC" | b"UN" | b"UR" | b"UT"
    )
}

#[derive(Debug, Clone)]
struct Element {
    tag: DicomTag,
    vr: Option<[u8; 2]>,
    value: ElementValue,
}

#[derive(Debug, Clone)]
enum ElementValue {
    Bytes(Vec<u8>),
    Sequence(Vec<Vec<Element>>),
}

fn read_u16_le(b: &[u8], pos: usize) -> Option<u16> {
    let s = b.get(pos..pos + 2)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

fn read_u32_le(b: &[u8], pos: usize) -> Option<u32> {
    let s = b.get(pos..pos + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn parse_elements(
    data: &[u8],
    explicit_vr: bool,
    depth: usize,
    total_seen: &mut usize,
) -> Result<(Vec<Element>, usize), MultimodalError> {
    if depth > MAX_SQ_DEPTH {
        return Err(MultimodalError::new(
            MultimodalErrorKind::ArchiveBombOrTraversal,
            "DICOM sequence nesting limit exceeded",
        ));
    }
    let mut elems = Vec::new();
    let mut pos = 0usize;
    while pos + 8 <= data.len() {
        *total_seen += 1;
        if *total_seen > MAX_DICOM_ELEMENTS {
            return Err(MultimodalError::new(
                MultimodalErrorKind::InputTooLarge,
                "DICOM element limit exceeded",
            ));
        }
        let group = read_u16_le(data, pos).unwrap_or(0);
        let element = read_u16_le(data, pos + 2).unwrap_or(0);
        let tag = DicomTag::new(group, element);

        // Item Delimitation (FFFE,E00D) or Sequence Delimitation (FFFE,E0DD)
        if group == 0xFFFE && (element == 0xE00D || element == 0xE0DD) {
            pos += 8;
            return Ok((elems, pos));
        }

        // File meta (0002,xxxx) is always Explicit VR Little Endian.
        let use_explicit = explicit_vr || group == 0x0002;
        let (vr, hdr_len, val_len) = if use_explicit && group != 0xFFFE {
            let vr_bytes = [
                *data.get(pos + 4).unwrap_or(&b' '),
                *data.get(pos + 5).unwrap_or(&b' '),
            ];
            if !vr_bytes[0].is_ascii_uppercase() || !vr_bytes[1].is_ascii_uppercase() {
                return Err(MultimodalError::new(
                    MultimodalErrorKind::MalformedDocument,
                    format!("invalid DICOM explicit VR at {}", tag.code()),
                ));
            }
            if is_long_explicit_vr(vr_bytes) {
                let len = read_u32_le(data, pos + 8).ok_or_else(|| {
                    MultimodalError::new(
                        MultimodalErrorKind::MalformedDocument,
                        "truncated long VR",
                    )
                })?;
                (Some(vr_bytes), 12usize, len)
            } else {
                let len = read_u16_le(data, pos + 6).ok_or_else(|| {
                    MultimodalError::new(
                        MultimodalErrorKind::MalformedDocument,
                        "truncated short VR",
                    )
                })?;
                (Some(vr_bytes), 8usize, u32::from(len))
            }
        } else {
            let len = read_u32_le(data, pos + 4).ok_or_else(|| {
                MultimodalError::new(
                    MultimodalErrorKind::MalformedDocument,
                    "truncated implicit VR",
                )
            })?;
            (None, 8usize, len)
        };

        pos = pos.saturating_add(hdr_len);
        let is_sq = vr == Some(*b"SQ") || tag == DicomTag::new(0x0040, 0xA730);
        if is_sq {
            let (seq_items, consumed) = if val_len == 0xFFFF_FFFF {
                parse_sq_items(&data[pos..], explicit_vr, depth + 1, true, total_seen)?
            } else {
                let end = pos.saturating_add(val_len as usize);
                if end > data.len() {
                    return Err(MultimodalError::new(
                        MultimodalErrorKind::MalformedDocument,
                        "truncated DICOM SQ payload",
                    ));
                }
                let (items, _) =
                    parse_sq_items(&data[pos..end], explicit_vr, depth + 1, false, total_seen)?;
                (items, val_len as usize)
            };
            pos += consumed;
            elems.push(Element {
                tag,
                vr,
                value: ElementValue::Sequence(seq_items),
            });
            continue;
        }

        if val_len == 0xFFFF_FFFF {
            return Err(MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                format!("unsupported undefined length non-SQ tag {}", tag.code()),
            ));
        }
        let end = pos.saturating_add(val_len as usize);
        if end > data.len() {
            return Err(MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                format!("truncated DICOM tag {}", tag.code()),
            ));
        }
        elems.push(Element {
            tag,
            vr,
            value: ElementValue::Bytes(data[pos..end].to_vec()),
        });
        pos = end;
    }
    Ok((elems, pos))
}

fn parse_sq_items(
    data: &[u8],
    explicit_vr: bool,
    depth: usize,
    delimited: bool,
    total_seen: &mut usize,
) -> Result<(Vec<Vec<Element>>, usize), MultimodalError> {
    let mut items = Vec::new();
    let mut pos = 0usize;
    while pos + 8 <= data.len() {
        let g = read_u16_le(data, pos).unwrap_or(0);
        let e = read_u16_le(data, pos + 2).unwrap_or(0);
        let len = read_u32_le(data, pos + 4).unwrap_or(0);
        if g == 0xFFFE && e == 0xE0DD {
            pos += 8;
            return Ok((items, pos));
        }
        if g != 0xFFFE || e != 0xE000 {
            return Err(MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                "expected DICOM Item tag (FFFE,E000)",
            ));
        }
        pos += 8;
        if len == 0xFFFF_FFFF {
            let (item_elems, used) =
                parse_elements(&data[pos..], explicit_vr, depth + 1, total_seen)?;
            items.push(item_elems);
            pos += used;
        } else {
            let end = pos.saturating_add(len as usize);
            if end > data.len() {
                return Err(MultimodalError::new(
                    MultimodalErrorKind::MalformedDocument,
                    "truncated DICOM SQ Item",
                ));
            }
            let (item_elems, _) =
                parse_elements(&data[pos..end], explicit_vr, depth + 1, total_seen)?;
            items.push(item_elems);
            pos = end;
        }
    }
    if delimited {
        return Err(MultimodalError::new(
            MultimodalErrorKind::MalformedDocument,
            "unterminated undefined-length DICOM SQ",
        ));
    }
    Ok((items, pos))
}

fn pad_dicom_bytes(mut bytes: Vec<u8>, pad: u8) -> Vec<u8> {
    if bytes.len() % 2 == 1 {
        bytes.push(pad);
    }
    bytes
}

fn write_elements(elems: &[Element], explicit_vr: bool, out: &mut Vec<u8>) {
    for el in elems {
        let mut payload = Vec::new();
        match &el.value {
            ElementValue::Bytes(b) => payload.extend_from_slice(b),
            ElementValue::Sequence(items) => {
                for item in items {
                    let mut item_bytes = Vec::new();
                    write_elements(item, explicit_vr, &mut item_bytes);
                    payload.extend_from_slice(&0xFFFEu16.to_le_bytes());
                    payload.extend_from_slice(&0xE000u16.to_le_bytes());
                    payload.extend_from_slice(&(item_bytes.len() as u32).to_le_bytes());
                    payload.extend_from_slice(&item_bytes);
                }
            }
        }
        let use_explicit = explicit_vr || el.tag.group == 0x0002;
        out.extend_from_slice(&el.tag.group.to_le_bytes());
        out.extend_from_slice(&el.tag.element.to_le_bytes());
        if use_explicit {
            let vr = el.vr.unwrap_or(*b"UN");
            out.extend_from_slice(&vr);
            if is_long_explicit_vr(vr) {
                out.extend_from_slice(&[0, 0]);
                out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            } else {
                out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
            }
        } else {
            out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        }
        out.extend_from_slice(&payload);
    }
}

fn shift_da_date(raw: &[u8], days: i64) -> Vec<u8> {
    if days == 0 {
        return Vec::new();
    }
    let s = String::from_utf8_lossy(raw);
    let trimmed = s.trim();
    if trimmed.len() == 8
        && trimmed.bytes().all(|b| b.is_ascii_digit())
        && let (Ok(y), Ok(m), Ok(d)) = (
            trimmed[0..4].parse::<i32>(),
            trimmed[4..6].parse::<u32>(),
            trimmed[6..8].parse::<u32>(),
        )
    {
        // Days since civil epoch (Howard Hinnant algorithm)
        let ord = civil_to_days(y, m, d) + days;
        let (sy, sm, sd) = days_to_civil(ord);
        return format!("{sy:04}{sm:02}{sd:02}").into_bytes();
    }
    Vec::new()
}

fn civil_to_days(y: i32, m: u32, d: u32) -> i64 {
    let y = i64::from(y) - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = i64::from(m);
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn days_to_civil(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let yr = y + if m <= 2 { 1 } else { 0 };
    (yr as i32, m as u32, d as u32)
}

fn is_ps315_remove_tag(tag: DicomTag) -> bool {
    matches!(
        (tag.group, tag.element),
        (0x0008, 0x0081)
            | (0x0008, 0x0082)
            | (0x0008, 0x0092)
            | (0x0008, 0x0094)
            | (0x0008, 0x0096)
            | (0x0008, 0x1040)
            | (0x0008, 0x1048)
            | (0x0008, 0x1050)
            | (0x0008, 0x1060)
            | (0x0008, 0x1070)
            | (0x0008, 0x1080)
            | (0x0010, 0x0032)
            | (0x0010, 0x1000)
            | (0x0010, 0x1001)
            | (0x0010, 0x1040)
            | (0x0010, 0x1060)
            | (0x0010, 0x2154)
            | (0x0010, 0x21B0)
            | (0x0010, 0x4000)
            | (0x0018, 0x1000)
            | (0x0040, 0x0275)
    )
}

fn is_uid_remap_tag(tag: DicomTag) -> bool {
    matches!(
        (tag.group, tag.element),
        (0x0002, 0x0003)
            | (0x0008, 0x0018)
            | (0x0020, 0x000D)
            | (0x0020, 0x000E)
            | (0x0020, 0x0052)
    )
}

pub fn deidentify_dicom_part10(
    dicom_bytes: &[u8],
    options: &DicomDeidentifyOptions,
    text_scrubber: &dyn Fn(&str) -> String,
) -> Result<DicomDeidentifyReport, MultimodalError> {
    if dicom_bytes.len() > MAX_DICOM_BYTES {
        return Err(MultimodalError::new(
            MultimodalErrorKind::InputTooLarge,
            "DICOM payload exceeds 32 MiB limit",
        ));
    }
    if dicom_bytes.len() < 136 || &dicom_bytes[128..132] != b"DICM" {
        return Err(MultimodalError::new(
            MultimodalErrorKind::MalformedDocument,
            "invalid DICOM Part 10 header (missing 128-byte preamble + 'DICM')",
        ));
    }

    let input_sha256 = sha256_prefixed(dicom_bytes);
    let mut total_seen = 0usize;
    let body = &dicom_bytes[132..];

    // Split Group 0002 (Explicit VR LE) from Group >= 0008.
    let mut meta_end = 0usize;
    while meta_end + 8 <= body.len() {
        let g = read_u16_le(body, meta_end).unwrap_or(0);
        if g != 0x0002 {
            break;
        }
        let vr = [body[meta_end + 4], body[meta_end + 5]];
        let (hdr, len) = if is_long_explicit_vr(vr) {
            let l = read_u32_le(body, meta_end + 8).unwrap_or(0) as usize;
            (12usize, l)
        } else {
            let l = read_u16_le(body, meta_end + 6).unwrap_or(0) as usize;
            (8usize, l)
        };
        meta_end = meta_end.saturating_add(hdr).saturating_add(len);
        if meta_end > body.len() {
            return Err(MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                "truncated DICOM File Meta Information",
            ));
        }
    }

    let (mut meta_elems, _) = parse_elements(&body[..meta_end], true, 0, &mut total_seen)?;
    let transfer_syntax_uid = meta_elems
        .iter()
        .find(|e| e.tag == DicomTag::new(0x0002, 0x0010))
        .and_then(|e| match &e.value {
            ElementValue::Bytes(b) => Some(
                String::from_utf8_lossy(b)
                    .trim_end_matches(['\0', ' '])
                    .to_owned(),
            ),
            ElementValue::Sequence(_) => None,
        })
        .unwrap_or_else(|| "1.2.840.10008.1.2.1".to_owned());

    let explicit_vr_dataset = transfer_syntax_uid != "1.2.840.10008.1.2";
    let is_uncompressed = matches!(
        transfer_syntax_uid.as_str(),
        "1.2.840.10008.1.2" | "1.2.840.10008.1.2.1"
    );

    let (mut ds_elems, _) =
        parse_elements(&body[meta_end..], explicit_vr_dataset, 0, &mut total_seen)?;

    // Read pixel geometry BEFORE scrubbing for blackout rects.
    let mut rows = 0u32;
    let mut cols = 0u32;
    let mut bits_alloc = 8u16;
    let mut samples_per_pixel = 1u16;
    for el in &ds_elems {
        if let ElementValue::Bytes(b) = &el.value {
            match (el.tag.group, el.tag.element) {
                (0x0028, 0x0010) if b.len() >= 2 => {
                    rows = u32::from(u16::from_le_bytes([b[0], b[1]]))
                }
                (0x0028, 0x0011) if b.len() >= 2 => {
                    cols = u32::from(u16::from_le_bytes([b[0], b[1]]))
                }
                (0x0028, 0x0100) if b.len() >= 2 => bits_alloc = u16::from_le_bytes([b[0], b[1]]),
                (0x0028, 0x0002) if b.len() >= 2 => {
                    samples_per_pixel = u16::from_le_bytes([b[0], b[1]])
                }
                _ => {}
            }
        }
    }

    let mut tags_scrubbed = BTreeSet::new();
    let mut uids_remapped = 0usize;
    let mut sr_items_scrubbed = 0usize;
    let mut blackout_regions_applied = 0usize;
    let secret = options.uid_hmac_secret.as_bytes();

    // Update meta elements (MediaStorageSOPInstanceUID).
    for el in &mut meta_elems {
        if is_uid_remap_tag(el.tag)
            && let ElementValue::Bytes(ref mut b) = el.value
        {
            let orig = String::from_utf8_lossy(b).into_owned();
            *b = pad_dicom_bytes(remap_dicom_uid(secret, &orig).into_bytes(), b'\0');
            uids_remapped += 1;
            tags_scrubbed.insert(el.tag.code());
        }
    }

    // Remove PS3.15 private/direct-drop tags (keep explicit list & remove group-0010/0008 dropped items).
    ds_elems.retain(|el| {
        if is_ps315_remove_tag(el.tag) {
            tags_scrubbed.insert(el.tag.code());
            false
        } else {
            true
        }
    });

    for el in &mut ds_elems {
        if is_uid_remap_tag(el.tag) {
            if let ElementValue::Bytes(ref mut b) = el.value {
                let orig = String::from_utf8_lossy(b).into_owned();
                *b = pad_dicom_bytes(remap_dicom_uid(secret, &orig).into_bytes(), b'\0');
                uids_remapped += 1;
                tags_scrubbed.insert(el.tag.code());
            }
            continue;
        }
        match (el.tag.group, el.tag.element) {
            (0x0010, 0x0010) | (0x0008, 0x0080) | (0x0008, 0x0090) | (0x0008, 0x1030) => {
                el.value = ElementValue::Bytes(pad_dicom_bytes(b"ANONYMIZED".to_vec(), b' '));
                tags_scrubbed.insert(el.tag.code());
            }
            (0x0010, 0x0020) | (0x0008, 0x0050) => {
                if let ElementValue::Bytes(ref b) = el.value {
                    let orig = String::from_utf8_lossy(b);
                    let mac = hmac_sha256(secret, orig.trim().as_bytes());
                    let id = format!(
                        "ANON_{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
                        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
                    );
                    el.value = ElementValue::Bytes(pad_dicom_bytes(id.into_bytes(), b' '));
                    tags_scrubbed.insert(el.tag.code());
                }
            }
            (0x0010, 0x0030) | (0x0008, 0x0020) | (0x0008, 0x0021) | (0x0008, 0x0022) => {
                if let ElementValue::Bytes(ref b) = el.value {
                    let shifted = shift_da_date(b, options.date_shift_days);
                    el.value = ElementValue::Bytes(pad_dicom_bytes(shifted, b' '));
                    tags_scrubbed.insert(el.tag.code());
                }
            }
            (0x0008, 0x0030) | (0x0008, 0x0031) => {
                el.value = ElementValue::Bytes(Vec::new());
                tags_scrubbed.insert(el.tag.code());
            }
            (0x0040, 0xA730) => {
                if let ElementValue::Sequence(ref mut items) = el.value {
                    sr_items_scrubbed +=
                        scrub_sr_sequence(items, text_scrubber, secret, &mut tags_scrubbed);
                }
            }
            (0x7FE0, 0x0010) if !options.blackout_rects.is_empty() => {
                if !is_uncompressed {
                    return Err(MultimodalError::new(
                        MultimodalErrorKind::EncryptedUnsupported,
                        "pixel blackout requires uncompressed transfer syntax",
                    ));
                }
                if let ElementValue::Bytes(ref mut pixels) = el.value {
                    let bpp =
                        usize::from(bits_alloc / 8).max(1) * usize::from(samples_per_pixel).max(1);
                    for rect in &options.blackout_rects {
                        let y_end = rect.y.saturating_add(rect.height).min(rows);
                        let x_end = rect.x.saturating_add(rect.width).min(cols);
                        for y in rect.y..y_end {
                            for x in rect.x..x_end {
                                let off = (y as usize * cols as usize + x as usize) * bpp;
                                if let Some(slice) = pixels.get_mut(off..off + bpp) {
                                    slice.fill(0);
                                }
                            }
                        }
                        blackout_regions_applied += 1;
                    }
                    tags_scrubbed.insert(el.tag.code());
                }
            }
            _ => {}
        }
    }

    // Recompute GroupLength (0002,0000) cleanly if present.
    let meta_body = {
        let non_len: Vec<Element> = meta_elems
            .iter()
            .filter(|e| e.tag != DicomTag::new(0x0002, 0x0000))
            .cloned()
            .collect();
        let mut tmp = Vec::new();
        write_elements(&non_len, true, &mut tmp);
        let len_elem = Element {
            tag: DicomTag::new(0x0002, 0x0000),
            vr: Some(*b"UL"),
            value: ElementValue::Bytes((tmp.len() as u32).to_le_bytes().to_vec()),
        };
        let mut hdr_out = Vec::with_capacity(12 + tmp.len());
        write_elements(&[len_elem], true, &mut hdr_out);
        hdr_out.extend_from_slice(&tmp);
        hdr_out
    };

    let mut out = Vec::with_capacity(132 + meta_body.len() + 256);
    out.resize(128, 0);
    out.extend_from_slice(b"DICM");
    out.extend_from_slice(&meta_body);
    write_elements(&ds_elems, explicit_vr_dataset, &mut out);

    let output_sha256 = sha256_prefixed(&out);
    let residual_clean = verify_dicom_residual(&out, explicit_vr_dataset);

    Ok(DicomDeidentifyReport {
        transfer_syntax_uid,
        explicit_vr_dataset,
        tags_scrubbed: tags_scrubbed.into_iter().collect(),
        uids_remapped,
        sr_items_scrubbed,
        blackout_regions_applied,
        residual_clean,
        input_sha256,
        output_sha256,
        scrubbed_bytes: out,
    })
}

fn scrub_sr_sequence(
    items: &mut [Vec<Element>],
    scrubber: &dyn Fn(&str) -> String,
    secret: &[u8],
    scrubbed_tags: &mut BTreeSet<String>,
) -> usize {
    let mut count = 0usize;
    for item in items {
        for el in item {
            if el.tag == DicomTag::new(0x0040, 0xA160)
                && let ElementValue::Bytes(ref mut b) = el.value
            {
                let orig = String::from_utf8_lossy(b);
                let clean = scrubber(orig.trim_end_matches('\0'));
                *b = pad_dicom_bytes(clean.into_bytes(), b' ');
                scrubbed_tags.insert(el.tag.code());
                count += 1;
            } else if el.tag == DicomTag::new(0x0040, 0xA123) {
                el.value = ElementValue::Bytes(pad_dicom_bytes(b"ANONYMIZED".to_vec(), b' '));
                scrubbed_tags.insert(el.tag.code());
                count += 1;
            } else if el.tag == DicomTag::new(0x0008, 0x1155)
                && let ElementValue::Bytes(ref mut b) = el.value
            {
                let orig = String::from_utf8_lossy(b).into_owned();
                *b = pad_dicom_bytes(remap_dicom_uid(secret, &orig).into_bytes(), b'\0');
                scrubbed_tags.insert(el.tag.code());
                count += 1;
            } else if el.tag == DicomTag::new(0x0040, 0xA730)
                && let ElementValue::Sequence(ref mut sub) = el.value
            {
                count += scrub_sr_sequence(sub, scrubber, secret, scrubbed_tags);
            }
        }
    }
    count
}

fn verify_dicom_residual(out_bytes: &[u8], explicit_vr: bool) -> bool {
    if out_bytes.len() < 136 || &out_bytes[128..132] != b"DICM" {
        return false;
    }
    let mut seen = 0usize;
    let Ok((elems, _)) = parse_elements(&out_bytes[132..], explicit_vr, 0, &mut seen) else {
        return false;
    };
    for el in &elems {
        if is_ps315_remove_tag(el.tag) {
            return false;
        }
        if matches!(
            (el.tag.group, el.tag.element),
            (0x0010, 0x0010) | (0x0008, 0x0080) | (0x0008, 0x0090)
        ) && let ElementValue::Bytes(b) = &el.value
        {
            let s = String::from_utf8_lossy(b);
            if s.trim() != "ANONYMIZED" {
                return false;
            }
        }
    }
    true
}
