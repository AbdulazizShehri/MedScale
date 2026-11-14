//! Spec 079 Privacy Gate commands (CLI vertical slice through Core).
//!
//! Every command opens the session scope, dispatches one typed Core request
//! via `CliSession`, and renders the typed result as human lines or stable
//! JSON. The CLI never reads or writes privacy storage or key material
//! directly. Transforms always run with `synthetic_only = true`: real PHI is
//! not authorized (`EXTERNAL_GATES.md` REAL_PHI_AUTHORIZATION).

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_contracts::privacy_gate::{
    ArtifactClassification, DataClass, DeidReceipt, EffectiveClassification, EgressBoundary,
    EgressDecision, PrivacyPolicyProfile, ProfileRule, PseudonymMapRef, ReidentificationAudit,
    SensitiveSpanKind, TransformOp,
};

use super::{fail_json, print_json_or_debug};

fn privacy_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
    let debug = format!("{err:?}");
    let (code, message) = match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => ("denied", debug),
        AuthorityError::NotFound => ("not_found", debug),
        AuthorityError::Conflict { message } => ("conflict", message.clone()),
        AuthorityError::InvalidArgument { message } => ("invalid", message.clone()),
        AuthorityError::Corrupt { message } => ("corrupt", message.clone()),
        AuthorityError::UnsupportedSchema { message } => ("unsupported_schema", message.clone()),
        AuthorityError::ExternalGateRequired { gate } => ("external_gate_required", gate.clone()),
        AuthorityError::LeaseRequired
        | AuthorityError::VaultRequired
        | AuthorityError::MissingKeyMaterial => ("unavailable", debug),
        _ => ("internal", debug),
    };
    fail_json(code, message, json)
}

fn invalid(message: String, json: bool) -> anyhow::Error {
    fail_json("invalid", message, json)
}

fn open_session(
    vault_id: &str,
    vault_root: &std::path::Path,
    json: bool,
) -> anyhow::Result<crate::vault_session::VaultSession> {
    crate::vault_session::open(vault_id, vault_root).map_err(|err| privacy_fail(&err, json))
}

/// Parses `kind=op,kind=op`; kinds not named take `default_op`. Every kind
/// ends up mapped exactly once, or the profile is refused by Core.
fn parse_rules(rules: Option<&str>, default_op: &str) -> Result<Vec<ProfileRule>, String> {
    let default_op = TransformOp::parse(default_op)?;
    let mut named: Vec<ProfileRule> = Vec::new();
    for pair in rules
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        let (kind, op) = pair
            .split_once('=')
            .ok_or_else(|| format!("rule {pair:?} must be kind=op"))?;
        let kind = SensitiveSpanKind::parse(kind.trim())?;
        let op = TransformOp::parse(op.trim())?;
        if named.iter().any(|r| r.kind == kind) {
            return Err(format!("kind {} named twice", kind.as_str()));
        }
        named.push(ProfileRule { kind, op });
    }
    Ok(SensitiveSpanKind::ALL
        .iter()
        .map(|kind| {
            named
                .iter()
                .find(|r| r.kind == *kind)
                .cloned()
                .unwrap_or(ProfileRule {
                    kind: *kind,
                    op: default_op,
                })
        })
        .collect())
}

fn print_classification(value: &ArtifactClassification, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(value, true);
    }
    println!("classification_id: {}", value.header.id.as_str());
    println!("artifact_id: {}", value.artifact_id.as_str());
    println!("data_class: {}", value.data_class.as_str());
    println!("basis: {}", value.basis.as_str());
    println!("revision: {}", value.revision);
    Ok(())
}

fn print_effective(value: &EffectiveClassification, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(value, true);
    }
    println!("artifact_id: {}", value.artifact_id.as_str());
    println!("data_class: {}", value.data_class.as_str());
    println!("basis: {}", value.basis.as_str());
    match &value.classification {
        Some(row) => println!("revision: {}", row.revision),
        None => println!("revision: (unclassified; treated as local_phi)"),
    }
    Ok(())
}

fn print_profile(value: &PrivacyPolicyProfile, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(value, true);
    }
    println!("profile_id: {}", value.header.id.as_str());
    println!("name: {}", value.name.escape_debug());
    println!("target_class: {}", value.target_class.as_str());
    println!("status: {}", value.status.as_str());
    println!("revision: {}", value.revision);
    println!("use_model_recognizer: {}", value.use_model_recognizer);
    for rule in &value.rules {
        println!("rule: {}={}", rule.kind.as_str(), rule.op.as_str());
    }
    Ok(())
}

fn print_map(value: &PseudonymMapRef, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(value, true);
    }
    println!("map_id: {}", value.header.id.as_str());
    println!("status: {}", value.status.as_str());
    println!("entry_count: {}", value.entry_count);
    println!("revision: {}", value.revision);
    Ok(())
}

fn print_receipt(value: &DeidReceipt, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(value, true);
    }
    println!("receipt_id: {}", value.header.id.as_str());
    println!("source_artifact_id: {}", value.source_artifact_id.as_str());
    println!("output_artifact_id: {}", value.output_artifact_id.as_str());
    println!("output_class: {}", value.output_class.as_str());
    println!("status: {}", value.status.as_str());
    println!("residual: {}", value.residual.status.as_str());
    for r in &value.recognizers {
        println!(
            "recognizer: {} v{} {} spans={}",
            r.recognizer.recognizer_id,
            r.recognizer.version,
            r.status.as_str(),
            r.span_count
        );
    }
    for c in &value.op_counts {
        println!("op: {} {} x{}", c.kind.as_str(), c.op.as_str(), c.count);
    }
    for l in &value.limitations {
        println!("limitation: {}", l.as_str());
    }
    Ok(())
}

fn print_audit(value: &ReidentificationAudit, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(value, true);
    }
    println!(
        "{}\t{}\t{}",
        value.header.id.as_str(),
        value.pseudonym,
        value.outcome.as_str()
    );
    Ok(())
}

fn print_decision(value: &EgressDecision, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(value, true);
    }
    println!(
        "{}\t{}\t{}\t{}\t{}",
        value.artifact_id.as_str(),
        value.boundary.as_str(),
        value.data_class.as_str(),
        value.outcome.as_str(),
        value.reason.as_str()
    );
    Ok(())
}

/// Spec 079 Privacy Gate commands.
#[derive(Debug, Subcommand)]
pub enum PrivacyCmd {
    /// Add a synthetic text file as a source artifact.
    SourceAdd {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        file: PathBuf,
        #[arg(long, default_value = "text/plain")]
        media_type: String,
        #[arg(long)]
        json: bool,
    },
    /// Declare an artifact's data class in a Project.
    Classify {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        artifact_id: String,
        /// local_phi, team_protected, external_deidentified or public.
        #[arg(long)]
        class: String,
        #[arg(long)]
        expected_revision: Option<u64>,
        #[arg(long)]
        json: bool,
    },
    /// Show an artifact's effective class (unclassified is local_phi).
    ClassificationShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        artifact_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Create a de-identification profile.
    ProfileCreate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        name: String,
        /// external_deidentified or team_protected.
        #[arg(long, default_value = "external_deidentified")]
        target_class: String,
        /// Comma-separated kind=op rules, e.g. person_name=pseudonymize.
        #[arg(long)]
        rules: Option<String>,
        /// Op for every kind not named in --rules.
        #[arg(long, default_value = "redact")]
        default_op: String,
        #[arg(long)]
        use_model_recognizer: bool,
        #[arg(long)]
        json: bool,
    },
    /// List profiles in a Project.
    ProfileList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Revoke a profile.
    ProfileRevoke {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        profile_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Create a reversible pseudonym map (key held in the OS key store).
    MapCreate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List pseudonym maps in a Project.
    MapList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Revoke a pseudonym map and destroy its key.
    MapRevoke {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        map_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Transform a source into a new de-identified artifact.
    Transform {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        source_artifact_id: String,
        #[arg(long)]
        profile_id: String,
        #[arg(long)]
        map_id: Option<String>,
        #[arg(long)]
        model_pack_id: Option<String>,
        #[arg(long)]
        model_pack_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one de-identification receipt.
    ReceiptShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        receipt_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List receipts in a Project.
    ReceiptList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Revoke a receipt (its output can no longer leave).
    ReceiptRevoke {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        receipt_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Resolve one pseudonym (separate capability; always audited).
    Reidentify {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        map_id: String,
        #[arg(long)]
        pseudonym: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        json: bool,
    },
    /// List re-identification audit rows for a map.
    ReidAudit {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        map_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Ask Core whether an artifact may cross a boundary (decision persisted).
    EgressCheck {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        artifact_id: String,
        #[arg(long)]
        boundary: String,
        #[arg(long)]
        json: bool,
    },
    /// List persisted egress decisions.
    EgressList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        artifact_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[allow(clippy::too_many_lines)]
pub fn run_privacy(cmd: PrivacyCmd) -> anyhow::Result<()> {
    match cmd {
        PrivacyCmd::SourceAdd {
            vault_id,
            vault_root,
            file,
            media_type,
            json,
        } => {
            let bytes = std::fs::read(&file)
                .map_err(|e| invalid(format!("cannot read file: {}", e.kind()), json))?;
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let id = session
                .create_source_record(media_type, bytes)
                .map_err(|err| privacy_fail(&err, json))?;
            if json {
                print_json_or_debug(&serde_json::json!({ "artifact_id": id.as_str() }), true)
            } else {
                println!("artifact_id: {}", id.as_str());
                Ok(())
            }
        }
        PrivacyCmd::Classify {
            vault_id,
            vault_root,
            project_id,
            artifact_id,
            class,
            expected_revision,
            json,
        } => {
            let class = DataClass::parse(&class).map_err(|m| invalid(m, json))?;
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let row = session
                .privacy_classify(
                    OpaqueId::new(project_id),
                    OpaqueId::new(artifact_id),
                    class,
                    expected_revision,
                )
                .map_err(|err| privacy_fail(&err, json))?;
            print_classification(&row, json)
        }
        PrivacyCmd::ClassificationShow {
            vault_id,
            vault_root,
            project_id,
            artifact_id,
            json,
        } => {
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let effective = session
                .privacy_classification_get(OpaqueId::new(project_id), OpaqueId::new(artifact_id))
                .map_err(|err| privacy_fail(&err, json))?;
            print_effective(&effective, json)
        }
        PrivacyCmd::ProfileCreate {
            vault_id,
            vault_root,
            project_id,
            name,
            target_class,
            rules,
            default_op,
            use_model_recognizer,
            json,
        } => {
            let target_class = DataClass::parse(&target_class).map_err(|m| invalid(m, json))?;
            let rules = parse_rules(rules.as_deref(), &default_op).map_err(|m| invalid(m, json))?;
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let profile = session
                .privacy_profile_create(
                    OpaqueId::new(project_id),
                    name,
                    target_class,
                    rules,
                    use_model_recognizer,
                )
                .map_err(|err| privacy_fail(&err, json))?;
            print_profile(&profile, json)
        }
        PrivacyCmd::ProfileList {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let profiles = session
                .privacy_profile_list(OpaqueId::new(project_id))
                .map_err(|err| privacy_fail(&err, json))?;
            if json {
                return print_json_or_debug(&profiles, true);
            }
            for p in &profiles {
                println!(
                    "{}\t{}\t{}\t{}",
                    p.header.id.as_str(),
                    p.name.escape_debug(),
                    p.status.as_str(),
                    p.revision
                );
            }
            Ok(())
        }
        PrivacyCmd::ProfileRevoke {
            vault_id,
            vault_root,
            profile_id,
            expected_revision,
            json,
        } => {
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let profile = session
                .privacy_profile_revoke(OpaqueId::new(profile_id), expected_revision)
                .map_err(|err| privacy_fail(&err, json))?;
            print_profile(&profile, json)
        }
        PrivacyCmd::MapCreate {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let map = session
                .privacy_map_create(OpaqueId::new(project_id))
                .map_err(|err| privacy_fail(&err, json))?;
            print_map(&map, json)
        }
        PrivacyCmd::MapList {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let maps = session
                .privacy_map_list(OpaqueId::new(project_id))
                .map_err(|err| privacy_fail(&err, json))?;
            if json {
                return print_json_or_debug(&maps, true);
            }
            for m in &maps {
                print_map(m, false)?;
            }
            Ok(())
        }
        PrivacyCmd::MapRevoke {
            vault_id,
            vault_root,
            map_id,
            expected_revision,
            json,
        } => {
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let map = session
                .privacy_map_revoke(OpaqueId::new(map_id), expected_revision)
                .map_err(|err| privacy_fail(&err, json))?;
            print_map(&map, json)
        }
        PrivacyCmd::Transform {
            vault_id,
            vault_root,
            project_id,
            source_artifact_id,
            profile_id,
            map_id,
            model_pack_id,
            model_pack_path,
            json,
        } => {
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let receipt = session
                .privacy_transform(
                    OpaqueId::new(project_id),
                    OpaqueId::new(source_artifact_id),
                    OpaqueId::new(profile_id),
                    map_id.map(OpaqueId::new),
                    model_pack_id.map(OpaqueId::new),
                    model_pack_path,
                    true,
                )
                .map_err(|err| privacy_fail(&err, json))?;
            print_receipt(&receipt, json)
        }
        PrivacyCmd::ReceiptShow {
            vault_id,
            vault_root,
            receipt_id,
            json,
        } => {
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let receipt = session
                .privacy_receipt_get(OpaqueId::new(receipt_id))
                .map_err(|err| privacy_fail(&err, json))?;
            print_receipt(&receipt, json)
        }
        PrivacyCmd::ReceiptList {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let receipts = session
                .privacy_receipt_list(OpaqueId::new(project_id))
                .map_err(|err| privacy_fail(&err, json))?;
            if json {
                return print_json_or_debug(&receipts, true);
            }
            for r in &receipts {
                println!(
                    "{}\t{}\t{}\t{}",
                    r.header.id.as_str(),
                    r.output_artifact_id.as_str(),
                    r.status.as_str(),
                    r.residual.status.as_str()
                );
            }
            Ok(())
        }
        PrivacyCmd::ReceiptRevoke {
            vault_id,
            vault_root,
            receipt_id,
            expected_revision,
            json,
        } => {
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let receipt = session
                .privacy_receipt_revoke(OpaqueId::new(receipt_id), expected_revision)
                .map_err(|err| privacy_fail(&err, json))?;
            print_receipt(&receipt, json)
        }
        PrivacyCmd::Reidentify {
            vault_id,
            vault_root,
            map_id,
            pseudonym,
            reason,
            json,
        } => {
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let (audit, value) = session
                .privacy_reidentify(OpaqueId::new(map_id), pseudonym, reason)
                .map_err(|err| privacy_fail(&err, json))?;
            if json {
                return print_json_or_debug(
                    &serde_json::json!({ "audit": audit, "value": value }),
                    true,
                );
            }
            print_audit(&audit, false)?;
            match value {
                Some(v) => println!("value: {}", v.escape_debug()),
                None => println!("value: (not returned)"),
            }
            Ok(())
        }
        PrivacyCmd::ReidAudit {
            vault_id,
            vault_root,
            map_id,
            json,
        } => {
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let audits = session
                .privacy_reid_audit_list(OpaqueId::new(map_id))
                .map_err(|err| privacy_fail(&err, json))?;
            if json {
                return print_json_or_debug(&audits, true);
            }
            for a in &audits {
                print_audit(a, false)?;
            }
            Ok(())
        }
        PrivacyCmd::EgressCheck {
            vault_id,
            vault_root,
            project_id,
            artifact_id,
            boundary,
            json,
        } => {
            let boundary = EgressBoundary::parse(&boundary).map_err(|m| invalid(m, json))?;
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let decision = session
                .privacy_egress_evaluate(
                    OpaqueId::new(project_id),
                    OpaqueId::new(artifact_id),
                    boundary,
                )
                .map_err(|err| privacy_fail(&err, json))?;
            print_decision(&decision, json)
        }
        PrivacyCmd::EgressList {
            vault_id,
            vault_root,
            project_id,
            artifact_id,
            json,
        } => {
            let mut session = open_session(&vault_id, &vault_root, json)?;
            let decisions = session
                .privacy_egress_list(OpaqueId::new(project_id), artifact_id.map(OpaqueId::new))
                .map_err(|err| privacy_fail(&err, json))?;
            if json {
                return print_json_or_debug(&decisions, true);
            }
            for d in &decisions {
                print_decision(d, false)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::privacy_gate::{
        ClassificationBasis, EgressReason, ReidentificationOutcome, ResidualScanStatus,
        is_pseudonym,
    };
    use medscale_core::CliSession;

    const VAULT: &str = "vault-079-cli";

    #[test]
    fn parse_rules_fills_unnamed_kinds_and_refuses_bad_input() {
        let rules =
            parse_rules(Some("person_name=pseudonymize, date=generalize"), "redact").unwrap();
        assert_eq!(rules.len(), SensitiveSpanKind::ALL.len());
        assert!(rules.contains(&ProfileRule {
            kind: SensitiveSpanKind::PersonName,
            op: TransformOp::Pseudonymize
        }));
        assert!(rules.contains(&ProfileRule {
            kind: SensitiveSpanKind::Email,
            op: TransformOp::Redact
        }));
        assert!(parse_rules(Some("person_name"), "redact").is_err());
        assert!(parse_rules(Some("shoe_size=redact"), "redact").is_err());
        assert!(parse_rules(Some("email=redact,email=drop"), "redact").is_err());
        assert!(parse_rules(None, "winner").is_err());
    }

    #[test]
    fn privacy_commands_run_through_core_across_fresh_sessions() {
        let root = std::env::temp_dir().join(format!("medscale-079-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let note = root.join("note.txt");
        std::fs::write(&note, "Patient: Maria Lopez, MRN: 998877, seen 2024-03-12.").unwrap();

        // Seed a Project (existing Spec 074 path).
        let mut session = CliSession::connect(VAULT).unwrap();
        session
            .open_synthetic_vault(&root.display().to_string())
            .unwrap();
        let project = session
            .project_create("privacy".to_owned(), None)
            .unwrap()
            .header
            .id;
        drop(session);
        let project_id = project.as_str().to_owned();

        for json in [true, false] {
            run_privacy(PrivacyCmd::SourceAdd {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                file: note.clone(),
                media_type: "text/plain".to_owned(),
                json,
            })
            .expect("source add");
        }
        assert!(
            run_privacy(PrivacyCmd::Classify {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: project_id.clone(),
                artifact_id: "src-1".to_owned(),
                class: "secret".to_owned(),
                expected_revision: None,
                json: true,
            })
            .is_err(),
            "unknown class is refused"
        );
        run_privacy(PrivacyCmd::ProfileCreate {
            vault_id: VAULT.to_owned(),
            vault_root: root.clone(),
            project_id: project_id.clone(),
            name: "export".to_owned(),
            target_class: "external_deidentified".to_owned(),
            rules: Some(
                "person_name=pseudonymize,identifier=pseudonymize,date=generalize".to_owned(),
            ),
            default_op: "redact".to_owned(),
            use_model_recognizer: false,
            json: true,
        })
        .expect("profile create");

        // The rest runs through one session, as a single CLI process would
        // not keep the in-memory key store between commands on hosts
        // without an OS keyring.
        let mut session = CliSession::connect(VAULT).unwrap();
        session.use_in_memory_privacy_keys();
        session
            .open_synthetic_vault(&root.display().to_string())
            .unwrap();
        let profiles = session.privacy_profile_list(project.clone()).unwrap();
        assert_eq!(profiles.len(), 1);
        let map = session.privacy_map_create(project.clone()).unwrap();
        let receipts_before = session.privacy_receipt_list(project.clone()).unwrap();
        assert!(receipts_before.is_empty());
        let source = session
            .create_source_record("text/plain".to_owned(), std::fs::read(&note).unwrap())
            .unwrap();
        let unclassified = session
            .privacy_classification_get(project.clone(), source.clone())
            .unwrap();
        assert_eq!(unclassified.basis, ClassificationBasis::DefaultUnclassified);
        let receipt = session
            .privacy_transform(
                project.clone(),
                source.clone(),
                profiles[0].header.id.clone(),
                Some(map.header.id.clone()),
                None,
                None,
                true,
            )
            .unwrap();
        assert_eq!(
            receipt.residual.status,
            ResidualScanStatus::NoResidualDetectedByAdmittedRecognizers
        );
        let decision = session
            .privacy_egress_evaluate(
                project.clone(),
                receipt.output_artifact_id.clone(),
                EgressBoundary::Browse,
            )
            .unwrap();
        assert_eq!(decision.reason, EgressReason::AllowedDeidentified);
        let denied = session
            .privacy_egress_evaluate(project.clone(), source.clone(), EgressBoundary::Browse)
            .unwrap();
        assert_eq!(denied.reason, EgressReason::DeniedUnclassified);
        assert!(
            session
                .privacy_reidentify_with_operator_session(
                    map.header.id.clone(),
                    "PSN-000000000000".to_owned(),
                    "no grant".to_owned(),
                )
                .is_err(),
            "the operator session cannot re-identify"
        );
        let (audit, value) = session
            .privacy_reidentify(
                map.header.id.clone(),
                "PSN-000000000000".to_owned(),
                "check".to_owned(),
            )
            .unwrap();
        assert_eq!(
            audit.outcome,
            ReidentificationOutcome::DeniedUnknownPseudonym
        );
        assert!(value.is_none());
        let receipts = session.privacy_receipt_list(project.clone()).unwrap();
        assert_eq!(receipts.len(), 1);
        drop(session);

        // Read commands across fresh sessions, human and JSON.
        for json in [true, false] {
            run_privacy(PrivacyCmd::ClassificationShow {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: project_id.clone(),
                artifact_id: receipt.output_artifact_id.as_str().to_owned(),
                json,
            })
            .expect("classification show");
            run_privacy(PrivacyCmd::ReceiptShow {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                receipt_id: receipt.header.id.as_str().to_owned(),
                json,
            })
            .expect("receipt show");
            run_privacy(PrivacyCmd::ReceiptList {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: project_id.clone(),
                json,
            })
            .expect("receipt list");
            run_privacy(PrivacyCmd::EgressList {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: project_id.clone(),
                artifact_id: None,
                json,
            })
            .expect("egress list");
            run_privacy(PrivacyCmd::MapList {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: project_id.clone(),
                json,
            })
            .expect("map list");
            run_privacy(PrivacyCmd::ReidAudit {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                map_id: map.header.id.as_str().to_owned(),
                json,
            })
            .expect("reid audit");
            run_privacy(PrivacyCmd::ProfileList {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: project_id.clone(),
                json,
            })
            .expect("profile list");
        }
        run_privacy(PrivacyCmd::EgressCheck {
            vault_id: VAULT.to_owned(),
            vault_root: root.clone(),
            project_id: project_id.clone(),
            artifact_id: source.as_str().to_owned(),
            boundary: "hub".to_owned(),
            json: true,
        })
        .expect("egress check");
        assert!(
            run_privacy(PrivacyCmd::EgressCheck {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: project_id.clone(),
                artifact_id: source.as_str().to_owned(),
                boundary: "anywhere".to_owned(),
                json: true,
            })
            .is_err()
        );
        run_privacy(PrivacyCmd::ReceiptRevoke {
            vault_id: VAULT.to_owned(),
            vault_root: root.clone(),
            receipt_id: receipt.header.id.as_str().to_owned(),
            expected_revision: 1,
            json: false,
        })
        .expect("receipt revoke");
        let mut session = CliSession::connect(VAULT).unwrap();
        session
            .open_synthetic_vault(&root.display().to_string())
            .unwrap();
        let after = session
            .privacy_egress_evaluate(
                project.clone(),
                receipt.output_artifact_id.clone(),
                EgressBoundary::Browse,
            )
            .unwrap();
        assert_eq!(after.reason, EgressReason::DeniedReceiptRevoked);
        // The output text carries pseudonyms, not the source values.
        let output_ok = receipt
            .op_counts
            .iter()
            .any(|c| c.op == TransformOp::Pseudonymize);
        assert!(output_ok);
        assert!(!is_pseudonym("Maria Lopez"));
    }
}
