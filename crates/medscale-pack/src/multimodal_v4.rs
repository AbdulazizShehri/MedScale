//! OpenMed v4.0.0 Wave 3 pure-Rust DoS-bounded multimodal extractors,
//! scrubbers, archive/PDF safety preflight, OOXML residual verification,
//! and sanitized local Tesseract OCR integration (zero new external crates).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_MULTIMODAL_INPUT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_ARCHIVE_ENTRIES: usize = 4_096;
pub const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_MEMBER_UNCOMPRESSED_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_EXPANSION_RATIO: f64 = 100.0;
pub const MAX_ARCHIVE_PATH_LEN: usize = 4_096;

pub const MAX_PDF_PAGES: usize = 10_000;
pub const MAX_PDF_OBJECTS: usize = 250_000;
pub const MAX_PDF_DECOMPRESSED_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MultimodalErrorKind {
    InputTooLarge,
    ArchiveBombOrTraversal,
    MalformedDocument,
    EncryptedUnsupported,
    BlockedExternalRights,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{kind:?}: {message}")]
pub struct MultimodalError {
    pub kind: MultimodalErrorKind,
    pub message: String,
}

impl MultimodalError {
    #[must_use]
    pub fn new(kind: MultimodalErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

pub fn sha256_prefixed(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    format!("sha256:{}", hex_lower(&digest))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}

/* ───────────────────────── Archive Bomb / Path Traversal Preflight ───────────────────────── */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveSafetyDecision {
    Allow,
    Quarantine,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveSafetyReason {
    InvalidMetadata,
    AbsolutePath,
    PathTraversal,
    PathTooLong,
    Link,
    DuplicatePath,
    EntryLimit,
    MemberSizeLimit,
    TotalSizeLimit,
    ExpansionRatio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveMemberMeta {
    pub path: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub link_target: Option<String>,
}

fn default_kind() -> String {
    "file".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveSafetyPolicy {
    pub max_entries: usize,
    pub max_total_uncompressed_bytes: u64,
    pub max_member_uncompressed_bytes: u64,
    pub max_expansion_ratio_x100: u64,
    pub max_path_length: usize,
}

impl Default for ArchiveSafetyPolicy {
    fn default() -> Self {
        Self {
            max_entries: MAX_ARCHIVE_ENTRIES,
            max_total_uncompressed_bytes: MAX_TOTAL_UNCOMPRESSED_BYTES,
            max_member_uncompressed_bytes: MAX_MEMBER_UNCOMPRESSED_BYTES,
            max_expansion_ratio_x100: (MAX_EXPANSION_RATIO * 100.0) as u64,
            max_path_length: MAX_ARCHIVE_PATH_LEN,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveSafetyReport {
    pub decision: ArchiveSafetyDecision,
    pub entry_count: usize,
    pub total_compressed_bytes: u64,
    pub total_uncompressed_bytes: u64,
    pub reasons: Vec<ArchiveSafetyReason>,
}

fn is_windows_reserved(segment: &str) -> bool {
    let base = segment.split('.').next().unwrap_or(segment).to_lowercase();
    matches!(
        base.as_str(),
        "aux"
            | "clock$"
            | "con"
            | "conin$"
            | "conout$"
            | "nul"
            | "prn"
            | "com1"
            | "com2"
            | "com3"
            | "com4"
            | "com5"
            | "com6"
            | "com7"
            | "com8"
            | "com9"
            | "lpt1"
            | "lpt2"
            | "lpt3"
            | "lpt4"
            | "lpt5"
            | "lpt6"
            | "lpt7"
            | "lpt8"
            | "lpt9"
    )
}

pub fn evaluate_archive_members(
    members: &[ArchiveMemberMeta],
    policy: &ArchiveSafetyPolicy,
) -> ArchiveSafetyReport {
    let mut reasons = BTreeSet::new();
    let mut seen_paths = BTreeSet::new();
    let mut total_compressed = 0u64;
    let mut total_uncompressed = 0u64;

    if members.len() > policy.max_entries {
        reasons.insert(ArchiveSafetyReason::EntryLimit);
    }

    for m in members {
        if m.path.len() > policy.max_path_length {
            reasons.insert(ArchiveSafetyReason::PathTooLong);
        }
        let kind_norm = m.kind.trim().to_lowercase();
        if m.link_target.is_some()
            || matches!(
                kind_norm.as_str(),
                "symlink" | "symbolic-link" | "hardlink" | "hard-link" | "link"
            )
        {
            reasons.insert(ArchiveSafetyReason::Link);
        }
        if !matches!(
            kind_norm.as_str(),
            "file" | "regular" | "regular-file" | "directory" | "dir" | ""
        ) && !reasons.contains(&ArchiveSafetyReason::Link)
        {
            reasons.insert(ArchiveSafetyReason::InvalidMetadata);
        }
        let p = m.path.as_str();
        let bytes = p.as_bytes();
        let has_drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
        if p.is_empty() || p.starts_with('/') || p.starts_with('\\') || has_drive {
            reasons.insert(ArchiveSafetyReason::AbsolutePath);
        }
        if p.contains('\\')
            || p.split('/').any(|seg| {
                seg == ".."
                    || is_windows_reserved(seg)
                    || seg
                        .chars()
                        .any(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\0'))
            })
        {
            reasons.insert(ArchiveSafetyReason::PathTraversal);
        }
        let norm_key = p.trim_end_matches('/').to_owned();
        if !seen_paths.insert(norm_key) {
            reasons.insert(ArchiveSafetyReason::DuplicatePath);
        }
        if m.uncompressed_size > policy.max_member_uncompressed_bytes {
            reasons.insert(ArchiveSafetyReason::MemberSizeLimit);
        }
        if m.compressed_size == 0 && m.uncompressed_size > 0 {
            reasons.insert(ArchiveSafetyReason::ExpansionRatio);
        } else if m.compressed_size > 0 {
            let lhs = u128::from(m.uncompressed_size) * 100u128;
            let rhs = u128::from(m.compressed_size) * u128::from(policy.max_expansion_ratio_x100);
            if lhs > rhs {
                reasons.insert(ArchiveSafetyReason::ExpansionRatio);
            }
        }
        total_compressed = total_compressed.saturating_add(m.compressed_size);
        total_uncompressed = total_uncompressed.saturating_add(m.uncompressed_size);
    }

    if total_uncompressed > policy.max_total_uncompressed_bytes {
        reasons.insert(ArchiveSafetyReason::TotalSizeLimit);
    }

    let decision = if reasons.contains(&ArchiveSafetyReason::PathTraversal)
        || reasons.contains(&ArchiveSafetyReason::AbsolutePath)
        || reasons.contains(&ArchiveSafetyReason::Link)
        || reasons.contains(&ArchiveSafetyReason::InvalidMetadata)
        || reasons.contains(&ArchiveSafetyReason::DuplicatePath)
    {
        ArchiveSafetyDecision::Reject
    } else if !reasons.is_empty() {
        ArchiveSafetyDecision::Quarantine
    } else {
        ArchiveSafetyDecision::Allow
    };

    ArchiveSafetyReport {
        decision,
        entry_count: members.len(),
        total_compressed_bytes: total_compressed,
        total_uncompressed_bytes: total_uncompressed,
        reasons: reasons.into_iter().collect(),
    }
}

/* ───────────────────────── Pure-Rust CRC32 + RFC 1951 Inflate + ZIP ───────────────────────── */

fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bit_buf: u64,
    bit_count: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bit_buf: 0,
            bit_count: 0,
        }
    }

    fn ensure(&mut self, n: u8) -> bool {
        while self.bit_count < n {
            if self.pos >= self.data.len() {
                return false;
            }
            self.bit_buf |= u64::from(self.data[self.pos]) << self.bit_count;
            self.pos += 1;
            self.bit_count += 8;
        }
        true
    }

    fn read_bits(&mut self, n: u8) -> Option<u32> {
        if n == 0 {
            return Some(0);
        }
        if !self.ensure(n) {
            return None;
        }
        let val = (self.bit_buf & ((1u64 << n) - 1)) as u32;
        self.bit_buf >>= n;
        self.bit_count -= n;
        Some(val)
    }

    fn align_byte(&mut self) {
        let drop_bits = self.bit_count % 8;
        self.bit_buf >>= drop_bits;
        self.bit_count -= drop_bits;
    }

    fn read_aligned_u16(&mut self) -> Option<u16> {
        self.align_byte();
        let lo = self.read_bits(8)? as u16;
        let hi = self.read_bits(8)? as u16;
        Some(lo | (hi << 8))
    }

    fn read_aligned_bytes(&mut self, n: usize, out: &mut Vec<u8>, cap: usize) -> bool {
        self.align_byte();
        if out.len().saturating_add(n) > cap {
            return false;
        }
        for _ in 0..n {
            let Some(b) = self.read_bits(8) else {
                return false;
            };
            out.push(b as u8);
        }
        true
    }
}

struct HuffTree {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl HuffTree {
    fn build(lengths: &[u8]) -> Option<Self> {
        let mut counts = [0u16; 16];
        for &len in lengths {
            if usize::from(len) >= counts.len() {
                return None;
            }
            if len > 0 {
                counts[usize::from(len)] = counts[usize::from(len)].checked_add(1)?;
            }
        }
        let mut left = 1i32;
        for &cnt in &counts[1..] {
            left <<= 1;
            left -= i32::from(cnt);
            if left < 0 {
                return None;
            }
        }
        let mut offsets = [0u16; 16];
        let mut total = 0u16;
        for bits in 1..16 {
            offsets[bits] = total;
            total = total.checked_add(counts[bits])?;
        }
        let mut symbols = vec![0u16; usize::from(total)];
        for (sym, &len) in lengths.iter().enumerate() {
            if len > 0 {
                let idx = usize::from(offsets[usize::from(len)]);
                offsets[usize::from(len)] += 1;
                if idx >= symbols.len() {
                    return None;
                }
                symbols[idx] = sym as u16;
            }
        }
        Some(Self { counts, symbols })
    }

    fn decode(&self, reader: &mut BitReader<'_>) -> Option<u16> {
        let mut code = 0i32;
        let mut first = 0i32;
        let mut index = 0i32;
        for len in 1..16 {
            let bit = reader.read_bits(1)? as i32;
            code |= bit;
            let count = i32::from(self.counts[len]);
            if code - count < first {
                return self.symbols.get((index + (code - first)) as usize).copied();
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        None
    }
}

pub fn inflate_raw_deflate(src: &[u8], max_out: usize) -> Result<Vec<u8>, MultimodalError> {
    const LEN_BASE: [u16; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LEN_EXTRA: [u8; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const DIST_BASE: [u16; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DIST_EXTRA: [u8; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];
    const CL_ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];

    let mal = || {
        MultimodalError::new(
            MultimodalErrorKind::MalformedDocument,
            "corrupt or over-limit deflate stream",
        )
    };
    let mut br = BitReader::new(src);
    let mut out = Vec::new();

    loop {
        let bfinal = br.read_bits(1).ok_or_else(mal)?;
        let btype = br.read_bits(2).ok_or_else(mal)?;

        if btype == 0 {
            let len = br.read_aligned_u16().ok_or_else(mal)?;
            let nlen = br.read_aligned_u16().ok_or_else(mal)?;
            if len != !nlen {
                return Err(mal());
            }
            if !br.read_aligned_bytes(usize::from(len), &mut out, max_out) {
                return Err(MultimodalError::new(
                    MultimodalErrorKind::ArchiveBombOrTraversal,
                    "decompressed stream exceeded cap",
                ));
            }
        } else if btype == 1 || btype == 2 {
            let (lit_tree, dist_tree) = if btype == 1 {
                let mut lit_lens = vec![0u8; 288];
                lit_lens[..144].fill(8);
                lit_lens[144..256].fill(9);
                lit_lens[256..280].fill(7);
                lit_lens[280..288].fill(8);
                let dist_lens = vec![5u8; 32];
                (
                    HuffTree::build(&lit_lens).ok_or_else(mal)?,
                    HuffTree::build(&dist_lens).ok_or_else(mal)?,
                )
            } else {
                let hlit = br.read_bits(5).ok_or_else(mal)? as usize + 257;
                let hdist = br.read_bits(5).ok_or_else(mal)? as usize + 1;
                let hclen = br.read_bits(4).ok_or_else(mal)? as usize + 4;
                if hlit > 286 || hdist > 30 || hclen > 19 {
                    return Err(mal());
                }
                let mut cl_lens = [0u8; 19];
                for i in 0..hclen {
                    cl_lens[CL_ORDER[i]] = br.read_bits(3).ok_or_else(mal)? as u8;
                }
                let cl_tree = HuffTree::build(&cl_lens).ok_or_else(mal)?;
                let total_codes = hlit + hdist;
                let mut all_lens = Vec::with_capacity(total_codes);
                while all_lens.len() < total_codes {
                    let sym = cl_tree.decode(&mut br).ok_or_else(mal)?;
                    match sym {
                        0..=15 => all_lens.push(sym as u8),
                        16 => {
                            let prev = *all_lens.last().ok_or_else(mal)?;
                            let rep = br.read_bits(2).ok_or_else(mal)? as usize + 3;
                            if all_lens.len() + rep > total_codes {
                                return Err(mal());
                            }
                            all_lens.resize(all_lens.len() + rep, prev);
                        }
                        17 => {
                            let rep = br.read_bits(3).ok_or_else(mal)? as usize + 3;
                            if all_lens.len() + rep > total_codes {
                                return Err(mal());
                            }
                            all_lens.resize(all_lens.len() + rep, 0);
                        }
                        18 => {
                            let rep = br.read_bits(7).ok_or_else(mal)? as usize + 11;
                            if all_lens.len() + rep > total_codes {
                                return Err(mal());
                            }
                            all_lens.resize(all_lens.len() + rep, 0);
                        }
                        _ => return Err(mal()),
                    }
                }
                (
                    HuffTree::build(&all_lens[..hlit]).ok_or_else(mal)?,
                    HuffTree::build(&all_lens[hlit..]).ok_or_else(mal)?,
                )
            };

            loop {
                let sym = lit_tree.decode(&mut br).ok_or_else(mal)?;
                if sym < 256 {
                    if out.len() >= max_out {
                        return Err(MultimodalError::new(
                            MultimodalErrorKind::ArchiveBombOrTraversal,
                            "decompressed stream exceeded cap",
                        ));
                    }
                    out.push(sym as u8);
                } else if sym == 256 {
                    break;
                } else {
                    let l_idx = usize::from(sym.checked_sub(257).ok_or_else(mal)?);
                    if l_idx >= LEN_BASE.len() {
                        return Err(mal());
                    }
                    let extra_l = br.read_bits(LEN_EXTRA[l_idx]).ok_or_else(mal)? as usize;
                    let length = usize::from(LEN_BASE[l_idx]) + extra_l;
                    let d_sym = usize::from(dist_tree.decode(&mut br).ok_or_else(mal)?);
                    if d_sym >= DIST_BASE.len() {
                        return Err(mal());
                    }
                    let extra_d = br.read_bits(DIST_EXTRA[d_sym]).ok_or_else(mal)? as usize;
                    let dist = usize::from(DIST_BASE[d_sym]) + extra_d;
                    if dist == 0 || dist > out.len() {
                        return Err(mal());
                    }
                    if out.len().saturating_add(length) > max_out {
                        return Err(MultimodalError::new(
                            MultimodalErrorKind::ArchiveBombOrTraversal,
                            "decompressed stream exceeded cap",
                        ));
                    }
                    for _ in 0..length {
                        let byte = out[out.len() - dist];
                        out.push(byte);
                    }
                }
            }
        } else {
            return Err(mal());
        }

        if bfinal == 1 {
            break;
        }
    }

    Ok(out)
}

pub fn inflate_zlib(src: &[u8], max_out: usize) -> Result<Vec<u8>, MultimodalError> {
    if src.len() < 6 {
        return Err(MultimodalError::new(
            MultimodalErrorKind::MalformedDocument,
            "truncated zlib stream",
        ));
    }
    let cmf = src[0];
    let flg = src[1];
    if (cmf & 0x0F) != 8 || !u16::from_be_bytes([cmf, flg]).is_multiple_of(31) {
        return Err(MultimodalError::new(
            MultimodalErrorKind::MalformedDocument,
            "invalid zlib header",
        ));
    }
    let mut start = 2usize;
    if (flg & 0x20) != 0 {
        start = 6;
    }
    if start >= src.len() {
        return Err(MultimodalError::new(
            MultimodalErrorKind::MalformedDocument,
            "empty zlib payload",
        ));
    }
    inflate_raw_deflate(&src[start..], max_out)
}

fn read_le_u16(b: &[u8], off: usize) -> Option<u16> {
    let s = b.get(off..off + 2)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

fn read_le_u32(b: &[u8], off: usize) -> Option<u32> {
    let s = b.get(off..off + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

pub fn read_zip_entries_checked(
    zip_bytes: &[u8],
    policy: &ArchiveSafetyPolicy,
) -> Result<(BTreeMap<String, Vec<u8>>, ArchiveSafetyReport), MultimodalError> {
    if zip_bytes.len() > MAX_MULTIMODAL_INPUT_BYTES {
        return Err(MultimodalError::new(
            MultimodalErrorKind::InputTooLarge,
            "zip payload exceeds maximum 32 MiB",
        ));
    }
    let min_scan = zip_bytes.len().saturating_sub(65_557);
    let eocd_pos = (min_scan..=zip_bytes.len().saturating_sub(22))
        .rev()
        .find(|&i| zip_bytes.get(i..i + 4) == Some(b"PK\x05\x06"))
        .ok_or_else(|| {
            MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                "missing ZIP end-of-central-directory record",
            )
        })?;
    let total_entries = read_le_u16(zip_bytes, eocd_pos + 10).unwrap_or(0) as usize;
    let cd_size = read_le_u32(zip_bytes, eocd_pos + 12).unwrap_or(0) as usize;
    let cd_offset = read_le_u32(zip_bytes, eocd_pos + 16).unwrap_or(0) as usize;
    if cd_offset.saturating_add(cd_size) > eocd_pos || total_entries > policy.max_entries {
        return Err(MultimodalError::new(
            MultimodalErrorKind::ArchiveBombOrTraversal,
            "ZIP central directory bounds or entry limit exceeded",
        ));
    }

    let mut metas = Vec::with_capacity(total_entries);
    let mut raw_slices = Vec::with_capacity(total_entries);
    let mut cur = cd_offset;
    for _ in 0..total_entries {
        if cur.saturating_add(46) > eocd_pos || zip_bytes.get(cur..cur + 4) != Some(b"PK\x01\x02") {
            return Err(MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                "corrupt ZIP central directory entry",
            ));
        }
        let method = read_le_u16(zip_bytes, cur + 10).unwrap_or(0);
        let comp_sz = read_le_u32(zip_bytes, cur + 20).unwrap_or(0) as u64;
        let uncomp_sz = read_le_u32(zip_bytes, cur + 24).unwrap_or(0) as u64;
        let name_len = read_le_u16(zip_bytes, cur + 28).unwrap_or(0) as usize;
        let extra_len = read_le_u16(zip_bytes, cur + 30).unwrap_or(0) as usize;
        let comment_len = read_le_u16(zip_bytes, cur + 32).unwrap_or(0) as usize;
        let ext_attr = read_le_u32(zip_bytes, cur + 38).unwrap_or(0);
        let lfh_off = read_le_u32(zip_bytes, cur + 42).unwrap_or(0) as usize;
        let name_start = cur + 46;
        let name_end = name_start.saturating_add(name_len);
        if name_end > eocd_pos {
            return Err(MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                "truncated ZIP entry name",
            ));
        }
        let name = String::from_utf8_lossy(&zip_bytes[name_start..name_end]).into_owned();
        let unix_mode = (ext_attr >> 16) & 0xF000;
        let kind = if unix_mode == 0xA000 {
            "symlink"
        } else if name.ends_with('/') {
            "directory"
        } else {
            "file"
        };
        metas.push(ArchiveMemberMeta {
            path: name.clone(),
            compressed_size: comp_sz,
            uncompressed_size: uncomp_sz,
            kind: kind.to_owned(),
            link_target: (unix_mode == 0xA000).then(|| "symlink".to_owned()),
        });
        raw_slices.push((name, method, comp_sz, uncomp_sz, lfh_off));
        cur = name_end
            .saturating_add(extra_len)
            .saturating_add(comment_len);
    }

    let report = evaluate_archive_members(&metas, policy);
    if report.decision != ArchiveSafetyDecision::Allow {
        return Err(MultimodalError::new(
            MultimodalErrorKind::ArchiveBombOrTraversal,
            format!("archive safety refused: {:?}", report.reasons),
        ));
    }

    let mut out = BTreeMap::new();
    let mut running_total = 0u64;
    for (name, method, comp_sz, uncomp_sz, lfh_off) in raw_slices {
        if name.ends_with('/') {
            continue;
        }
        if lfh_off.saturating_add(30) > cd_offset
            || zip_bytes.get(lfh_off..lfh_off + 4) != Some(b"PK\x03\x04")
        {
            return Err(MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                "corrupt ZIP local file header",
            ));
        }
        let l_name_len = read_le_u16(zip_bytes, lfh_off + 26).unwrap_or(0) as usize;
        let l_extra_len = read_le_u16(zip_bytes, lfh_off + 28).unwrap_or(0) as usize;
        let data_start = lfh_off + 30 + l_name_len + l_extra_len;
        let data_end = data_start.saturating_add(comp_sz as usize);
        if data_end > cd_offset {
            return Err(MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                "truncated ZIP entry data",
            ));
        }
        let payload = &zip_bytes[data_start..data_end];
        let cap = (policy.max_member_uncompressed_bytes as usize).min(MAX_MULTIMODAL_INPUT_BYTES);
        let decoded = match method {
            0 => {
                if payload.len() as u64 != uncomp_sz {
                    return Err(MultimodalError::new(
                        MultimodalErrorKind::MalformedDocument,
                        "stored ZIP size mismatch",
                    ));
                }
                payload.to_vec()
            }
            8 => inflate_raw_deflate(payload, cap)?,
            other => {
                return Err(MultimodalError::new(
                    MultimodalErrorKind::MalformedDocument,
                    format!("unsupported ZIP compression method {other}"),
                ));
            }
        };
        if decoded.len() as u64 > policy.max_member_uncompressed_bytes
            || running_total.saturating_add(decoded.len() as u64)
                > policy.max_total_uncompressed_bytes
        {
            return Err(MultimodalError::new(
                MultimodalErrorKind::ArchiveBombOrTraversal,
                "actual decompressed ZIP bytes exceeded policy",
            ));
        }
        running_total += decoded.len() as u64;
        out.insert(name, decoded);
    }

    Ok((out, report))
}

pub fn write_stored_zip(entries: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut cd = Vec::new();
    for (name, data) in entries {
        let offset = buf.len() as u32;
        let name_bytes = name.as_bytes();
        let crc = crc32_ieee(data);
        let size = data.len() as u32;
        // Local File Header
        let _ = buf.write_all(b"PK\x03\x04");
        let _ = buf.write_all(&20u16.to_le_bytes());
        let _ = buf.write_all(&0x0800u16.to_le_bytes()); // UTF-8
        let _ = buf.write_all(&0u16.to_le_bytes()); // stored
        let _ = buf.write_all(&0u32.to_le_bytes()); // dos time/date
        let _ = buf.write_all(&crc.to_le_bytes());
        let _ = buf.write_all(&size.to_le_bytes());
        let _ = buf.write_all(&size.to_le_bytes());
        let _ = buf.write_all(&(name_bytes.len() as u16).to_le_bytes());
        let _ = buf.write_all(&0u16.to_le_bytes());
        let _ = buf.write_all(name_bytes);
        let _ = buf.write_all(data);

        // Central Directory Header
        let _ = cd.write_all(b"PK\x01\x02");
        let _ = cd.write_all(&20u16.to_le_bytes());
        let _ = cd.write_all(&20u16.to_le_bytes());
        let _ = cd.write_all(&0x0800u16.to_le_bytes());
        let _ = cd.write_all(&0u16.to_le_bytes());
        let _ = cd.write_all(&0u32.to_le_bytes());
        let _ = cd.write_all(&crc.to_le_bytes());
        let _ = cd.write_all(&size.to_le_bytes());
        let _ = cd.write_all(&size.to_le_bytes());
        let _ = cd.write_all(&(name_bytes.len() as u16).to_le_bytes());
        let _ = cd.write_all(&0u16.to_le_bytes());
        let _ = cd.write_all(&0u16.to_le_bytes());
        let _ = cd.write_all(&0u16.to_le_bytes());
        let _ = cd.write_all(&0u16.to_le_bytes());
        let _ = cd.write_all(&0u32.to_le_bytes());
        let _ = cd.write_all(&offset.to_le_bytes());
        let _ = cd.write_all(name_bytes);
    }
    let cd_offset = buf.len() as u32;
    let cd_size = cd.len() as u32;
    let count = entries.len() as u16;
    buf.extend_from_slice(&cd);
    let _ = buf.write_all(b"PK\x05\x06");
    let _ = buf.write_all(&0u32.to_le_bytes());
    let _ = buf.write_all(&count.to_le_bytes());
    let _ = buf.write_all(&count.to_le_bytes());
    let _ = buf.write_all(&cd_size.to_le_bytes());
    let _ = buf.write_all(&cd_offset.to_le_bytes());
    let _ = buf.write_all(&0u16.to_le_bytes());
    buf
}

/* ───────────────────────── Format Extractors, Scrubbers & Verifiers ───────────────────────── */

pub fn refuse_outlook_msg_gpl3(data_or_ext: &str, bytes: &[u8]) -> Result<(), MultimodalError> {
    let ext_is_msg = data_or_ext.trim().eq_ignore_ascii_case("msg")
        || data_or_ext.trim().to_ascii_lowercase().ends_with(".msg");
    let is_ole2 = bytes.starts_with(b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1");
    if ext_is_msg || (is_ole2 && data_or_ext.contains("msg")) {
        return Err(MultimodalError::new(
            MultimodalErrorKind::BlockedExternalRights,
            "OLE2 .msg parsing requires GPL-3.0 extract-msg and is quarantined in MedScale; convert to RFC 5322 .eml",
        ));
    }
    Ok(())
}

fn decode_basic_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

#[must_use]
pub fn extract_html_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut hidden: Option<&'static str> = None;
    let mut i = 0usize;
    while i < html.len() {
        let rest = &html[i..];
        if let Some(tag_end_rel) = rest.find('>')
            && rest.starts_with('<')
        {
            let raw_tag = &rest[1..tag_end_rel];
            let lower_tag = raw_tag.trim().to_ascii_lowercase();
            let tag_name = lower_tag
                .trim_start_matches('/')
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("");
            if let Some(active) = hidden {
                if lower_tag.starts_with('/') && tag_name == active {
                    hidden = None;
                }
            } else if !lower_tag.starts_with('/') {
                match tag_name {
                    "script" => hidden = Some("script"),
                    "style" => hidden = Some("style"),
                    "template" => hidden = Some("template"),
                    "br" | "p" | "div" | "li" | "tr" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                        if !out.ends_with('\n') && !out.is_empty() =>
                    {
                        out.push('\n');
                    }
                    _ => {}
                }
            } else if matches!(
                tag_name,
                "p" | "div" | "li" | "tr" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
            ) && !out.ends_with('\n')
                && !out.is_empty()
            {
                out.push('\n');
            }
            i += tag_end_rel + 1;
            continue;
        }
        if hidden.is_none() {
            if let Some(next_lt) = rest.find('<') {
                out.push_str(&decode_basic_entities(&rest[..next_lt]));
                i += next_lt;
            } else {
                out.push_str(&decode_basic_entities(rest));
                break;
            }
        } else {
            let Some(ch) = rest.chars().next() else {
                break;
            };
            i += ch.len_utf8();
        }
    }
    let lines: Vec<&str> = out
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    lines.join("\n")
}

#[must_use]
pub fn extract_markdown_text(md: &str) -> String {
    let mut lines = Vec::new();
    for line in md.lines() {
        let mut t = line.trim();
        if t.starts_with("```") {
            continue;
        }
        while t.starts_with('#') {
            t = t.trim_start_matches('#');
        }
        let mut clean = t
            .trim()
            .replace("**", "")
            .replace("__", "")
            .replace('`', "");
        if let Some(stripped) = clean.strip_prefix("> ") {
            clean = stripped.to_owned();
        }
        if !clean.is_empty() {
            lines.push(clean);
        }
    }
    lines.join("\n")
}

#[must_use]
pub fn extract_rtf_text(rtf: &str) -> String {
    let mut out = String::new();
    let mut skip_depth: Option<usize> = None;
    let mut depth = 0usize;
    let bytes = rtf.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                if skip_depth.is_none() {
                    let ahead = &rtf[i..rtf.len().min(i + 20)];
                    if ahead.starts_with("{\\*")
                        || ahead.starts_with("{\\fonttbl")
                        || ahead.starts_with("{\\colortbl")
                        || ahead.starts_with("{\\stylesheet")
                        || ahead.starts_with("{\\info")
                    {
                        skip_depth = Some(depth);
                    }
                }
                i += 1;
            }
            b'}' => {
                if skip_depth == Some(depth) {
                    skip_depth = None;
                }
                depth = depth.saturating_sub(1);
                i += 1;
            }
            b'\\' => {
                i += 1;
                if i >= bytes.len() {
                    break;
                }
                if matches!(bytes[i], b'{' | b'}' | b'\\') {
                    if skip_depth.is_none() {
                        out.push(bytes[i] as char);
                    }
                    i += 1;
                } else if bytes[i] == b'\'' && i + 2 < bytes.len() {
                    if skip_depth.is_none()
                        && let Ok(val) = u8::from_str_radix(&rtf[i + 1..i + 3], 16)
                        && val.is_ascii()
                    {
                        out.push(val as char);
                    }
                    i += 3;
                } else {
                    let start = i;
                    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    let word = &rtf[start..i];
                    if i < bytes.len() && (bytes[i] == b'-' || bytes[i].is_ascii_digit()) {
                        i += 1;
                        while i < bytes.len() && bytes[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                    if i < bytes.len() && bytes[i] == b' ' {
                        i += 1;
                    }
                    if skip_depth.is_none() && (word == "par" || word == "line") {
                        out.push('\n');
                    }
                }
            }
            b'\r' | b'\n' => i += 1,
            _ => {
                let ch = rtf[i..].chars().next().unwrap_or('\0');
                if skip_depth.is_none() && ch != '\0' {
                    out.push(ch);
                }
                i += ch.len_utf8().max(1);
            }
        }
    }
    out.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn scrub_rtf_info_and_text(rtf: &str, scrubber: &dyn Fn(&str) -> String) -> String {
    let mut cleaned = String::with_capacity(rtf.len());
    let mut i = 0usize;
    while i < rtf.len() {
        if rtf[i..].starts_with("{\\info") {
            let mut depth = 0usize;
            while i < rtf.len() {
                let b = rtf.as_bytes()[i];
                if b == b'{' {
                    depth += 1;
                } else if b == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                i += 1;
            }
            continue;
        }
        let ch = rtf[i..].chars().next().unwrap_or(' ');
        cleaned.push(ch);
        i += ch.len_utf8();
    }
    scrubber(&cleaned)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmlScrubResult {
    pub scrubbed_eml: String,
    pub body_text: String,
    pub scrubbed_headers: Vec<String>,
}

pub fn scrub_eml(raw: &str, scrubber: &dyn Fn(&str) -> String) -> EmlScrubResult {
    let norm = raw.replace("\r\n", "\n");
    let (hdrs, body) = norm.split_once("\n\n").unwrap_or((norm.as_str(), ""));
    let phi_hdrs = [
        "from:",
        "to:",
        "cc:",
        "bcc:",
        "reply-to:",
        "sender:",
        "return-path:",
        "x-original-to:",
    ];
    let mut scrubbed_headers = Vec::new();
    let mut out_hdrs = Vec::new();
    for line in hdrs.lines() {
        let lower = line.to_ascii_lowercase();
        if phi_hdrs.iter().any(|p| lower.starts_with(p)) {
            if let Some((k, _)) = line.split_once(':') {
                scrubbed_headers.push(k.trim().to_owned());
                out_hdrs.push(format!("{k}: [REDACTED:HEADER]"));
            }
        } else if lower.starts_with("subject:") {
            if let Some((k, v)) = line.split_once(':') {
                scrubbed_headers.push(k.trim().to_owned());
                out_hdrs.push(format!("{k}: {}", scrubber(v.trim())));
            }
        } else {
            out_hdrs.push(line.to_owned());
        }
    }
    let scrubbed_body = scrubber(body);
    EmlScrubResult {
        scrubbed_eml: format!("{}\n\n{}", out_hdrs.join("\n"), scrubbed_body),
        body_text: scrubbed_body,
        scrubbed_headers,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabularColumnSpec {
    pub name: String,
    pub assigned_class: String,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabularScrubResult {
    pub scrubbed_csv: String,
    pub columns: Vec<TabularColumnSpec>,
}

fn split_csv_row(line: &str, delim: char) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            if in_quotes && chars.peek() == Some(&'"') {
                cur.push('"');
                let _ = chars.next();
            } else {
                in_quotes = !in_quotes;
            }
        } else if c == delim && !in_quotes {
            cells.push(cur.clone());
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    cells.push(cur);
    cells
}

fn classify_tabular_header(header: &str) -> (&'static str, &'static str) {
    let h = header.trim().to_ascii_lowercase();
    if h.contains("mrn") || h.contains("ssn") || h == "id" || h.ends_with("_id") {
        ("DIRECT_ID", "hash")
    } else if h.contains("name")
        || h.contains("dob")
        || h.contains("birth")
        || h.contains("phone")
        || h.contains("email")
        || h.contains("address")
    {
        ("DIRECT_ID", "mask")
    } else if h.contains("zip") || h.contains("age") || h.contains("gender") || h.contains("date") {
        ("QUASI_ID", "mask")
    } else {
        ("SAFE", "keep")
    }
}

pub fn scrub_tabular_csv(raw: &str, scrubber: &dyn Fn(&str) -> String) -> TabularScrubResult {
    let mut lines = raw.lines();
    let Some(header_line) = lines.next() else {
        return TabularScrubResult {
            scrubbed_csv: String::new(),
            columns: Vec::new(),
        };
    };
    let delim = if header_line.contains('\t') {
        '\t'
    } else {
        ','
    };
    let headers = split_csv_row(header_line, delim);
    let columns: Vec<TabularColumnSpec> = headers
        .iter()
        .map(|h| {
            let (assigned_class, action) = classify_tabular_header(h);
            TabularColumnSpec {
                name: h.clone(),
                assigned_class: assigned_class.to_owned(),
                action: action.to_owned(),
            }
        })
        .collect();

    let mut out_lines = Vec::new();
    out_lines.push(headers.join(&delim.to_string()));
    for row in lines {
        if row.trim().is_empty() {
            continue;
        }
        let cells = split_csv_row(row, delim);
        let mut scrubbed_cells = Vec::with_capacity(cells.len());
        for (idx, cell) in cells.iter().enumerate() {
            let spec = columns.get(idx);
            let out_cell = match spec.map(|s| s.action.as_str()) {
                Some("hash") => {
                    let h = Sha256::digest(cell.as_bytes());
                    format!("ID_NUM_{}", &hex_lower(&h)[..8])
                }
                Some("mask") => {
                    let name = spec
                        .map(|s| s.name.to_ascii_lowercase())
                        .unwrap_or_default();
                    if name.contains("dob") || name.contains("birth") {
                        "[DATE_OF_BIRTH]".to_owned()
                    } else if name.contains("name") {
                        "[PERSON]".to_owned()
                    } else {
                        scrubber(cell)
                    }
                }
                _ => scrubber(cell),
            };
            scrubbed_cells.push(out_cell);
        }
        out_lines.push(scrubbed_cells.join(&delim.to_string()));
    }

    TabularScrubResult {
        scrubbed_csv: out_lines.join("\n"),
        columns,
    }
}

pub fn scrub_chatlog_jsonl(raw: &str, scrubber: &dyn Fn(&str) -> String) -> String {
    let speaker_keys = [
        "speaker",
        "speaker_id",
        "participant",
        "participant_id",
        "author",
        "author_id",
        "user",
        "user_id",
        "name",
    ];
    let mut out = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(trimmed)
            && let Some(obj) = val.as_object_mut()
        {
            for k in speaker_keys {
                if let Some(serde_json::Value::String(orig)) = obj.get(k).cloned() {
                    let digest = Sha256::digest(orig.as_bytes());
                    obj.insert(
                        k.to_owned(),
                        serde_json::Value::String(format!("USERNAME_{}", &hex_lower(&digest)[..8])),
                    );
                }
            }
            for content_key in ["content", "text"] {
                if let Some(serde_json::Value::String(orig)) = obj.get(content_key).cloned() {
                    obj.insert(
                        content_key.to_owned(),
                        serde_json::Value::String(scrubber(&orig)),
                    );
                }
            }
            if let Ok(s) = serde_json::to_string(&val) {
                out.push(s);
            }
        } else {
            out.push(scrubber(trimmed));
        }
    }
    out.join("\n")
}

#[must_use]
pub fn scrub_vcard(raw: &str) -> String {
    raw.lines()
        .map(|l| {
            let u = l.to_ascii_uppercase();
            if u.starts_with("FN:")
                || u.starts_with("N:")
                || u.starts_with("TEL")
                || u.starts_with("EMAIL")
                || u.starts_with("ADR")
                || u.starts_with("BDAY:")
            {
                let key = l.split(':').next().unwrap_or("PHI");
                format!("{key}:[REDACTED:VCARD]")
            } else {
                l.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn scrub_ical(raw: &str, scrubber: &dyn Fn(&str) -> String) -> String {
    raw.lines()
        .map(|l| {
            let u = l.to_ascii_uppercase();
            if u.starts_with("ATTENDEE") || u.starts_with("ORGANIZER") {
                let key = l.split(':').next().unwrap_or("PHI");
                format!("{key}:[REDACTED:ICAL]")
            } else if u.starts_with("SUMMARY:")
                || u.starts_with("DESCRIPTION:")
                || u.starts_with("LOCATION:")
            {
                let (k, v) = l.split_once(':').unwrap_or((l, ""));
                format!("{k}:{}", scrubber(v))
            } else {
                l.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn scrub_subtitles_vtt_srt(raw: &str, scrubber: &dyn Fn(&str) -> String) -> String {
    let mut out = Vec::new();
    for line in raw.lines() {
        let t = line.trim();
        if t.contains("-->")
            || t.eq_ignore_ascii_case("WEBVTT")
            || t.chars().all(|c| c.is_ascii_digit())
        {
            out.push(line.to_owned());
        } else {
            // Strip `<v SpeakerName>` WebVTT voice tags before scrubbing caption prose.
            let mut clean = String::new();
            let mut rest = line;
            while let Some(vs) = rest.find("<v ") {
                clean.push_str(&rest[..vs]);
                clean.push_str("<v SPEAKER>");
                if let Some(end) = rest[vs..].find('>') {
                    rest = &rest[vs + end + 1..];
                } else {
                    rest = "";
                    break;
                }
            }
            clean.push_str(rest);
            out.push(scrubber(&clean));
        }
    }
    out.join("\n")
}

/* ───────────────────────── OOXML Residual Verification & Scrubbing ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OoxmlResidualFinding {
    pub category: String,
    pub count: usize,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OoxmlScrubResult {
    pub extracted_text: String,
    pub residual_findings: Vec<OoxmlResidualFinding>,
    pub clean_zip_bytes: Vec<u8>,
    pub post_scrub_findings: Vec<OoxmlResidualFinding>,
}

fn ooxml_part_category(name: &str) -> Option<&'static str> {
    if matches!(name, "docProps/core.xml") {
        Some("core_properties")
    } else if matches!(name, "docProps/app.xml") {
        Some("app_properties")
    } else if matches!(name, "docProps/custom.xml") {
        Some("custom_properties")
    } else if name.contains("comments") {
        Some("comments")
    } else if name.starts_with("word/embeddings/")
        || name.starts_with("xl/embeddings/")
        || name.starts_with("ppt/embeddings/")
        || name.starts_with("customXml/")
    {
        Some("embedded_objects")
    } else {
        None
    }
}

fn inspect_ooxml_map(entries: &BTreeMap<String, Vec<u8>>) -> Vec<OoxmlResidualFinding> {
    let mut counts: BTreeMap<String, (usize, Sha256)> = BTreeMap::new();
    for (name, data) in entries {
        if let Some(cat) = ooxml_part_category(name) {
            let entry = counts
                .entry(cat.to_owned())
                .or_insert_with(|| (0, Sha256::new()));
            entry.0 += 1;
            entry.1.update(data);
        }
        if name.ends_with(".xml") {
            let xml = String::from_utf8_lossy(data);
            if xml.contains("<w:ins")
                || xml.contains("<w:del")
                || xml.contains("<w:delText")
                || xml.contains("w:commentRangeStart")
            {
                let entry = counts
                    .entry("tracked_changes".to_owned())
                    .or_insert_with(|| (0, Sha256::new()));
                entry.0 += 1;
                entry.1.update(data);
            }
            if (name.starts_with("word/document")
                || name.starts_with("xl/sharedStrings")
                || name.starts_with("ppt/slides/"))
                && (xml.contains("<w:t") || xml.contains("<t") || xml.contains("<a:t"))
            {
                let entry = counts
                    .entry("uncovered_text".to_owned())
                    .or_insert_with(|| (0, Sha256::new()));
                entry.0 += 1;
                entry.1.update(data);
            }
        }
    }
    counts
        .into_iter()
        .map(|(category, (count, hasher))| OoxmlResidualFinding {
            category,
            count,
            digest: hex_lower(&hasher.finalize()),
        })
        .collect()
}

pub fn scrub_ooxml_zip(
    zip_bytes: &[u8],
    scrubber: &dyn Fn(&str) -> String,
) -> Result<OoxmlScrubResult, MultimodalError> {
    let (mut entries, _) = read_zip_entries_checked(zip_bytes, &ArchiveSafetyPolicy::default())?;
    let residual_findings = inspect_ooxml_map(&entries);

    let mut extracted_parts = Vec::new();
    let keys: Vec<String> = entries.keys().cloned().collect();
    for k in keys {
        if matches!(
            k.as_str(),
            "docProps/core.xml" | "docProps/app.xml" | "docProps/custom.xml"
        ) || k.contains("comments")
            || k.starts_with("customXml/")
            || k.contains("/embeddings/")
        {
            entries.remove(&k);
            continue;
        }
        if k.starts_with("word/document")
            || k.starts_with("xl/sharedStrings")
            || k.starts_with("xl/worksheets/")
            || k.starts_with("ppt/slides/")
            || k.ends_with(".xhtml")
            || k.ends_with(".html")
        {
            let raw_xml = String::from_utf8_lossy(&entries[&k]).into_owned();
            extracted_parts.push(extract_html_text(&raw_xml));
            // Strip tracked-revision `<w:del>...</w:del>` nodes & author attrs cleanly while preserving XML tags.
            let stripped = strip_xml_blocks(&raw_xml, "w:del");
            let scrubbed_xml = scrub_xml_text_nodes(&stripped, scrubber);
            entries.insert(k, scrubbed_xml.into_bytes());
        }
    }

    let post_scrub_findings = inspect_ooxml_map(&entries)
        .into_iter()
        .filter(|f| f.category != "uncovered_text")
        .collect();
    let clean_zip_bytes = write_stored_zip(&entries);
    Ok(OoxmlScrubResult {
        extracted_text: extracted_parts.join("\n"),
        residual_findings,
        clean_zip_bytes,
        post_scrub_findings,
    })
}

fn strip_xml_blocks(xml: &str, tag: &str) -> String {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(pos) = rest.find(&open) {
        out.push_str(&rest[..pos]);
        if let Some(end_rel) = rest[pos..].find(&close) {
            rest = &rest[pos + end_rel + close.len()..];
        } else {
            break;
        }
    }
    out.push_str(rest);
    out
}

fn scrub_xml_text_nodes(xml: &str, scrubber: &dyn Fn(&str) -> String) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(gt) = rest.find('>') {
        out.push_str(&rest[..=gt]);
        rest = &rest[gt + 1..];
        if let Some(lt) = rest.find('<') {
            if lt > 0 {
                let text = &rest[..lt];
                if text.trim().is_empty() {
                    out.push_str(text);
                } else {
                    out.push_str(&scrubber(text));
                }
            }
            rest = &rest[lt..];
        } else {
            out.push_str(rest);
            break;
        }
    }
    out
}

/* ───────────────────────── PDF Geometry Preflight & Text/Metadata Scrub ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfPreflightScrubResult {
    pub status: String,
    pub version_major: u8,
    pub version_minor: u8,
    pub declared_page_count: usize,
    pub reason_codes: Vec<String>,
    pub extracted_text: String,
    pub scrubbed_pdf_bytes: Vec<u8>,
    pub residual_clean: bool,
}

pub fn preflight_and_scrub_pdf(
    pdf_bytes: &[u8],
    scrubber: &dyn Fn(&str) -> String,
) -> Result<PdfPreflightScrubResult, MultimodalError> {
    if pdf_bytes.len() > MAX_MULTIMODAL_INPUT_BYTES {
        return Err(MultimodalError::new(
            MultimodalErrorKind::InputTooLarge,
            "PDF exceeds maximum 32 MiB",
        ));
    }
    let header_slice = &pdf_bytes[..pdf_bytes.len().min(1024)];
    let Some(hdr_pos) = header_slice.windows(5).position(|w| w == b"%PDF-") else {
        return Err(MultimodalError::new(
            MultimodalErrorKind::MalformedDocument,
            "pdf_header_missing",
        ));
    };
    let ver_major = pdf_bytes
        .get(hdr_pos + 5)
        .copied()
        .filter(u8::is_ascii_digit)
        .map(|b| b - b'0')
        .unwrap_or(1);
    let ver_minor = pdf_bytes
        .get(hdr_pos + 7)
        .copied()
        .filter(u8::is_ascii_digit)
        .map(|b| b - b'0')
        .unwrap_or(4);

    let raw_str = String::from_utf8_lossy(pdf_bytes);
    if raw_str.contains("/Encrypt") {
        return Err(MultimodalError::new(
            MultimodalErrorKind::EncryptedUnsupported,
            "pdf_encrypted: encrypted PDFs are refused",
        ));
    }
    let obj_count = raw_str.matches(" obj").count();
    if obj_count > MAX_PDF_OBJECTS {
        return Err(MultimodalError::new(
            MultimodalErrorKind::InputTooLarge,
            "pdf_object_limit",
        ));
    }
    let mut declared_page_count = 0usize;
    if let Some(idx) = raw_str.find("/Count ") {
        let num_str: String = raw_str[idx + 7..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        declared_page_count = num_str.parse().unwrap_or(0);
    }
    let page_markers = raw_str
        .matches("/Type /Page")
        .count()
        .saturating_sub(raw_str.matches("/Type /Pages").count());
    if declared_page_count == 0 {
        declared_page_count = page_markers;
    }
    if declared_page_count > MAX_PDF_PAGES {
        return Err(MultimodalError::new(
            MultimodalErrorKind::InputTooLarge,
            "pdf_page_limit",
        ));
    }
    let has_media_box = raw_str.contains("/MediaBox");
    let mut reason_codes = Vec::new();
    if !has_media_box {
        reason_codes.push("media_box_missing".to_owned());
    }
    let status = if reason_codes.is_empty() && declared_page_count > 0 {
        "readable"
    } else if declared_page_count > 0 {
        "review"
    } else {
        "rejected"
    };

    // Extract text literals `(...) Tj` / `(...) '` plus decompress FlateDecode streams safely.
    let mut texts = vec![extract_pdf_text_literals(&raw_str)];
    let mut search_from = 0usize;
    while let Some(rel) = pdf_bytes[search_from..]
        .windows(6)
        .position(|w| w == b"stream")
    {
        let s_pos = search_from + rel + 6;
        let payload_start = if pdf_bytes.get(s_pos..s_pos + 2) == Some(b"\r\n") {
            s_pos + 2
        } else if matches!(pdf_bytes.get(s_pos), Some(b'\n' | b'\r')) {
            s_pos + 1
        } else {
            search_from = s_pos;
            continue;
        };
        if let Some(end_rel) = pdf_bytes[payload_start..]
            .windows(9)
            .position(|w| w == b"endstream")
        {
            let slice = &pdf_bytes[payload_start..payload_start + end_rel];
            if slice.starts_with(b"\x78")
                && let Ok(inflated) = inflate_zlib(slice, MAX_PDF_DECOMPRESSED_BYTES)
            {
                let dec_str = String::from_utf8_lossy(&inflated);
                let more = extract_pdf_text_literals(&dec_str);
                if !more.is_empty() {
                    texts.push(more);
                }
            }
            search_from = payload_start + end_rel + 9;
        } else {
            break;
        }
    }
    let extracted_text = texts
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    // Scrub uncompressed XMP packet and Info string literals with length-preserving replacement or clean rewrite.
    let has_flate_stream = raw_str_contains_flate(&raw_str);
    let mut scrubbed = raw_str.into_owned();
    if let Some(xmp_start) = scrubbed.find("<x:xmpmeta")
        && let Some(xmp_end_rel) = scrubbed[xmp_start..].find("</x:xmpmeta>")
    {
        let xmp_end = xmp_start + xmp_end_rel + "</x:xmpmeta>".len();
        let blank = " ".repeat(xmp_end - xmp_start);
        scrubbed.replace_range(xmp_start..xmp_end, &blank);
    }
    scrubbed = scrubber(&scrubbed);
    let has_residual_raw = scrubbed.contains("/Author (") || scrubbed.contains("/Title (");

    Ok(PdfPreflightScrubResult {
        status: status.to_owned(),
        version_major: ver_major,
        version_minor: ver_minor,
        declared_page_count,
        reason_codes,
        extracted_text,
        scrubbed_pdf_bytes: scrubbed.into_bytes(),
        residual_clean: !has_residual_raw && !has_flate_stream,
    })
}

fn raw_str_contains_flate(s: &str) -> bool {
    s.contains("/FlateDecode") && s.contains("BT")
}

fn extract_pdf_text_literals(s: &str) -> String {
    let mut out = String::new();
    let mut in_paren = false;
    let mut escape = false;
    for ch in s.chars() {
        if in_paren {
            if escape {
                out.push(ch);
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == ')' {
                in_paren = false;
                out.push(' ');
            } else {
                out.push(ch);
            }
        } else if ch == '(' {
            in_paren = true;
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/* ───────────────────────── Media Scrubbers (PNG / JPEG / WAV / MP3 / EDF) & OCR ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryScrubReport {
    pub format: String,
    pub removed_segments: Vec<String>,
    pub clean_bytes: Vec<u8>,
    pub output_sha256: String,
}

pub fn scrub_png_metadata(png: &[u8]) -> Result<BinaryScrubReport, MultimodalError> {
    const SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if png.len() < 8 || &png[..8] != SIG {
        return Err(MultimodalError::new(
            MultimodalErrorKind::MalformedDocument,
            "invalid PNG signature",
        ));
    }
    let mut out = SIG.to_vec();
    let mut removed = Vec::new();
    let mut pos = 8usize;
    while pos + 12 <= png.len() {
        let len = u32::from_be_bytes([png[pos], png[pos + 1], png[pos + 2], png[pos + 3]]) as usize;
        let ctype = std::str::from_utf8(&png[pos + 4..pos + 8]).unwrap_or("????");
        let end = pos.saturating_add(12).saturating_add(len);
        if end > png.len() {
            return Err(MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                "truncated PNG chunk",
            ));
        }
        if matches!(ctype, "tEXt" | "zTXt" | "iTXt" | "eXIf" | "tIME") {
            removed.push(ctype.to_owned());
        } else {
            out.extend_from_slice(&png[pos..end]);
        }
        pos = end;
        if ctype == "IEND" {
            break;
        }
    }
    let digest = sha256_prefixed(&out);
    Ok(BinaryScrubReport {
        format: "png".to_owned(),
        removed_segments: removed,
        clean_bytes: out,
        output_sha256: digest,
    })
}

pub fn scrub_jpeg_metadata(jpeg: &[u8]) -> Result<BinaryScrubReport, MultimodalError> {
    if jpeg.len() < 4 || jpeg[0] != 0xFF || jpeg[1] != 0xD8 {
        return Err(MultimodalError::new(
            MultimodalErrorKind::MalformedDocument,
            "invalid JPEG SOI header",
        ));
    }
    let mut out = vec![0xFF, 0xD8];
    let mut removed = Vec::new();
    let mut pos = 2usize;
    while pos + 1 < jpeg.len() {
        if jpeg[pos] != 0xFF {
            return Err(MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                "malformed JPEG marker",
            ));
        }
        let marker = jpeg[pos + 1];
        if marker == 0xD9 || marker == 0xDA {
            // EOI or SOS (scan entropy follows to end)
            out.extend_from_slice(&jpeg[pos..]);
            break;
        }
        if (0xD0..=0xD7).contains(&marker) || marker == 0x01 || marker == 0x00 {
            out.extend_from_slice(&jpeg[pos..pos + 2]);
            pos += 2;
            continue;
        }
        if pos + 4 > jpeg.len() {
            return Err(MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                "truncated JPEG segment",
            ));
        }
        let seg_len = u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]) as usize;
        if seg_len < 2 || pos + 2 + seg_len > jpeg.len() {
            return Err(MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                "invalid JPEG segment length",
            ));
        }
        let seg_end = pos + 2 + seg_len;
        if marker == 0xE1 || marker == 0xFE {
            removed.push(format!("0xFF{:02X}", marker));
        } else {
            out.extend_from_slice(&jpeg[pos..seg_end]);
        }
        pos = seg_end;
    }
    let digest = sha256_prefixed(&out);
    Ok(BinaryScrubReport {
        format: "jpeg".to_owned(),
        removed_segments: removed,
        clean_bytes: out,
        output_sha256: digest,
    })
}

pub fn scrub_wav_metadata(wav: &[u8]) -> Result<BinaryScrubReport, MultimodalError> {
    if wav.len() < 12 || &wav[..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return Err(MultimodalError::new(
            MultimodalErrorKind::MalformedDocument,
            "invalid RIFF/WAVE header",
        ));
    }
    let mut chunks = Vec::new();
    let mut removed = Vec::new();
    let mut pos = 12usize;
    while pos + 8 <= wav.len() {
        let id = std::str::from_utf8(&wav[pos..pos + 4]).unwrap_or("????");
        let sz =
            u32::from_le_bytes([wav[pos + 4], wav[pos + 5], wav[pos + 6], wav[pos + 7]]) as usize;
        let padded = sz.saturating_add(sz % 2);
        let end = pos.saturating_add(8).saturating_add(padded);
        if end > wav.len() {
            return Err(MultimodalError::new(
                MultimodalErrorKind::MalformedDocument,
                "truncated WAV chunk",
            ));
        }
        if matches!(id, "LIST" | "id3 " | "JUNK" | "bext") {
            removed.push(id.to_owned());
        } else {
            chunks.extend_from_slice(&wav[pos..end]);
        }
        pos = end;
    }
    let mut out = Vec::with_capacity(12 + chunks.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((4 + chunks.len()) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(&chunks);
    let digest = sha256_prefixed(&out);
    Ok(BinaryScrubReport {
        format: "wav".to_owned(),
        removed_segments: removed,
        clean_bytes: out,
        output_sha256: digest,
    })
}

pub fn scrub_mp3_tags(mp3: &[u8]) -> BinaryScrubReport {
    let mut start = 0usize;
    let mut end = mp3.len();
    let mut removed = Vec::new();
    if mp3.len() >= 10 && &mp3[..3] == b"ID3" {
        let size = ((mp3[6] as usize & 0x7F) << 21)
            | ((mp3[7] as usize & 0x7F) << 14)
            | ((mp3[8] as usize & 0x7F) << 7)
            | (mp3[9] as usize & 0x7F);
        let candidate = 10usize.saturating_add(size).min(mp3.len());
        start = candidate;
        removed.push("ID3v2".to_owned());
    }
    if end.saturating_sub(start) >= 128 && &mp3[end - 128..end - 125] == b"TAG" {
        end -= 128;
        removed.push("ID3v1".to_owned());
    }
    let clean_bytes = mp3[start..end].to_vec();
    let output_sha256 = sha256_prefixed(&clean_bytes);
    BinaryScrubReport {
        format: "mp3".to_owned(),
        removed_segments: removed,
        clean_bytes,
        output_sha256,
    }
}

pub fn scrub_edf_ecg_header(edf: &[u8]) -> Result<BinaryScrubReport, MultimodalError> {
    if edf.len() < 256 || !edf[..8].starts_with(b"0       ") {
        return Err(MultimodalError::new(
            MultimodalErrorKind::MalformedDocument,
            "invalid EDF/ECG header (expected >= 256 bytes starting with version '0       ')",
        ));
    }
    let mut out = edf.to_vec();
    out[8..88].fill(b' ');
    out[8..8 + 20].copy_from_slice(b"X X X [REDACTED:EDF]");
    out[88..168].fill(b' ');
    out[88..88 + 22].copy_from_slice(b"Startdate X X X X     ");
    let digest = sha256_prefixed(&out);
    Ok(BinaryScrubReport {
        format: "edf_ecg".to_owned(),
        removed_segments: vec![
            "edf.local_patient_identification".to_owned(),
            "edf.local_recording_identification".to_owned(),
        ],
        clean_bytes: out,
        output_sha256: digest,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalOcrResult {
    pub available: bool,
    pub extracted_text: String,
    pub reason: String,
}

/// Invokes a local `tesseract` binary strictly via `argv` stdin/stdout without shell expansion.
pub fn run_sanitized_local_tesseract(
    image_bytes: &[u8],
    binary_override: Option<&Path>,
) -> LocalOcrResult {
    if image_bytes.len() > 16 * 1024 * 1024 || image_bytes.len() < 8 {
        return LocalOcrResult {
            available: false,
            extracted_text: String::new(),
            reason: "image_bounds_invalid".to_owned(),
        };
    }
    let bin = binary_override
        .and_then(|p| p.to_str())
        .filter(|s| !s.trim().is_empty() && !s.starts_with('-'))
        .unwrap_or("tesseract");
    let mut child = match Command::new(bin)
        .args(["stdin", "stdout", "--psm", "6", "-l", "eng"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => {
            return LocalOcrResult {
                available: false,
                extracted_text: String::new(),
                reason: "tesseract_binary_unavailable".to_owned(),
            };
        }
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(image_bytes);
    }
    match child.wait_with_output() {
        Ok(out) if out.status.success() => LocalOcrResult {
            available: true,
            extracted_text: String::from_utf8_lossy(&out.stdout).trim().to_owned(),
            reason: "ok".to_owned(),
        },
        _ => LocalOcrResult {
            available: false,
            extracted_text: String::new(),
            reason: "tesseract_execution_failed".to_owned(),
        },
    }
}
