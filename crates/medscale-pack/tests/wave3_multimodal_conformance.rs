//! Wave 3 conformance & adversarial safety tests for `medscale_pack::{multimodal_v4, dicom_v4}`.

use std::collections::BTreeMap;
use std::path::Path;

use medscale_pack::dicom_v4::{
    DicomDeidentifyOptions, PixelBlackoutRect, deidentify_dicom_part10, remap_dicom_uid,
};
use medscale_pack::multimodal_v4::{
    ArchiveMemberMeta, ArchiveSafetyDecision, ArchiveSafetyPolicy, ArchiveSafetyReason,
    MultimodalErrorKind, evaluate_archive_members, extract_html_text, extract_markdown_text,
    extract_rtf_text, preflight_and_scrub_pdf, read_zip_entries_checked, refuse_outlook_msg_gpl3,
    run_sanitized_local_tesseract, scrub_chatlog_jsonl, scrub_edf_ecg_header, scrub_eml,
    scrub_ical, scrub_jpeg_metadata, scrub_mp3_tags, scrub_ooxml_zip, scrub_png_metadata,
    scrub_rtf_info_and_text, scrub_subtitles_vtt_srt, scrub_tabular_csv, scrub_vcard,
    scrub_wav_metadata, write_stored_zip,
};
use serde_json::Value;

fn synth_scrub(s: &str) -> String {
    s.replace("Jane Example", "[PERSON]")
        .replace("Jane", "[PERSON]")
        .replace("Example", "[PERSON]")
        .replace("jane@example.org", "[EMAIL]")
        .replace("555-0199", "[PHONE]")
        .replace("1980-01-02", "[DATE]")
}

fn vec_json() -> Value {
    let raw = include_str!("vectors/wave3_multimodal_dicom_v4.json");
    serde_json::from_str(raw).expect("wave3 multimodal JSON vector parses")
}

#[test]
fn conformance_multimodal_extractors_archive_ooxml_and_pdf() {
    let v = vec_json();

    let html_sample = "<html><body><h1>Discharge Note</h1><p>Patient: Jane Example, DOB 1980-01-02.</p><script>alert(1)</script></body></html>";
    assert_eq!(
        extract_html_text(html_sample),
        v["html"]["text"].as_str().expect("html text")
    );

    let md_sample = "# Follow-up\n\nSynthetic note for **Alex Rivera** with A1c 6.4%.\n";
    assert_eq!(
        extract_markdown_text(md_sample),
        v["markdown"]["text"].as_str().expect("markdown text")
    );

    let csv_sample = "patient_name,dob,mrn,glucose_mg_dl\nJane Example,1980-01-02,MRN-123456,118\n";
    let csv_res = scrub_tabular_csv(csv_sample, &synth_scrub);
    assert_eq!(csv_res.columns.len(), 4);
    assert!(csv_res.scrubbed_csv.contains("[PERSON]"));
    assert!(csv_res.scrubbed_csv.contains("[DATE_OF_BIRTH]"));
    assert!(csv_res.scrubbed_csv.contains("ID_NUM_"));
    assert!(csv_res.scrubbed_csv.contains(",118"));

    let chat_sample =
        "{\"speaker\":\"Dr. Sam Example\",\"content\":\"Synthetic consult note; call 555-0199\"}\n";
    let chat_out = scrub_chatlog_jsonl(chat_sample, &synth_scrub);
    assert!(chat_out.contains("USERNAME_"));
    assert!(chat_out.contains("[PHONE]"));

    let safety = evaluate_archive_members(
        &[
            ArchiveMemberMeta {
                path: "../../etc/passwd".to_owned(),
                compressed_size: 10,
                uncompressed_size: 20,
                kind: "file".to_owned(),
                link_target: None,
            },
            ArchiveMemberMeta {
                path: "normal.txt".to_owned(),
                compressed_size: 10,
                uncompressed_size: 2000,
                kind: "file".to_owned(),
                link_target: None,
            },
        ],
        &ArchiveSafetyPolicy::default(),
    );
    assert_eq!(safety.decision, ArchiveSafetyDecision::Reject);
    assert!(safety.reasons.contains(&ArchiveSafetyReason::PathTraversal));
    assert!(
        safety
            .reasons
            .contains(&ArchiveSafetyReason::ExpansionRatio)
    );

    // OOXML build -> inspect residual -> scrub -> re-read ZIP clean.
    let mut parts = BTreeMap::new();
    parts.insert("[Content_Types].xml".to_owned(), b"<Types/>".to_vec());
    parts.insert(
        "docProps/core.xml".to_owned(),
        b"<coreProperties><creator>Jane Example</creator></coreProperties>".to_vec(),
    );
    parts.insert(
        "word/document.xml".to_owned(),
        b"<w:document><w:body><w:p><w:r><w:t>Jane Example glucose 120 mg/dL</w:t></w:r></w:p></w:body></w:document>".to_vec(),
    );
    let ooxml_bytes = write_stored_zip(&parts);
    let scrubbed = scrub_ooxml_zip(&ooxml_bytes, &synth_scrub).expect("ooxml scrub");
    assert!(
        scrubbed
            .residual_findings
            .iter()
            .any(|f| f.category == "core_properties")
    );
    assert!(scrubbed.post_scrub_findings.is_empty());
    let (clean_entries, _) =
        read_zip_entries_checked(&scrubbed.clean_zip_bytes, &ArchiveSafetyPolicy::default())
            .expect("clean zip re-reads");
    assert!(!clean_entries.contains_key("docProps/core.xml"));
    let doc_text = String::from_utf8_lossy(&clean_entries["word/document.xml"]);
    assert!(doc_text.contains("[PERSON] glucose 120 mg/dL"));
    assert!(!doc_text.contains("Jane"));

    // PDF preflight + XMP scrub.
    let pdf_raw = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n4 0 obj\n(Jane Example)\n<x:xmpmeta>author=Jane Example</x:xmpmeta>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n";
    let pdf_res = preflight_and_scrub_pdf(pdf_raw, &synth_scrub).expect("pdf preflight");
    assert_eq!(
        pdf_res.status,
        v["pdf_geometry"]["status"].as_str().unwrap()
    );
    assert_eq!(pdf_res.declared_page_count, 1);
    assert!(pdf_res.extracted_text.contains("Jane Example"));
    let pdf_out = String::from_utf8_lossy(&pdf_res.scrubbed_pdf_bytes);
    assert!(!pdf_out.contains("Jane"));
}

#[test]
fn adversarial_multimodal_media_and_gpl_quarantine() {
    // GPL-3.0 .msg quarantine
    let ole2 = b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1synthetic_msg";
    let err = refuse_outlook_msg_gpl3("sample.msg", ole2).expect_err(".msg must refuse");
    assert_eq!(err.kind, MultimodalErrorKind::BlockedExternalRights);

    // Encrypted PDF refusal
    let enc_err = preflight_and_scrub_pdf(b"%PDF-1.6\n<< /Encrypt 1 0 R >>", &synth_scrub)
        .expect_err("encrypted pdf must refuse");
    assert_eq!(enc_err.kind, MultimodalErrorKind::EncryptedUnsupported);

    // ZIP bomb / path traversal refusal on binary ZIP
    let mut bad_parts = BTreeMap::new();
    bad_parts.insert("../../evil.txt".to_owned(), b"boom".to_vec());
    let bad_zip = write_stored_zip(&bad_parts);
    let zip_err = read_zip_entries_checked(&bad_zip, &ArchiveSafetyPolicy::default())
        .expect_err("traversal zip must fail");
    assert_eq!(zip_err.kind, MultimodalErrorKind::ArchiveBombOrTraversal);

    // RTF, EML, vCard, iCal, VTT
    let rtf = "{\\rtf1{\\info{\\author Jane Example}}\\par Hello Jane Example}";
    assert_eq!(extract_rtf_text(rtf), "Hello Jane Example");
    assert!(!scrub_rtf_info_and_text(rtf, &synth_scrub).contains("\\author"));

    let eml = "From: jane@example.org\r\nSubject: Jane Example note\r\n\r\nCall 555-0199.";
    let eml_res = scrub_eml(eml, &synth_scrub);
    assert!(!eml_res.scrubbed_eml.contains("jane@example.org"));
    assert!(eml_res.scrubbed_eml.contains("[PHONE]"));

    let vcard = "BEGIN:VCARD\nFN:Jane Example\nTEL:555-0199\nEND:VCARD";
    assert!(!scrub_vcard(vcard).contains("Jane"));
    let ical =
        "BEGIN:VEVENT\nATTENDEE:mailto:jane@example.org\nSUMMARY:Visit Jane Example\nEND:VEVENT";
    assert!(!scrub_ical(ical, &synth_scrub).contains("jane@example.org"));

    let vtt = "WEBVTT\n\n00:00:01.000 --> 00:00:03.000\n<v Jane Example>Call 555-0199";
    let vtt_out = scrub_subtitles_vtt_srt(vtt, &synth_scrub);
    assert!(vtt_out.contains("<v SPEAKER>Call [PHONE]"));

    // PNG chunk strip (`tEXt` removed, `IHDR` + `IEND` kept)
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(&4u32.to_be_bytes());
    png.extend_from_slice(b"tEXt");
    png.extend_from_slice(b"Name");
    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(b"IEND");
    png.extend_from_slice(&0u32.to_be_bytes());
    let png_rep = scrub_png_metadata(&png).expect("png scrub");
    assert_eq!(png_rep.removed_segments, vec!["tEXt".to_owned()]);
    assert!(!png_rep.clean_bytes.windows(4).any(|w| w == b"tEXt"));

    // JPEG APP1 strip
    let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x06, b'E', b'x', b'i', b'f'];
    jpeg.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x04, 0x01, 0x02, 0xFF, 0xD9]);
    let jpg_rep = scrub_jpeg_metadata(&jpeg).expect("jpeg scrub");
    assert_eq!(jpg_rep.removed_segments, vec!["0xFFE1".to_owned()]);

    // WAV LIST strip & MP3 ID3 strip & EDF+ scrub
    let mut wav = b"RIFF".to_vec();
    let list_chunk = b"LIST\x04\x00\x00\x00INFO";
    let data_chunk = b"data\x02\x00\x00\x00\x01\x02";
    wav.extend_from_slice(&((4 + list_chunk.len() + data_chunk.len()) as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(list_chunk);
    wav.extend_from_slice(data_chunk);
    let wav_rep = scrub_wav_metadata(&wav).expect("wav scrub");
    assert_eq!(wav_rep.removed_segments, vec!["LIST".to_owned()]);

    let mut mp3 = Vec::new();
    mp3.extend_from_slice(b"ID3\x04\x00\x00\x00\x00\x00\x04");
    mp3.extend_from_slice(b"JUNK");
    mp3.extend_from_slice(b"\xFF\xFB\x90\x00AUDIO");
    mp3.extend_from_slice(b"TAG");
    mp3.extend_from_slice(&[b'X'; 125]);
    let mp3_rep = scrub_mp3_tags(&mp3);
    assert_eq!(mp3_rep.removed_segments, vec!["ID3v2", "ID3v1"]);
    assert_eq!(mp3_rep.clean_bytes, b"\xFF\xFB\x90\x00AUDIO");

    let mut edf = vec![b' '; 256];
    edf[..8].copy_from_slice(b"0       ");
    edf[8..20].copy_from_slice(b"Jane Example");
    let edf_rep = scrub_edf_ecg_header(&edf).expect("edf scrub");
    assert!(!String::from_utf8_lossy(&edf_rep.clean_bytes).contains("Jane"));

    // Sanitized local Tesseract gracefully reports unavailable on missing binary
    let ocr = run_sanitized_local_tesseract(&png, Some(Path::new("nonexistent_tesseract_bin_v4")));
    assert!(!ocr.available);
}

fn push_evr_short(buf: &mut Vec<u8>, g: u16, e: u16, vr: &[u8; 2], mut val: Vec<u8>) {
    if val.len() % 2 == 1 {
        val.push(b' ');
    }
    buf.extend_from_slice(&g.to_le_bytes());
    buf.extend_from_slice(&e.to_le_bytes());
    buf.extend_from_slice(vr);
    buf.extend_from_slice(&(val.len() as u16).to_le_bytes());
    buf.extend_from_slice(&val);
}

fn push_evr_long(buf: &mut Vec<u8>, g: u16, e: u16, vr: &[u8; 2], val: &[u8]) {
    buf.extend_from_slice(&g.to_le_bytes());
    buf.extend_from_slice(&e.to_le_bytes());
    buf.extend_from_slice(vr);
    buf.extend_from_slice(&[0, 0]);
    buf.extend_from_slice(&(val.len() as u32).to_le_bytes());
    buf.extend_from_slice(val);
}

#[test]
fn native_dicom_part10_ps315_uid_sr_and_pixel_blackout() {
    let mut dcm = vec![0u8; 128];
    dcm.extend_from_slice(b"DICM");
    // Transfer Syntax Explicit VR LE
    push_evr_short(
        &mut dcm,
        0x0002,
        0x0010,
        b"UI",
        b"1.2.840.10008.1.2.1".to_vec(),
    );
    // Patient & Study tags + UID
    push_evr_short(&mut dcm, 0x0008, 0x0018, b"UI", b"1.2.840.999.1".to_vec());
    push_evr_short(&mut dcm, 0x0008, 0x0080, b"LO", b"Riyadh General".to_vec());
    push_evr_short(&mut dcm, 0x0010, 0x0010, b"PN", b"Example^Jane".to_vec());
    push_evr_short(&mut dcm, 0x0010, 0x0020, b"LO", b"MRN-998877".to_vec());
    push_evr_short(&mut dcm, 0x0010, 0x0030, b"DA", b"19800102".to_vec());
    push_evr_short(&mut dcm, 0x0010, 0x1040, b"LO", b"123 Main St".to_vec());
    push_evr_short(&mut dcm, 0x0020, 0x000D, b"UI", b"1.2.840.999.2".to_vec());
    // Uncompressed 4x4 8-bit 1-channel image
    push_evr_short(&mut dcm, 0x0028, 0x0002, b"US", 1u16.to_le_bytes().to_vec());
    push_evr_short(&mut dcm, 0x0028, 0x0010, b"US", 4u16.to_le_bytes().to_vec());
    push_evr_short(&mut dcm, 0x0028, 0x0011, b"US", 4u16.to_le_bytes().to_vec());
    push_evr_short(&mut dcm, 0x0028, 0x0100, b"US", 8u16.to_le_bytes().to_vec());

    // SR ContentSequence (0040,A730) with one Item containing TextValue (0040,A160)
    let mut item_bytes = Vec::new();
    push_evr_long(
        &mut item_bytes,
        0x0040,
        0xA160,
        b"UT",
        b"Report for Jane Example.",
    );
    let mut sq_bytes = Vec::new();
    sq_bytes.extend_from_slice(&0xFFFEu16.to_le_bytes());
    sq_bytes.extend_from_slice(&0xE000u16.to_le_bytes());
    sq_bytes.extend_from_slice(&(item_bytes.len() as u32).to_le_bytes());
    sq_bytes.extend_from_slice(&item_bytes);
    push_evr_long(&mut dcm, 0x0040, 0xA730, b"SQ", &sq_bytes);

    let pixels = vec![0xAAu8; 16];
    push_evr_long(&mut dcm, 0x7FE0, 0x0010, b"OB", &pixels);

    let opts = DicomDeidentifyOptions {
        uid_hmac_secret: "wave3-test-key".to_owned(),
        date_shift_days: 7,
        blackout_rects: vec![PixelBlackoutRect {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        }],
    };
    let rep = deidentify_dicom_part10(&dcm, &opts, &synth_scrub).expect("dicom scrub");
    assert!(rep.residual_clean);
    assert_eq!(rep.uids_remapped, 2);
    assert_eq!(rep.sr_items_scrubbed, 1);
    assert_eq!(rep.blackout_regions_applied, 1);

    let remapped = remap_dicom_uid(b"wave3-test-key", "1.2.840.999.2");
    assert!(remapped.starts_with("2.25."));
    assert_eq!(
        remapped,
        remap_dicom_uid(b"wave3-test-key", "1.2.840.999.2")
    );

    let out_lossy = String::from_utf8_lossy(&rep.scrubbed_bytes);
    assert!(!out_lossy.contains("Example^Jane"));
    assert!(!out_lossy.contains("123 Main St"));
    assert!(!out_lossy.contains("Riyadh General"));
    assert!(out_lossy.contains("19800109"));
    assert!(out_lossy.contains(&remapped));
    assert!(out_lossy.contains("Report for [PERSON]."));

    // Pixel data at end of scrubbed_bytes should have zeros in (1..3, 1..3) and 0xAA at (0,0)
    let pix_tail = &rep.scrubbed_bytes[rep.scrubbed_bytes.len() - 16..];
    assert_eq!(pix_tail[0], 0xAA);
    assert_eq!(pix_tail[4 + 1], 0x00);
    assert_eq!(pix_tail[2 * 4 + 2], 0x00);
}
