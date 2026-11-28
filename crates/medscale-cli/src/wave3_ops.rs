//! OpenMed v4.0.0 Wave 3 CLI surface (`multimodal`, `dicom`, `interop`, `eval`,
//! `batch` JSONL streaming, opt-in stdio MCP server & loopback-only `serve`).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;

use clap::{Args, Subcommand};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::openmed_wave2::{ClinicalV4AnalyzeRequest, PrivacyV4DeidentifyRequest};
use medscale_contracts::openmed_wave3::{
    DicomV4DeidentifyRequest, EvalV4RunRequest, InteropV4ProcessRequest, McpV4PolicyCheckRequest,
    MultimodalV4ProcessRequest,
};
use medscale_core::CliSession;
use serde_json::{Value, json};

use super::{fail_json, print_json_or_debug};

fn cli_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
    let message = match err {
        AuthorityError::InvalidArgument { message } => message.clone(),
        other => format!("{other:?}"),
    };
    let code = match err {
        AuthorityError::InvalidArgument { .. } => "invalid",
        AuthorityError::Unauthorized
        | AuthorityError::SessionDenied
        | AuthorityError::SessionRequired => "denied",
        _ => "internal",
    };
    fail_json(code, message, json)
}

fn read_text_or_file(
    text: Option<String>,
    file: Option<PathBuf>,
    json: bool,
) -> anyhow::Result<String> {
    match (text, file) {
        (Some(t), None) => Ok(t),
        (None, Some(p)) => std::fs::read_to_string(&p)
            .map_err(|e| fail_json("invalid", format!("cannot read file: {}", e.kind()), json)),
        _ => Err(fail_json(
            "invalid",
            "provide exactly one of --text or --file".to_owned(),
            json,
        )),
    }
}

fn read_json_file(path: &PathBuf, label: &str, json: bool) -> anyhow::Result<Value> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        fail_json(
            "invalid",
            format!("cannot read {label}: {}", e.kind()),
            json,
        )
    })?;
    serde_json::from_str(&raw)
        .map_err(|e| fail_json("invalid", format!("invalid {label} JSON: {e}"), json))
}

fn encode_b64(data: &[u8]) -> String {
    const TBL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        out.push(TBL[b0 >> 2] as char);
        out.push(TBL[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        if chunk.len() > 1 {
            out.push(TBL[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TBL[b2 & 0x3F] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[derive(Debug, Subcommand)]
pub enum MultimodalCmd {
    /// Extract, scrub & verify metadata residuals for HTML/MD/RTF/EML/CSV/ChatLog/OOXML/PDF/PNG/JPEG/WAV/MP3/EDF.
    Process {
        #[arg(long)]
        format: String,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(long)]
        policy: Option<String>,
        #[arg(long, default_value_t = false)]
        no_scrub: bool,
        #[arg(long)]
        json: bool,
    },
    /// Inspect an archive member manifest JSON file for traversal/bomb violations.
    CheckArchive {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum DicomCmd {
    /// Native DICOM Part 10 + PS3.15 tag scrub, 2.25.<u128> UID remap, SR redaction & pixel blackout.
    Deidentify {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        secret: Option<String>,
        #[arg(long, default_value_t = 0)]
        date_shift_days: i64,
        #[arg(long)]
        policy: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum InteropCmd {
    /// Run a Wave 3 interop pipeline (`fhir_deidentify`, `fhir_integrity`, `smart_audit`, `hl7v2_redact`, `cda_redact`, `x12_redact`, `omop_project`, `bridge_check`).
    Run {
        #[arg(long)]
        operation: String,
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(long)]
        text: Option<String>,
        #[arg(long, default_value_t = 0)]
        date_shift_days: i64,
        #[arg(long)]
        workflow_id: Option<String>,
        #[arg(long)]
        required_scope: Vec<String>,
        #[arg(long)]
        declared_scope: Vec<String>,
        #[arg(long)]
        bridge: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
#[allow(clippy::enum_variant_names)]
pub enum EvalCmd {
    /// Check whether a credentialed DUA dataset path sits safely outside the repository root.
    DuaCheck {
        #[arg(long)]
        dataset: String,
        #[arg(long)]
        path: String,
        #[arg(long, default_value_t = false)]
        network_fetch: bool,
        #[arg(long)]
        json: bool,
    },
    /// Check restricted terminology license boundary (rejects SNOMED/CPT/full UMLS/LOINC bundles).
    TerminologyCheck {
        #[arg(long)]
        name: String,
        #[arg(long)]
        json: bool,
    },
    /// Compute train/eval overlap leakage count over a JSON `{ "train": [...], "eval": [...] }` file.
    LeakageCheck {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Args)]
pub struct BatchArgs {
    /// JSONL file path (`-` for stdin), each object containing `text` (and optional `policy`).
    #[arg(long, default_value = "-")]
    pub input: String,
    #[arg(long, default_value = "hipaa_safe_harbor")]
    pub policy: String,
    #[arg(long, default_value_t = 1000)]
    pub max_lines: usize,
}

#[derive(Debug, Args)]
pub struct McpArgs {
    #[arg(long, default_value_t = false)]
    pub list_tools: bool,
    #[arg(long)]
    pub check_endpoint: Option<String>,
    #[arg(long)]
    pub resolved_ip: Vec<String>,
    #[arg(long, default_value_t = false)]
    pub allow_loopback: bool,
    #[arg(long, default_value_t = false)]
    pub stdio: bool,
    #[arg(long, default_value_t = 64)]
    pub max_requests: usize,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Must bind a loopback IP (`127.0.0.1:<port>` or `[::1]:<port>`); non-loopback is refused.
    #[arg(long, default_value = "127.0.0.1:0")]
    pub bind: String,
    #[arg(long, default_value_t = 1)]
    pub max_requests: usize,
    #[arg(long)]
    pub json: bool,
}

pub fn run_multimodal(cmd: MultimodalCmd) -> anyhow::Result<()> {
    let mut session = CliSession::connect_wave3_studio("ephemeral-multimodal")
        .map_err(|e| cli_fail(&e, false))?;
    match cmd {
        MultimodalCmd::Process {
            format,
            text,
            file,
            policy,
            no_scrub,
            json,
        } => {
            let binary_fmts = [
                "ooxml_zip",
                "docx",
                "xlsx",
                "pptx",
                "epub",
                "pdf",
                "png",
                "jpeg",
                "wav",
                "mp3",
                "edf",
                "msg",
            ];
            let is_bin = binary_fmts.contains(&format.to_ascii_lowercase().as_str());
            let (raw_text, b64) = if is_bin {
                let path = file.ok_or_else(|| {
                    fail_json("invalid", "binary format requires --file".to_owned(), json)
                })?;
                let bytes = std::fs::read(&path).map_err(|e| {
                    fail_json("invalid", format!("cannot read file: {}", e.kind()), json)
                })?;
                (None, Some(encode_b64(&bytes)))
            } else {
                (Some(read_text_or_file(text, file, json)?), None)
            };
            let resp = session
                .multimodal_v4_process(MultimodalV4ProcessRequest {
                    format,
                    text: raw_text,
                    base64_payload: b64,
                    archive_members: Vec::new(),
                    scrub: !no_scrub,
                    policy_name: policy,
                })
                .map_err(|e| cli_fail(&e, json))?;
            if json {
                return print_json_or_debug(&resp, true);
            }
            println!(
                "format={} status={} sha256={}",
                resp.format, resp.status, resp.output_sha256
            );
            println!("{}", resp.scrubbed_text);
            Ok(())
        }
        MultimodalCmd::CheckArchive { file, json } => {
            let val = read_json_file(&file, "archive members", json)?;
            let members = serde_json::from_value(val).map_err(|e| {
                fail_json(
                    "invalid",
                    format!("invalid archive members array: {e}"),
                    json,
                )
            })?;
            let resp = session
                .multimodal_v4_process(MultimodalV4ProcessRequest {
                    format: "archive_meta".to_owned(),
                    archive_members: members,
                    ..MultimodalV4ProcessRequest::default()
                })
                .map_err(|e| cli_fail(&e, json))?;
            if json {
                return print_json_or_debug(&resp, true);
            }
            println!(
                "archive_status={} reasons={:?}",
                resp.status, resp.removed_segments
            );
            Ok(())
        }
    }
}

pub fn run_dicom(cmd: DicomCmd) -> anyhow::Result<()> {
    let mut session =
        CliSession::connect_wave3_studio("ephemeral-dicom").map_err(|e| cli_fail(&e, false))?;
    match cmd {
        DicomCmd::Deidentify {
            file,
            secret,
            date_shift_days,
            policy,
            json,
        } => {
            let bytes = std::fs::read(&file).map_err(|e| {
                fail_json("invalid", format!("cannot read DICOM: {}", e.kind()), json)
            })?;
            let resp = session
                .dicom_v4_deidentify(DicomV4DeidentifyRequest {
                    base64_payload: encode_b64(&bytes),
                    uid_hmac_secret: secret,
                    date_shift_days,
                    blackout_rects: Vec::new(),
                    policy_name: policy,
                })
                .map_err(|e| cli_fail(&e, json))?;
            if json {
                return print_json_or_debug(&resp, true);
            }
            println!(
                "transfer_syntax={} residual_clean={} uids_remapped={} sr_scrubbed={}",
                resp.transfer_syntax_uid,
                resp.residual_clean,
                resp.uids_remapped,
                resp.sr_items_scrubbed
            );
            Ok(())
        }
    }
}

pub fn run_interop(cmd: InteropCmd) -> anyhow::Result<()> {
    let mut session =
        CliSession::connect_wave3_studio("ephemeral-interop").map_err(|e| cli_fail(&e, false))?;
    match cmd {
        InteropCmd::Run {
            operation,
            file,
            text,
            date_shift_days,
            workflow_id,
            required_scope,
            declared_scope,
            bridge,
            json,
        } => {
            let mut payload = Value::Null;
            let mut raw_text = text;
            if let Some(p) = file {
                let content = std::fs::read_to_string(&p).map_err(|e| {
                    fail_json("invalid", format!("cannot read input: {}", e.kind()), json)
                })?;
                if content.trim_start().starts_with('{') {
                    payload = serde_json::from_str(&content)
                        .map_err(|e| fail_json("invalid", format!("invalid JSON: {e}"), json))?;
                } else {
                    raw_text = Some(content);
                }
            }
            let resp = session
                .interop_v4_process(InteropV4ProcessRequest {
                    operation,
                    payload,
                    raw_text,
                    date_shift_days,
                    reference_year: Some(2026),
                    workflow_id,
                    required_scopes: required_scope,
                    declared_scopes: declared_scope,
                    bridge_name: bridge,
                    policy_name: None,
                })
                .map_err(|e| cli_fail(&e, json))?;
            if json {
                return print_json_or_debug(&resp, true);
            }
            println!(
                "operation={} status={} redactions={}",
                resp.operation, resp.status, resp.redaction_count
            );
            Ok(())
        }
    }
}

pub fn run_eval(cmd: EvalCmd) -> anyhow::Result<()> {
    let mut session =
        CliSession::connect_wave3_studio("ephemeral-eval").map_err(|e| cli_fail(&e, false))?;
    match cmd {
        EvalCmd::DuaCheck {
            dataset,
            path,
            network_fetch,
            json,
        } => {
            let resp = session
                .eval_v4_run(EvalV4RunRequest {
                    operation: "dua_path_guard".to_owned(),
                    dataset_name: Some(dataset),
                    dataset_path: Some(path),
                    network_fetch,
                    ..EvalV4RunRequest::default()
                })
                .map_err(|e| cli_fail(&e, json))?;
            if json {
                return print_json_or_debug(&resp, true);
            }
            println!("passed={} result={}", resp.passed, resp.result);
            Ok(())
        }
        EvalCmd::TerminologyCheck { name, json } => {
            let resp = session
                .eval_v4_run(EvalV4RunRequest {
                    operation: "terminology_guard".to_owned(),
                    terminology_name: Some(name),
                    ..EvalV4RunRequest::default()
                })
                .map_err(|e| cli_fail(&e, json))?;
            if json {
                return print_json_or_debug(&resp, true);
            }
            println!("passed={} result={}", resp.passed, resp.result);
            Ok(())
        }
        EvalCmd::LeakageCheck { file, json } => {
            let val = read_json_file(&file, "leakage input", json)?;
            let train_texts = val
                .get("train")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            let eval_texts = val
                .get("eval")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            let resp = session
                .eval_v4_run(EvalV4RunRequest {
                    operation: "leakage_check".to_owned(),
                    train_texts,
                    eval_texts,
                    ..EvalV4RunRequest::default()
                })
                .map_err(|e| cli_fail(&e, json))?;
            if json {
                return print_json_or_debug(&resp, true);
            }
            println!("passed={} result={}", resp.passed, resp.result);
            Ok(())
        }
    }
}

pub fn run_batch(args: BatchArgs) -> anyhow::Result<()> {
    let mut session =
        CliSession::connect_wave2_studio("ephemeral-batch").map_err(|e| cli_fail(&e, true))?;
    let reader: Box<dyn BufRead> = if args.input == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        let f = std::fs::File::open(&args.input).map_err(|e| {
            fail_json(
                "invalid",
                format!("cannot open batch input: {}", e.kind()),
                true,
            )
        })?;
        Box::new(BufReader::new(f))
    };
    let mut stdout = std::io::stdout().lock();
    for (idx, line) in reader.lines().take(args.max_lines).enumerate() {
        let raw = line.map_err(|e| fail_json("invalid", e.to_string(), true))?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let item: Value = serde_json::from_str(trimmed)
            .map_err(|e| fail_json("invalid", format!("line {idx}: invalid JSON: {e}"), true))?;
        let text = item
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let policy = item
            .get("policy")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| args.policy.clone());
        let res = session
            .privacy_v4_deidentify(PrivacyV4DeidentifyRequest {
                text,
                policy_name: Some(policy),
                calendar: None,
                date_shift_days: None,
                patient_key: None,
                shift_secret: None,
                max_shift_days: None,
                rtl_isolate: false,
                clinical_protect: true,
                safety_sweep: true,
            })
            .map_err(|e| cli_fail(&e, true))?;
        let out_row = json!({
            "redacted_text": res.redacted_text,
            "policy_name": res.policy_name,
            "base_direction": res.base_direction,
            "span_count": res.detected_spans.len(),
            "receipt_sha256": res.receipt.receipt_sha256,
            "receipt_verified": res.receipt_verified,
        });
        writeln!(stdout, "{}", serde_json::to_string(&out_row)?)?;
    }
    stdout.flush()?;
    Ok(())
}

pub fn run_mcp(args: McpArgs) -> anyhow::Result<()> {
    let mut session =
        CliSession::connect_wave3_studio("ephemeral-mcp").map_err(|e| cli_fail(&e, args.json))?;
    if args.list_tools {
        let resp = session
            .mcp_v4_policy_check(McpV4PolicyCheckRequest {
                operation: "list_tools".to_owned(),
                ..McpV4PolicyCheckRequest::default()
            })
            .map_err(|e| cli_fail(&e, args.json))?;
        return print_json_or_debug(&resp.result, args.json);
    }
    if let Some(url) = args.check_endpoint {
        let ips = if args.resolved_ip.is_empty() {
            vec!["127.0.0.1".to_owned()]
        } else {
            args.resolved_ip
        };
        let resp = session
            .mcp_v4_policy_check(McpV4PolicyCheckRequest {
                operation: "validate_endpoint".to_owned(),
                endpoint: Some(url),
                resolved_ips: ips,
                allow_loopback: args.allow_loopback,
                ..McpV4PolicyCheckRequest::default()
            })
            .map_err(|e| cli_fail(&e, args.json))?;
        return print_json_or_debug(&resp, args.json);
    }
    if args.stdio {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout().lock();
        for line in stdin.lock().lines().take(args.max_requests) {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let req: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
            let id = req.get("id").cloned().unwrap_or(Value::Null);
            let method = req.get("method").and_then(Value::as_str).unwrap_or("");
            let reply = match method {
                "initialize" => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {"protocolVersion": "2025-03-26", "serverInfo": {"name": "medscale-mcp", "version": env!("CARGO_PKG_VERSION")}}
                }),
                "tools/list" => {
                    let resp = session
                        .mcp_v4_policy_check(McpV4PolicyCheckRequest {
                            operation: "list_tools".to_owned(),
                            ..McpV4PolicyCheckRequest::default()
                        })
                        .map_err(|e| cli_fail(&e, true))?;
                    json!({"jsonrpc": "2.0", "id": id, "result": resp.result})
                }
                "tools/call" => {
                    let params = req.get("params").cloned().unwrap_or(Value::Null);
                    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
                    let out = match name {
                        "medscale.privacy.deidentify" => {
                            let text = arguments
                                .get("text")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_owned();
                            let r = session
                                .privacy_v4_deidentify(PrivacyV4DeidentifyRequest {
                                    text,
                                    policy_name: arguments
                                        .get("policy_name")
                                        .and_then(Value::as_str)
                                        .map(str::to_owned),
                                    calendar: None,
                                    date_shift_days: None,
                                    patient_key: None,
                                    shift_secret: None,
                                    max_shift_days: None,
                                    rtl_isolate: false,
                                    clinical_protect: true,
                                    safety_sweep: true,
                                })
                                .map_err(|e| cli_fail(&e, true))?;
                            json!(r)
                        }
                        "medscale.clinical.analyze" => {
                            let text = arguments
                                .get("text")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_owned();
                            let r = session
                                .clinical_v4_analyze(ClinicalV4AnalyzeRequest {
                                    text,
                                    ..ClinicalV4AnalyzeRequest::default()
                                })
                                .map_err(|e| cli_fail(&e, true))?;
                            json!(r)
                        }
                        other => json!({"error": format!("unknown MCP tool `{other}`")}),
                    };
                    json!({"jsonrpc": "2.0", "id": id, "result": out})
                }
                other => {
                    json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": format!("unsupported method {other}")}})
                }
            };
            writeln!(stdout, "{reply}")?;
        }
        stdout.flush()?;
    }
    Ok(())
}

pub fn run_serve(args: ServeArgs) -> anyhow::Result<()> {
    let addr: SocketAddr = args.bind.parse().map_err(|e| {
        fail_json(
            "invalid",
            format!("invalid bind SocketAddr: {e}"),
            args.json,
        )
    })?;
    if !addr.ip().is_loopback() {
        return Err(fail_json(
            "denied",
            "local-first serve strictly refuses non-loopback bind addresses".to_owned(),
            args.json,
        ));
    }
    let listener = TcpListener::bind(addr)
        .map_err(|e| fail_json("internal", format!("bind failed: {e}"), args.json))?;
    let local = listener.local_addr()?;
    if args.json {
        println!(
            "{}",
            json!({"listening": local.to_string(), "loopback_only": true})
        );
    } else {
        println!(
            "MedScale local loopback service listening on http://{local} (max_requests={})",
            args.max_requests
        );
    }

    let mut session =
        CliSession::connect_wave3_studio("ephemeral-serve").map_err(|e| cli_fail(&e, args.json))?;

    for _ in 0..args.max_requests {
        let Ok((mut stream, _)) = listener.accept() else {
            break;
        };
        let mut buf = [0u8; 32 * 1024];
        let n = stream.read(&mut buf).unwrap_or(0);
        let (status, json_body) = if n == buf.len() {
            (413, json!({"error": "request_too_large"}))
        } else {
            let req_str = String::from_utf8_lossy(&buf[..n]);
            let first_line = req_str.lines().next().unwrap_or("");
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            let method = parts.first().copied().unwrap_or("");
            let path = parts.get(1).copied().unwrap_or("/");
            let body_str = req_str.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");

            match (method, path) {
                ("GET", "/healthz") => (200, json!({"status": "ok", "loopback_only": true})),
                ("POST", "/v1/privacy/deidentify") => {
                    let parsed: Result<PrivacyV4DeidentifyRequest, _> =
                        serde_json::from_str(body_str);
                    match parsed {
                        Ok(req) => match session.privacy_v4_deidentify(req) {
                            Ok(res) => (200, json!(res)),
                            Err(e) => (400, json!({"error": format!("{e:?}")})),
                        },
                        Err(e) => (400, json!({"error": format!("invalid request: {e}")})),
                    }
                }
                ("POST", "/v1/clinical/analyze") => {
                    let parsed: Result<ClinicalV4AnalyzeRequest, _> =
                        serde_json::from_str(body_str);
                    match parsed {
                        Ok(req) => match session.clinical_v4_analyze(req) {
                            Ok(res) => (200, json!(res)),
                            Err(e) => (400, json!({"error": format!("{e:?}")})),
                        },
                        Err(e) => (400, json!({"error": format!("invalid request: {e}")})),
                    }
                }
                _ => (404, json!({"error": "not_found"})),
            }
        };
        let reason = match status {
            200 => "OK",
            400 => "Bad Request",
            413 => "Payload Too Large",
            _ => "Not Found",
        };
        let body_bytes = serde_json::to_vec(&json_body).unwrap_or_default();
        let header = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body_bytes.len()
        );
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(&body_bytes);
        let _ = stream.flush();
    }
    Ok(())
}
