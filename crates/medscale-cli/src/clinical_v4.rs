//! MedScale clinical NLP, medication reconciliation, guarded dosing,
//! and compositional UCUM unit conversion CLI commands (via Core `CliSession`).

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::openmed_wave2::{
    ClaimDto, ClinicalV4AnalyzeRequest, DoseCheckDto, DoseRangeDto, MedMentionDto,
    UnitsV4ConvertRequest,
};
use medscale_core::CliSession;

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

fn read_json_vec<T: serde::de::DeserializeOwned>(
    path: &PathBuf,
    label: &str,
    json: bool,
) -> anyhow::Result<Vec<T>> {
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

#[derive(Debug, Subcommand)]
pub enum ClinicalCmd {
    /// Analyze a clinical note for LOINC sections, ConText/negation, Sigs, relations/events, SDOH & specialty templates.
    AnalyzeNote {
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(long)]
        specialty: Option<String>,
        #[arg(long)]
        sig: Vec<String>,
        #[arg(long)]
        nli_entailment: Option<f64>,
        #[arg(long)]
        nli_contradiction: Option<f64>,
        #[arg(long)]
        nli_neutral: Option<f64>,
        #[arg(long)]
        claims_file: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Parse one or more medication sig strings into structured dose/unit/route/frequency/PRN fields.
    ParseSig {
        #[arg(long, required = true)]
        sig: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Reconcile a longitudinal list of medication mentions (`MedMentionDto[]` JSON file).
    ReconcileMeds {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Guarded clinician-review dose-range advisory check over `DoseCheckDto[]` & `DoseRangeDto[]` JSON files.
    CheckDose {
        #[arg(long)]
        doses: PathBuf,
        #[arg(long)]
        ranges: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum UnitsCmd {
    /// Parse a UCUM expression compositionally (dimensions, canonical unit, affine flag).
    Parse {
        #[arg(long)]
        unit: String,
        #[arg(long)]
        json: bool,
    },
    /// Convert a quantity between commensurable UCUM units (or mass<->molar with `--analyte` / `--molar-mass`).
    Convert {
        #[arg(long)]
        value: f64,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        analyte: Option<String>,
        #[arg(long)]
        molar_mass: Option<f64>,
        #[arg(long)]
        json: bool,
    },
}

pub fn run_clinical(cmd: ClinicalCmd) -> anyhow::Result<()> {
    let mut session = CliSession::connect_wave2_studio("ephemeral-clinical-v4")
        .map_err(|e| cli_fail(&e, false))?;
    match cmd {
        ClinicalCmd::AnalyzeNote {
            text,
            file,
            specialty,
            sig,
            nli_entailment,
            nli_contradiction,
            nli_neutral,
            claims_file,
            json,
        } => {
            let note = match (text, file) {
                (Some(t), None) => t,
                (None, Some(p)) => std::fs::read_to_string(&p).map_err(|e| {
                    fail_json("invalid", format!("cannot read note: {}", e.kind()), json)
                })?,
                _ => {
                    return Err(fail_json(
                        "invalid",
                        "provide exactly one of --text or --file".to_owned(),
                        json,
                    ));
                }
            };
            let nli_scores = match (nli_entailment, nli_contradiction, nli_neutral) {
                (Some(e), Some(c), Some(n)) => Some((e, c, n)),
                (None, None, None) => None,
                _ => {
                    return Err(fail_json(
                        "invalid",
                        "provide all three of --nli-entailment, --nli-contradiction, --nli-neutral"
                            .to_owned(),
                        json,
                    ));
                }
            };
            let claims: Vec<ClaimDto> = if let Some(p) = claims_file {
                read_json_vec(&p, "claims file", json)?
            } else {
                Vec::new()
            };
            let resp = session
                .clinical_v4_analyze(ClinicalV4AnalyzeRequest {
                    text: note,
                    specialty,
                    sig_lines: sig,
                    nli_scores,
                    claims,
                    ..ClinicalV4AnalyzeRequest::default()
                })
                .map_err(|e| cli_fail(&e, json))?;
            if json {
                return print_json_or_debug(&resp, true);
            }
            println!(
                "sections={} negation_scopes={} assertions={} sigs={} relations={} events={} sdoh={} conflicts={}",
                resp.sections.len(),
                resp.negation_scopes.len(),
                resp.context_assertions.len(),
                resp.parsed_sigs.len(),
                resp.relations.len(),
                resp.events.len(),
                resp.sdoh.len(),
                resp.claim_conflict_count
            );
            println!("disclaimer: {}", resp.disclaimer);
            Ok(())
        }
        ClinicalCmd::ParseSig { sig, json } => {
            let resp = session
                .clinical_v4_analyze(ClinicalV4AnalyzeRequest {
                    sig_lines: sig,
                    ..ClinicalV4AnalyzeRequest::default()
                })
                .map_err(|e| cli_fail(&e, json))?;
            if json {
                return print_json_or_debug(&resp.parsed_sigs, true);
            }
            for s in &resp.parsed_sigs {
                println!(
                    "raw={:?} dose={:?} unit={:?} route={:?} freq_per_day={:?} prn={}",
                    s.raw, s.dose, s.unit, s.route, s.frequency_per_day, s.as_needed
                );
            }
            Ok(())
        }
        ClinicalCmd::ReconcileMeds { file, json } => {
            let med_mentions: Vec<MedMentionDto> = read_json_vec(&file, "med mentions file", json)?;
            let resp = session
                .clinical_v4_analyze(ClinicalV4AnalyzeRequest {
                    med_mentions,
                    ..ClinicalV4AnalyzeRequest::default()
                })
                .map_err(|e| cli_fail(&e, json))?;
            if json {
                return print_json_or_debug(&resp.reconciled_meds, true);
            }
            for m in &resp.reconciled_meds {
                println!(
                    "{}\tstatus={}\tdose={:?}\tconflict={}",
                    m.ingredient, m.current_status, m.current_dose, m.has_conflict
                );
            }
            Ok(())
        }
        ClinicalCmd::CheckDose {
            doses,
            ranges,
            json,
        } => {
            let dose_checks: Vec<DoseCheckDto> = read_json_vec(&doses, "doses file", json)?;
            let dose_ranges: Vec<DoseRangeDto> = read_json_vec(&ranges, "ranges file", json)?;
            let resp = session
                .clinical_v4_analyze(ClinicalV4AnalyzeRequest {
                    dose_checks,
                    dose_ranges,
                    ..ClinicalV4AnalyzeRequest::default()
                })
                .map_err(|e| cli_fail(&e, json))?;
            if json {
                return print_json_or_debug(&resp.dose_advisories, true);
            }
            for a in &resp.dose_advisories {
                println!(
                    "{} ({}): {} | review_required={} | autonomous={}",
                    a.drug, a.route, a.status, a.requires_clinician_review, a.autonomous_decision
                );
            }
            Ok(())
        }
    }
}

pub fn run_units(cmd: UnitsCmd) -> anyhow::Result<()> {
    let mut session =
        CliSession::connect_wave2_studio("ephemeral-units-v4").map_err(|e| cli_fail(&e, false))?;
    match cmd {
        UnitsCmd::Parse { unit, json } => {
            let resp = session
                .units_v4_convert(UnitsV4ConvertRequest {
                    value: None,
                    from_unit: unit,
                    to_unit: None,
                    analyte: None,
                    molar_mass_g_per_mol: None,
                })
                .map_err(|e| cli_fail(&e, json))?;
            if json {
                return print_json_or_debug(&resp, true);
            }
            println!("{}", serde_json::to_string_pretty(&resp.parsed)?);
            Ok(())
        }
        UnitsCmd::Convert {
            value,
            from,
            to,
            analyte,
            molar_mass,
            json,
        } => {
            let resp = session
                .units_v4_convert(UnitsV4ConvertRequest {
                    value: Some(value),
                    from_unit: from,
                    to_unit: Some(to),
                    analyte,
                    molar_mass_g_per_mol: molar_mass,
                })
                .map_err(|e| cli_fail(&e, json))?;
            if json {
                return print_json_or_debug(&resp, true);
            }
            println!("{}", serde_json::to_string_pretty(&resp)?);
            Ok(())
        }
    }
}
