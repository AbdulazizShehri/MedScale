//! MedScale CLI — facade-only authority surface (Spec 006).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use medscale_contracts::doctor::PrivacyProof;
use medscale_contracts::fixture_ui::{FixtureUiSurface, FixtureUiViewModel};
use medscale_contracts::workflow::CliJsonError;
use medscale_core::{
    CliSession, CoreFacade, HostIpcClient, HostIpcServer, JourneyConfig, build_doctor_report,
    endpoint_for_vault_root, privacy_proof_artifact_present, run_minimum_lovable_journey,
};
use serde::Serialize;

mod analytics;
mod audio;
mod browse;
mod collaboration;
mod compute;
mod data_source;
mod extensions;
mod federation;
mod hub;
mod huddles;
mod institutional;
mod knowledge;
mod medagent;
mod model_fleet;
mod privacy_gate;
mod project;
mod r_workspace;
mod research_packs;

#[derive(Debug, Parser)]
#[command(
    name = "medscale",
    version,
    about = "MedScale · evidence-first local clinical intelligence CLI",
    long_about = "MedScale · evidence-first local clinical intelligence CLI\n\nHuman-readable output stays restrained and explicit about authority. Use --json on supported read commands for stable machine-readable output."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Operator health / privacy / vault axes (no secrets).
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        vault_root: Option<PathBuf>,
    },
    /// Encrypted or synthetic vault operations via Core Host.
    Vault {
        #[command(subcommand)]
        action: VaultCmd,
    },
    /// Ingest synthetic FHIR R4 JSON through the authority facade.
    Ingest {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        file: PathBuf,
        #[arg(long, default_value = "synthetic")]
        mode: String,
    },
    /// Deterministic subject timeline (Spec 004).
    Timeline {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        json: bool,
    },
    /// Narrow LLM-free Brief.
    Brief {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        json: bool,
    },
    /// Coverage accounting.
    Coverage {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        json: bool,
    },
    /// Compact product/runtime status without secret-bearing values.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Explain CLI/Desktop contract parity and intentional presentation-only gaps.
    Capabilities {
        #[arg(long)]
        json: bool,
    },
    /// Grouped longitudinal patient reads over trusted presentation contracts.
    Patient {
        #[command(subcommand)]
        action: PatientCmd,
    },
    /// Read-only controlled-action views.
    Actions {
        #[command(subcommand)]
        action: ActionsCmd,
    },
    /// Read-only disclosure/audit views.
    Audit {
        #[command(subcommand)]
        action: AuditCmd,
    },
    /// FHIR support and interchange honesty.
    Fhir {
        #[command(subcommand)]
        action: FhirCmd,
    },
    /// Emit PRIVACY_PROOF JSON schema (baseline).
    PrivacyProof {
        #[arg(long)]
        json: bool,
    },
    /// Offline Pack v0 operations (Spec 008; local path only).
    Packs {
        #[command(subcommand)]
        action: PacksCmd,
    },
    /// Deterministic fixture UI view-models (no final visual design).
    FixtureUi {
        #[command(subcommand)]
        action: FixtureUiCmd,
    },
    /// Spec 021 minimum lovable trusted workflow (synthetic).
    Journey {
        #[command(subcommand)]
        action: JourneyCmd,
    },
    /// Localhost OS IPC host/client (Spec 024 READY_BASE).
    HostIpc {
        #[command(subcommand)]
        action: HostIpcCmd,
    },
    /// Project workspace (Spec 074; organizational references through Core).
    Project {
        #[command(subcommand)]
        action: project::ProjectCmd,
    },
    /// Experiment workspace (Spec 074; scoped to one Project).
    Experiment {
        #[command(subcommand)]
        action: project::ExperimentCmd,
    },
    /// Collaboration substrate (Spec 076; participant/room/membership through Core).
    Collab {
        #[command(subcommand)]
        action: Box<collaboration::CollabCmd>,
    },
    /// Collaboration substrate (Spec 076; thread/message/task/note/approval/activity through Core).
    CollabWork {
        #[command(subcommand)]
        action: Box<collaboration::CollabWorkCmd>,
    },
    /// MedAgent Workbench (Spec 077; agent identity through Core).
    MedAgent {
        #[command(subcommand)]
        action: Box<medagent::MedAgentCmd>,
    },
    /// Model Fleet + Compare (Spec 078; agent lanes through Core).
    ModelFleet {
        #[command(subcommand)]
        action: Box<model_fleet::ModelFleetCmd>,
    },
    /// Analytics Gate (Spec 082; read-only SQL, cohorts and statistics over
    /// exact snapshots).
    Analytics {
        #[command(subcommand)]
        action: Box<analytics::AnalyticsCmd>,
    },
    /// Knowledge + Research Canvas (Spec 083; lexical retrieval over exact
    /// source revisions, retrieval receipts, and live-reference canvases).
    Knowledge {
        #[command(subcommand)]
        action: Box<knowledge::KnowledgeCmd>,
    },
    /// MedScale Compute (Spec 085; closed job kinds in the MedScale worker,
    /// no arbitrary code, no network).
    Compute {
        #[command(subcommand)]
        action: Box<compute::ComputeCmd>,
    },
    /// R Workspace (Spec 086; exact snapshot staging, external IDE launch,
    /// explicit publication; MedScale does not run R).
    R {
        #[command(subcommand)]
        action: Box<r_workspace::RCmd>,
    },
    /// Community Extensions (Spec 087; signed declarative packs, explicit
    /// publisher trust and grants; no extension code runs).
    Extension {
        #[command(subcommand)]
        action: Box<extensions::ExtensionCmd>,
    },
    /// AudioFlow Advanced huddles (Spec 088; per-act consent, synthetic
    /// labeling, retention and deletion; no speech synthesis).
    Huddle {
        #[command(subcommand)]
        action: Box<huddles::HuddleCmd>,
    },
    /// Research Packs (Spec 089; declarative domain schemas, workflows and
    /// evidence assessments; no code, no capability grants).
    Pack {
        #[command(subcommand)]
        action: Box<research_packs::PackCmd>,
    },
    /// Institutional adapters (Spec 090; explicit optional external writes
    /// with durable effect states; no network transport in this build).
    Adapter {
        #[command(subcommand)]
        action: Box<institutional::AdapterCmd>,
    },
    /// Federation (Spec 091; signed bundles between explicitly trusted
    /// institutions, moved out of band; raw PHI never federates).
    Federation {
        #[command(subcommand)]
        action: Box<federation::FederationCmd>,
    },
    /// MedScale Hub foundation (Spec 084; local-socket IPC only, no network).
    Hub {
        #[command(subcommand)]
        action: Box<hub::HubCmd>,
    },
    /// AudioFlow Foundation (Spec 081; local audio import, capture and
    /// transcript lineage).
    Audio {
        #[command(subcommand)]
        action: Box<audio::AudioCmd>,
    },
    /// Governed Browse (Spec 080; read-only, policy-gated web retrieval).
    Browse {
        #[command(subcommand)]
        action: Box<browse::BrowseCmd>,
    },
    /// Privacy Gate (Spec 079; classification, de-identification, egress).
    Privacy {
        #[command(subcommand)]
        action: Box<privacy_gate::PrivacyCmd>,
    },
    /// Data source fabric (Spec 075; governed sources through Core).
    /// Boxed: the locator/filter/transform DSL variants are large by value.
    DataSource {
        #[command(subcommand)]
        action: Box<data_source::DataSourceCmd>,
    },
    /// Immutable data snapshots (Spec 075; import/refresh/rows through Core).
    Snapshot {
        #[command(subcommand)]
        action: Box<data_source::SnapshotCmd>,
    },
    /// Saved data views (Spec 075; projections through Core).
    DataView {
        #[command(subcommand)]
        action: Box<data_source::DataViewCmd>,
    },
    /// Deterministic data transformations (Spec 075; through Core).
    DataTransform {
        #[command(subcommand)]
        action: Box<data_source::DataTransformCmd>,
    },
    /// Versioned dataset releases (Spec 075; through Core).
    DataRelease {
        #[command(subcommand)]
        action: Box<data_source::DataReleaseCmd>,
    },
}

#[derive(Debug, Subcommand)]
enum PatientCmd {
    /// Emit one combined timeline + Brief + coverage snapshot.
    Show {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        json: bool,
    },
    /// Deterministic subject timeline.
    Timeline {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        json: bool,
    },
    /// Narrow LLM-free Brief.
    Brief {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        json: bool,
    },
    /// Coverage accounting.
    Coverage {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ActionsCmd {
    /// List durable controlled-action outbox entries; UNKNOWN is not retry authority.
    Outbox {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum AuditCmd {
    /// List append-only disclosure records.
    Disclosures {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum FhirCmd {
    /// Print the honest FHIR R4 support matrix; not a full-conformance claim.
    Support {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum VaultCmd {
    Create {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        /// Read passphrase from this environment variable (never argv).
        #[arg(long, default_value = "MEDSCALE_PASSPHRASE")]
        passphrase_env: String,
    },
    Open {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long, default_value = "MEDSCALE_PASSPHRASE")]
        passphrase_env: String,
    },
    OpenSynthetic {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum PacksCmd {
    /// Admit a Pack from a local directory (offline only).
    Install {
        #[arg(long)]
        vault_id: String,
        path: PathBuf,
    },
    /// List admitted packs in the current Core Host session.
    List {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Query a local model catalog snapshot (Spec 103; read-only, offline).
    /// Counts are listing counts: nothing is downloaded, admitted or run.
    Catalog {
        /// JSON Lines manifest snapshot on disk (for example models.jsonl).
        #[arg(long)]
        snapshot: PathBuf,
        /// Exact 40-hex commit the snapshot was taken from.
        #[arg(long)]
        commit: String,
        #[arg(long, default_value = "maziyarpanahi/openmed")]
        repository: String,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        family: Option<String>,
        #[arg(long)]
        language: Option<String>,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        architecture: Option<String>,
        /// cpu_portable | apple_silicon_only | unknown
        #[arg(long)]
        device: Option<String>,
        /// For example DISCOVERABLE or RIGHTS_PENDING.
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        max_params: Option<u64>,
        #[arg(long)]
        max_disk_mb: Option<f64>,
        /// OpenMed size tier, for example Tiny, Small, Base or Large.
        #[arg(long)]
        tier: Option<String>,
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Runnable format variants of a catalog model, best first for this host
    /// (or `--host apple_silicon|other`). Read-only; no network access.
    Variants {
        #[arg(long)]
        snapshot: PathBuf,
        #[arg(long)]
        commit: String,
        #[arg(long, default_value = "maziyarpanahi/openmed")]
        repository: String,
        #[arg(long)]
        repo_id: String,
        #[arg(long)]
        host: Option<String>,
    },
    /// Verify a locally supplied Hugging Face snapshot against its catalog row,
    /// build a signed Pack and admit it (Spec 103; no network access).
    AdmitSnapshot {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        snapshot: PathBuf,
        #[arg(long)]
        commit: String,
        #[arg(long, default_value = "maziyarpanahi/openmed")]
        repository: String,
        /// Directory with metadata.json, config.json, tokenizer.json and the ONNX file.
        #[arg(long)]
        snapshot_dir: PathBuf,
        #[arg(long, default_value = "model.onnx")]
        onnx_file: String,
        #[arg(long, default_value_t = 128)]
        sequence_length: u32,
        /// Where the Pack directory is written.
        #[arg(long)]
        out: PathBuf,
    },
    /// Download the files one catalog row needs from the Hugging Face Hub
    /// (Spec 103; explicit opt-in). Requires --i-consent-to-download and an
    /// approved byte ceiling; only Hub hosts are contacted, every file is
    /// verified against the catalog-pinned commit, nothing else is sent.
    Acquire {
        #[arg(long)]
        snapshot: PathBuf,
        #[arg(long)]
        commit: String,
        #[arg(long, default_value = "maziyarpanahi/openmed")]
        repository: String,
        /// Catalog repository id, for example OpenMed/<model>.
        #[arg(long)]
        repo_id: String,
        #[arg(long, default_value = "model.onnx")]
        onnx_file: String,
        /// Maximum total bytes you approve for this download.
        #[arg(long)]
        approve_bytes: u64,
        /// Explicit consent flag; nothing is downloaded without it.
        #[arg(long)]
        i_consent_to_download: bool,
        /// Snapshot directory to create (metadata, config, tokenizer, ONNX).
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum FixtureUiCmd {
    /// Emit doctor FixtureUiViewModel JSON (synthetic-only; no visual design).
    Doctor {
        #[arg(long)]
        vault_root: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum JourneyCmd {
    /// Run the documented synthetic end-to-end journey via CoreFacade.
    Run {
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long, default_value = "journey-subject")]
        subject: String,
        #[arg(long, default_value = "journey-vault")]
        vault_id: String,
        #[arg(long)]
        backup_dir: PathBuf,
        #[arg(long)]
        restore_dir: PathBuf,
        /// Reject the previewed proposal instead of accepting.
        #[arg(long, default_value_t = false)]
        reject: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum HostIpcCmd {
    /// Serve CoreFacade over localhost OS IPC (holds Spec 016 writer lock).
    Serve {
        #[arg(long)]
        vault_root: PathBuf,
        /// Namespaced endpoint ([A-Za-z0-9_-]+). Default: hash of vault_root.
        #[arg(long)]
        endpoint: Option<String>,
    },
    /// Dispatch one JSON AuthorityRequest file via IPC and print the response.
    Call {
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        request_json: PathBuf,
    },
}

#[derive(Debug, Serialize)]
struct CliStatusSummary {
    product_name: String,
    version: String,
    local_only: bool,
    synthetic_only: bool,
    real_phi_authorized: bool,
    desktop_shell: String,
    network_default_deny: bool,
    private_data_ready: bool,
    release_ready: bool,
    multi_client_release_ready: bool,
    wcag_conformance_claimed: bool,
    missing_release_evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CliPatientSnapshot {
    subject: String,
    timeline: medscale_contracts::presentation::SubjectTimelineV1,
    brief: medscale_contracts::presentation::SubjectBriefV1,
    coverage: medscale_contracts::presentation::SubjectCoverageV1,
    authority_note: String,
}

#[derive(Debug, Clone, Serialize)]
struct CliCapabilityRow {
    surface: &'static str,
    path: &'static str,
    authority: &'static str,
    status: &'static str,
}

/// Stable CLI exit codes (Spec 021).
const EXIT_OK: u8 = 0;
const EXIT_ERROR: u8 = 1;
const EXIT_JSON_ERROR: u8 = 2;

/// Main-thread stack budget.
///
/// Windows reserves 1 MiB for the main thread while clap parsing plus the
/// synchronous dispatch match exceed it in debug builds once the command tree
/// grows (observed stack overflow on plain `medscale status` after the Spec
/// 074 project commands landed; 1 MiB spawned-thread probes of every parse
/// path pass, so the budget need is just above the platform default). Run the
/// CLI body on an explicit 8 MiB thread instead of relying on platform
/// main-stack defaults. The `cli_wire_enum_sizes_stay_stack_safe` test guards
/// the wire types; this guards the platform.
const CLI_STACK_SIZE: usize = 8 << 20;

fn main() -> ExitCode {
    std::thread::Builder::new()
        .name("medscale-main".to_owned())
        .stack_size(CLI_STACK_SIZE)
        .spawn(run_main)
        .expect("spawn main thread")
        .join()
        .unwrap_or_else(|_| ExitCode::from(EXIT_ERROR))
}

fn run_main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::from(EXIT_OK),
        Err(err) => {
            if let Some(json_err) = err.downcast_ref::<JsonCliFailure>() {
                eprintln!("{}", json_err.0);
                ExitCode::from(EXIT_JSON_ERROR)
            } else {
                eprintln!("error: {err:#}");
                ExitCode::from(EXIT_ERROR)
            }
        }
    }
}

#[derive(Debug)]
struct JsonCliFailure(String);

impl std::fmt::Display for JsonCliFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for JsonCliFailure {}

fn fail_json(code: &str, message: impl Into<String>, as_json: bool) -> anyhow::Error {
    let err = CliJsonError::new(code, message);
    if as_json {
        match serde_json::to_string_pretty(&err) {
            Ok(s) => JsonCliFailure(s).into(),
            Err(e) => anyhow::anyhow!("json error encode failed: {e}"),
        }
    } else {
        anyhow::anyhow!("{}: {}", err.code, err.message)
    }
}

fn status_summary(report: &medscale_contracts::doctor::DoctorReport) -> CliStatusSummary {
    CliStatusSummary {
        product_name: report.product_name.clone(),
        version: report.version.clone(),
        local_only: report.local_only,
        synthetic_only: report.synthetic_only,
        real_phi_authorized: report.real_phi_authorized,
        desktop_shell: report.desktop_shell.clone(),
        network_default_deny: report.network_broker.default_deny,
        private_data_ready: report.vault_privacy.private_data_ready,
        release_ready: report.release_qualification.release_ready,
        multi_client_release_ready: report.host_authority.multi_client_release_ready,
        wcag_conformance_claimed: report.accessibility.wcag_conformance_claimed,
        missing_release_evidence: report
            .release_qualification
            .missing_evidence_classes
            .clone(),
    }
}

fn human_heading(surface: &str) -> String {
    format!("MedScale · {surface}")
}

fn capability_rows() -> Vec<CliCapabilityRow> {
    vec![
        CliCapabilityRow {
            surface: "Status / doctor",
            path: "status | doctor",
            authority: "DoctorReport",
            status: "CLI + Desktop",
        },
        CliCapabilityRow {
            surface: "Patient longitudinal",
            path: "patient show|timeline|brief|coverage",
            authority: "trusted presentation contracts",
            status: "CLI + Desktop",
        },
        CliCapabilityRow {
            surface: "Population Insights",
            path: "underlying patient/evidence reads",
            authority: "presentation only; no risk authority",
            status: "Desktop visual aggregation",
        },
        CliCapabilityRow {
            surface: "Controlled actions",
            path: "actions outbox",
            authority: "ListOutbox / EffectState",
            status: "CLI read + Desktop review",
        },
        CliCapabilityRow {
            surface: "Workflow Studio",
            path: "actions outbox",
            authority: "outbox/action contracts",
            status: "Desktop visual composition",
        },
        CliCapabilityRow {
            surface: "Audit",
            path: "audit disclosures",
            authority: "ListDisclosures",
            status: "CLI + Desktop",
        },
        CliCapabilityRow {
            surface: "FHIR",
            path: "fhir support",
            authority: "GetFhirSupportMatrix",
            status: "CLI + Desktop",
        },
        CliCapabilityRow {
            surface: "Packs",
            path: "packs install|list",
            authority: "Pack admission contracts",
            status: "CLI authority path",
        },
        CliCapabilityRow {
            surface: "Host IPC",
            path: "host-ipc serve|call",
            authority: "versioned AuthorityRequest",
            status: "CLI operator path",
        },
    ]
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Doctor { json, vault_root } => {
            let root = vault_root.map(|p| p.display().to_string());
            let report =
                build_doctor_report(root.as_deref(), false, privacy_proof_artifact_present());
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{} doctor", report.product_name);
                println!("version: {}", report.version);
                println!("local_only: {}", report.local_only);
                println!("egress: {}", report.product_runtime_egress);
                println!("synthetic_only: {}", report.synthetic_only);
                println!("real_phi_authorized: {}", report.real_phi_authorized);
                println!(
                    "vault_root: {}",
                    report.vault_root.as_deref().unwrap_or("unset")
                );
                println!("vault_open: {}", report.vault_open);
                println!("sync_risk: {:?}", report.sync_risk);
                println!("filesystem_claim_ok: {}", report.filesystem_claim_ok);
                println!("key_store: {:?}", report.key_store);
                println!("privacy_proof: {:?}", report.privacy_proof_freshness);
                println!("desktop_shell: {}", report.desktop_shell);
                println!("tauri_admitted: {}", report.tauri_admitted);
                println!(
                    "network_broker: present={} default_deny={} allowlist_entries={} live_partner={}",
                    report.network_broker.present,
                    report.network_broker.default_deny,
                    report.network_broker.allowlist_entries,
                    report.network_broker.live_partner_authorized
                );
                println!(
                    "packs_runtime: present={} offline_only={} admitted={} online_download={}",
                    report.packs_runtime.present,
                    report.packs_runtime.offline_only,
                    report.packs_runtime.admitted_count,
                    report.packs_runtime.online_download_authorized
                );
                println!(
                    "mobile: present={} apps_shipped={} 16kb={}",
                    report.mobile.present,
                    report.mobile.apps_shipped,
                    report.mobile.page_size_16kb_claim
                );
                println!(
                    "controlled_actions: present={} outbox_restart_qualified={} nphies={} blind_retry={}",
                    report.controlled_actions.present,
                    report.controlled_actions.outbox_restart_qualified,
                    report.controlled_actions.nphies_authorized,
                    report.controlled_actions.unknown_blind_retry
                );
                println!(
                    "online_packs: present={} authorized={} broker={}",
                    report.online_packs.present,
                    report.online_packs.online_download_authorized,
                    report.online_packs.broker_required
                );
                println!(
                    "mesc_artifact: present={} admitted={} verifier_ready_base={} gate={} required={} integration_status={}",
                    report.mesc_artifact.present,
                    report.mesc_artifact.artifact_admitted,
                    report.mesc_artifact.verifier_ready_base,
                    report.mesc_artifact.gate,
                    report.mesc_artifact.required,
                    report.mesc_artifact.integration_status
                );
                println!(
                    "vault_privacy: ready={} sealed_at_close={} open_work_risk={} page_encrypted={} sqlcipher={} wipe_on_close={} os_keyring_available={} os_keyring_used={} probes_present={} swap_snapshot_honesty={} encrypted_authority_sync_qualified={} residual_open={} pagefile={:?} swap={:?} snapshot={:?} core_dump={:?}",
                    report.vault_privacy.private_data_ready,
                    report.vault_privacy.sealed_at_close,
                    report.vault_privacy.open_work_plaintext_risk,
                    report.vault_privacy.open_work_page_encrypted,
                    report.vault_privacy.sqlcipher_enabled,
                    report.vault_privacy.work_wipe_on_close,
                    report.vault_privacy.os_keyring_available,
                    report.vault_privacy.os_keyring_used,
                    report.vault_privacy.probes_present,
                    report.vault_privacy.swap_snapshot_honesty_present,
                    report.vault_privacy.encrypted_authority_sync_qualified,
                    report.vault_privacy.residual_risk_classes_open.join(","),
                    report.vault_privacy.pagefile_existence,
                    report.vault_privacy.swap_existence,
                    report.vault_privacy.snapshot_existence,
                    report.vault_privacy.core_dump_config_existence
                );
                println!(
                    "host_authority: ready_base={} multi_client_release={} os_ipc={}",
                    report.host_authority.ready_base,
                    report.host_authority.multi_client_release_ready,
                    report.host_authority.os_ipc_qualified
                );
                println!(
                    "record_semantics: ready_base={} precision_time={} amendments={} release_ready={}",
                    report.record_semantics.ready_base,
                    report.record_semantics.precision_aware_time,
                    report.record_semantics.append_only_amendments,
                    report.record_semantics.release_ready
                );
                println!(
                    "fhir_interchange: ready_base={} fhir={} full_conformance={} validator_authority={} release_ready={}",
                    report.fhir_interchange.ready_base,
                    report.fhir_interchange.fhir_version,
                    report.fhir_interchange.full_conformance_claimed,
                    report.fhir_interchange.validator_is_authority,
                    report.fhir_interchange.release_ready
                );
                println!(
                    "fhir_support_matrix: resources={} patient_structural={:?} conformance_claimed={}",
                    report.fhir_support_matrix.resources.len(),
                    report.fhir_support_matrix.status_for("Patient").structural,
                    report.fhir_support_matrix.full_conformance_claimed
                );
                println!(
                    "workflow: ready_base={} release_ready={} disclosure={}",
                    report.workflow.workflow_ready_base,
                    report.workflow.release_ready,
                    report.workflow.disclosure_append_supported
                );
                println!(
                    "release_qualification: prep_ready_base={} release_ready={} locked={} macos_ci={} macos_qualified={} branch_protection={} perf_harness={} sbom_scaffold={} sbom_lock_bound={} release_sbom_qualified={} dry_run_verifier={} migration_recovery={} pkg_upgrade_scaffold={} portable_package={} package_lifecycle={} host_perf_path={} required_checks_synced={} notice_inventory={} rights_license_decision={} missing={}",
                    report.release_qualification.prep_ready_base,
                    report.release_qualification.release_ready,
                    report.release_qualification.locked_builds,
                    report.release_qualification.macos_ci_present,
                    report.release_qualification.macos_qualified,
                    report.release_qualification.branch_protection_configured,
                    report.release_qualification.perf_harness_present,
                    report.release_qualification.sbom_scaffold_present,
                    report.release_qualification.sbom_lock_bound,
                    report.release_qualification.release_sbom_qualified,
                    report
                        .release_qualification
                        .release_dry_run_verifier_present,
                    report.release_qualification.migration_recovery_ready_base,
                    report
                        .release_qualification
                        .package_upgrade_rollback_scaffold_present,
                    report
                        .release_qualification
                        .portable_release_package_qualified,
                    report.release_qualification.package_lifecycle_qualified,
                    report
                        .release_qualification
                        .host_perf_measurement_path_present,
                    report.release_qualification.required_checks_packet_synced,
                    report.release_qualification.notice_inventory_present,
                    report.release_qualification.rights_license_decision,
                    report.release_qualification.missing_evidence_classes.len()
                );
                println!(
                    "accessibility: ready_base={} fixture_labels={} fixture_semantics={} cli_keyboard={} disclosure={} wcag_claimed={} final_v0={} release_ready={}",
                    report.accessibility.ready_base,
                    report.accessibility.fixture_cli_labels_checked,
                    report.accessibility.fixture_state_semantics_checked,
                    report.accessibility.cli_keyboard_path_documented,
                    report.accessibility.disclosure_clarity_checked,
                    report.accessibility.wcag_conformance_claimed,
                    report.accessibility.final_v0_ui_present,
                    report.accessibility.release_ready
                );
                println!(
                    "evidence_corpus: ready_base={} versioned={} synthetic_owned={} clinical_quality={} scale_generator={} release_ready={} corpus={:?}@{:?}",
                    report.evidence_corpus.ready_base,
                    report.evidence_corpus.versioned_corpus,
                    report.evidence_corpus.synthetic_owned_only,
                    report.evidence_corpus.clinical_quality_claimed,
                    report.evidence_corpus.scale_corpus_generator_present,
                    report.evidence_corpus.release_ready,
                    report.evidence_corpus.current_corpus_id,
                    report.evidence_corpus.current_version
                );
                println!(
                    "pack_signer: ready_base={} synthetic_trust_root={} anti_rollback={} release_ready={}",
                    report.pack_signer.ready_base,
                    report.pack_signer.synthetic_trust_root,
                    report.pack_signer.anti_rollback,
                    report.pack_signer.release_ready
                );
                println!(
                    "os_sandbox: ready_base={} linux_measured={} linux_landlock_composition={} linux_seccomp_composition={} windows_measured={} windows_appcontainer_fs_measured={} windows_appcontainer_network_measured={} windows_appcontainer_lpac_measured={} macos_measured={} macos_app_sandbox_entitlements_measured={} macos_app_sandbox_enforcement_measured={} composition_inventory={} platform_qualified={} release_ready={}",
                    report.os_sandbox.ready_base,
                    report.os_sandbox.linux_measured,
                    report.os_sandbox.linux_landlock_composition_measured,
                    report.os_sandbox.linux_seccomp_composition_measured,
                    report.os_sandbox.windows_measured,
                    report.os_sandbox.windows_appcontainer_fs_measured,
                    report.os_sandbox.windows_appcontainer_network_measured,
                    report.os_sandbox.windows_appcontainer_lpac_measured,
                    report.os_sandbox.macos_measured,
                    report.os_sandbox.macos_app_sandbox_entitlements_measured,
                    report.os_sandbox.macos_app_sandbox_enforcement_measured,
                    report.os_sandbox.composition_inventory_present,
                    report.os_sandbox.platform_qualified,
                    report.os_sandbox.release_ready
                );
                for missing in &report.release_qualification.missing_evidence_classes {
                    println!("release_qualification_missing: {missing}");
                }
                for note in &report.notes {
                    println!("note: {note}");
                }
            }
            let rendered = if json {
                serde_json::to_string(&report)?
            } else {
                format!("{report:?}")
            };
            assert_no_secret_markers(&rendered)?;
            Ok(())
        }
        Commands::Status { json } => {
            let report = build_doctor_report(None, false, privacy_proof_artifact_present());
            let status = status_summary(&report);
            if json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                println!("{} {}", status.product_name, status.version);
                println!("local_only: {}", status.local_only);
                println!("synthetic_only: {}", status.synthetic_only);
                println!("real_phi_authorized: {}", status.real_phi_authorized);
                println!("desktop_shell: {}", status.desktop_shell);
                println!("network_default_deny: {}", status.network_default_deny);
                println!("private_data_ready: {}", status.private_data_ready);
                println!("release_ready: {}", status.release_ready);
                println!(
                    "multi_client_release_ready: {}",
                    status.multi_client_release_ready
                );
                println!(
                    "wcag_conformance_claimed: {}",
                    status.wcag_conformance_claimed
                );
                println!(
                    "missing_release_evidence: {}",
                    status.missing_release_evidence.join(",")
                );
            }
            assert_no_secret_markers(&serde_json::to_string(&status)?)?;
            Ok(())
        }
        Commands::Capabilities { json } => {
            let rows = capability_rows();
            if json {
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else {
                println!("{}", human_heading("capability map"));
                println!("Evidence first. Action second.");
                println!();
                for row in rows {
                    println!(
                        "{} | {} | {} | {}",
                        row.surface, row.path, row.authority, row.status
                    );
                }
            }
            Ok(())
        }
        Commands::Vault { action } => match action {
            VaultCmd::Create {
                vault_id,
                vault_root,
                passphrase_env,
            } => {
                let pw = std::env::var(&passphrase_env).with_context(|| {
                    format!("set passphrase in env var {passphrase_env} (not argv)")
                })?;
                let codes = vault_create_sealed(&vault_id, &vault_root, &pw)?;
                println!("vault_created: {}", vault_root.display());
                println!("vault_sealed: true");
                println!("recovery_codes_count: {}", codes.len());
                println!("note: store recovery codes offline; not re-printed by doctor");
                Ok(())
            }
            VaultCmd::Open {
                vault_id,
                vault_root,
                passphrase_env,
            } => {
                let pw = std::env::var(&passphrase_env)
                    .with_context(|| format!("set passphrase in env var {passphrase_env}"))?;
                vault_open_sealed(&vault_id, &vault_root, &pw)?;
                println!("vault_open: {}", vault_root.display());
                println!("vault_sealed: true");
                Ok(())
            }
            VaultCmd::OpenSynthetic {
                vault_id,
                vault_root,
            } => {
                let mut session = CliSession::connect(&vault_id).map_err(auth)?;
                session
                    .open_synthetic_vault(&vault_root.display().to_string())
                    .map_err(auth)?;
                println!("synthetic_vault_open: {}", vault_root.display());
                Ok(())
            }
        },
        Commands::Ingest {
            vault_id,
            vault_root,
            file,
            mode,
        } => {
            if mode != "synthetic" {
                bail!("only synthetic ingest is authorized (REAL_PHI not authorized)");
            }
            let mut session = CliSession::connect(&vault_id).map_err(auth)?;
            session
                .open_synthetic_vault(&vault_root.display().to_string())
                .map_err(auth)?;
            let source_id = session
                .ingest_fhir_file(&file.display().to_string())
                .map_err(auth)?;
            println!("ingested_source_id: {}", source_id.as_str());
            Ok(())
        }
        Commands::Timeline {
            vault_id,
            subject,
            json,
        } => {
            let mut session = CliSession::connect(&vault_id).map_err(auth)?;
            let body = session.timeline(&subject).map_err(auth)?;
            print_json_or_debug(&body, json)
        }
        Commands::Brief {
            vault_id,
            subject,
            json,
        } => {
            let mut session = CliSession::connect(&vault_id).map_err(auth)?;
            let body = session.brief(&subject).map_err(auth)?;
            print_json_or_debug(&body, json)
        }
        Commands::Coverage {
            vault_id,
            subject,
            json,
        } => {
            let mut session = CliSession::connect(&vault_id).map_err(auth)?;
            let body = session.coverage(&subject).map_err(auth)?;
            print_json_or_debug(&body, json)
        }
        Commands::Patient { action } => match action {
            PatientCmd::Show {
                vault_id,
                subject,
                json,
            } => {
                let mut session = CliSession::connect(&vault_id).map_err(auth)?;
                let timeline = session.timeline(&subject).map_err(auth)?;
                let brief = session.brief(&subject).map_err(auth)?;
                let coverage = session.coverage(&subject).map_err(auth)?;
                let snapshot = CliPatientSnapshot {
                    subject,
                    timeline,
                    brief,
                    coverage,
                    authority_note: "Read-only trusted projections; missing/conflicting evidence is preserved and no clinical interpretation is invented.".to_owned(),
                };
                if json {
                    println!("{}", serde_json::to_string_pretty(&snapshot)?);
                } else {
                    println!("patient: {}", snapshot.subject);
                    println!("timeline_events: {}", snapshot.timeline.events.len());
                    println!("coverage_slots: {}", snapshot.coverage.slots.len());
                    println!("brief: narrow LLM-free trusted projection");
                    println!("authority: {}", snapshot.authority_note);
                }
                assert_no_secret_markers(&serde_json::to_string(&snapshot)?)?;
                Ok(())
            }
            PatientCmd::Timeline {
                vault_id,
                subject,
                json,
            } => {
                let mut session = CliSession::connect(&vault_id).map_err(auth)?;
                let body = session.timeline(&subject).map_err(auth)?;
                print_json_or_debug(&body, json)
            }
            PatientCmd::Brief {
                vault_id,
                subject,
                json,
            } => {
                let mut session = CliSession::connect(&vault_id).map_err(auth)?;
                let body = session.brief(&subject).map_err(auth)?;
                print_json_or_debug(&body, json)
            }
            PatientCmd::Coverage {
                vault_id,
                subject,
                json,
            } => {
                let mut session = CliSession::connect(&vault_id).map_err(auth)?;
                let body = session.coverage(&subject).map_err(auth)?;
                print_json_or_debug(&body, json)
            }
        },
        Commands::Actions { action } => match action {
            ActionsCmd::Outbox { vault_id, json } => {
                let mut session = CliSession::connect(&vault_id).map_err(auth)?;
                let entries = session.outbox().map_err(auth)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&entries)?);
                } else if entries.is_empty() {
                    println!("outbox: empty");
                    println!("note: read-only view; UNKNOWN never authorizes blind retry");
                } else {
                    for entry in entries {
                        println!(
                            "{} | {} | {:?} | payload_sha256={}",
                            entry.action_id.as_str(),
                            entry.action,
                            entry.effect_state,
                            entry.payload_digest.to_hex()
                        );
                    }
                }
                Ok(())
            }
        },
        Commands::Audit { action } => match action {
            AuditCmd::Disclosures { vault_id, json } => {
                let mut session = CliSession::connect(&vault_id).map_err(auth)?;
                let records = session.disclosures().map_err(auth)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&records)?);
                } else if records.is_empty() {
                    println!("disclosures: empty");
                } else {
                    for record in records {
                        println!(
                            "{} | purpose={} | scope={} | synthetic_only={} | release_ready_claimed={}",
                            record.disclosure_id.as_str(),
                            record.purpose,
                            record.scope,
                            record.synthetic_only,
                            record.release_ready_claimed
                        );
                    }
                }
                Ok(())
            }
        },
        Commands::Fhir { action } => match action {
            FhirCmd::Support { vault_id, json } => {
                let mut session = CliSession::connect(&vault_id).map_err(auth)?;
                let matrix = session.fhir_support_matrix().map_err(auth)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&matrix)?);
                } else {
                    println!("FHIR {} support matrix", matrix.fhir_version);
                    println!("synthetic_only: {}", matrix.synthetic_only);
                    println!(
                        "full_conformance_claimed: {}",
                        matrix.full_conformance_claimed
                    );
                    println!("release_ready: {}", matrix.release_ready);
                    for resource in &matrix.resources {
                        println!(
                            "{} | lexical={:?} structural={:?} provenance={:?} clinical_interpretation={:?}",
                            resource.resource_type,
                            resource.lexical,
                            resource.structural,
                            resource.provenance,
                            resource.clinical_interpretation
                        );
                    }
                }
                Ok(())
            }
        },
        Commands::PrivacyProof { json } => {
            let proof = PrivacyProof::spec_006_baseline();
            if json {
                println!("{}", serde_json::to_string_pretty(&proof)?);
            } else {
                println!("{proof:?}");
            }
            Ok(())
        }
        Commands::Packs { action } => match action {
            PacksCmd::Install { vault_id, path } => {
                let mut session = CliSession::connect(&vault_id).map_err(auth)?;
                let result = session
                    .packs_install_local(&path.display().to_string())
                    .map_err(auth)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
                if !result.admitted {
                    bail!("pack admission denied: {:?}", result.reason);
                }
                Ok(())
            }
            PacksCmd::List { vault_id, json } => {
                let mut session = CliSession::connect(&vault_id).map_err(auth)?;
                // Note: list is session-local; install then list in same process for CLI demos.
                let packs = session.packs_list().map_err(auth)?;
                print_json_or_debug(&packs, json)
            }
            PacksCmd::Catalog {
                snapshot,
                commit,
                repository,
                text,
                task,
                family,
                language,
                format,
                architecture,
                device,
                status,
                max_params,
                max_disk_mb,
                tier,
                offset,
                limit,
            } => {
                let view = medscale_core::model_catalog::query_catalog_snapshot(
                    &snapshot,
                    &repository,
                    &commit,
                    &medscale_core::model_catalog::CatalogQuery {
                        text,
                        task,
                        family,
                        language,
                        format,
                        architecture,
                        license_claim: None,
                        tier,
                        device_fit: device,
                        status,
                        max_params,
                        max_disk_mb,
                        offset,
                        limit,
                    },
                )?;
                println!("{}", serde_json::to_string_pretty(&view)?);
                Ok(())
            }
            PacksCmd::Variants {
                snapshot,
                commit,
                repository,
                repo_id,
                host,
            } => {
                let variants = medscale_core::model_catalog::catalog_variants(
                    &snapshot,
                    &repository,
                    &commit,
                    &repo_id,
                    host.as_deref(),
                )?;
                println!("{}", serde_json::to_string_pretty(&variants)?);
                Ok(())
            }
            PacksCmd::AdmitSnapshot {
                vault_id,
                snapshot,
                commit,
                repository,
                snapshot_dir,
                onnx_file,
                sequence_length,
                out,
            } => {
                let pack_id = medscale_core::model_catalog::build_pack_from_snapshot(
                    &snapshot,
                    &repository,
                    &commit,
                    &snapshot_dir,
                    &onnx_file,
                    sequence_length,
                    &out,
                )?;
                let mut session = CliSession::connect(&vault_id).map_err(auth)?;
                let result = session
                    .packs_install_local(&out.display().to_string())
                    .map_err(auth)?;
                println!("pack_id: {pack_id}");
                println!("{}", serde_json::to_string_pretty(&result)?);
                if !result.admitted {
                    bail!("pack admission denied: {:?}", result.reason);
                }
                Ok(())
            }
            PacksCmd::Acquire {
                snapshot,
                commit,
                repository,
                repo_id,
                onnx_file,
                approve_bytes,
                i_consent_to_download,
                out,
            } => {
                if !i_consent_to_download {
                    bail!(
                        "refusing to download: pass --i-consent-to-download to approve this acquisition"
                    );
                }
                let bytes = std::fs::read(&snapshot)?;
                let catalog =
                    medscale_core::model_catalog::import_catalog(&repository, &commit, &bytes)?;
                let report = medscale_core::model_acquisition::acquire_snapshot(
                    &medscale_core::model_acquisition::live_transport(),
                    &catalog,
                    &medscale_core::model_acquisition::AcquisitionConsent {
                        repo_id,
                        onnx_file,
                        approved_bytes: approve_bytes,
                    },
                    &out,
                )?;
                println!("{}", serde_json::to_string_pretty(&report)?);
                Ok(())
            }
        },
        Commands::FixtureUi { action } => match action {
            FixtureUiCmd::Doctor { vault_root } => {
                let root = vault_root.map(|p| p.display().to_string());
                let report =
                    build_doctor_report(root.as_deref(), false, privacy_proof_artifact_present());
                let vm = FixtureUiViewModel::from_doctor(&report);
                if vm.surface != FixtureUiSurface::Doctor {
                    bail!("unexpected fixture UI surface");
                }
                if !vm.respects_phi_boundary() {
                    bail!("fixture UI PHI boundary violated");
                }
                assert_no_secret_markers(&serde_json::to_string(&vm)?)?;
                println!("{}", serde_json::to_string_pretty(&vm)?);
                Ok(())
            }
        },
        Commands::Journey { action } => match action {
            JourneyCmd::Run {
                vault_root,
                fixture,
                subject,
                vault_id,
                backup_dir,
                restore_dir,
                reject,
                json,
            } => {
                if !fixture.is_file() {
                    return Err(fail_json(
                        "fixture_missing",
                        format!("fixture not found: {}", fixture.display()),
                        json,
                    ));
                }
                let _ = std::fs::remove_dir_all(&backup_dir);
                let _ = std::fs::remove_dir_all(&restore_dir);
                let facade = CoreFacade::new();
                let cfg = JourneyConfig {
                    vault_id,
                    vault_root: vault_root.display().to_string(),
                    fixture_path: fixture.display().to_string(),
                    subject,
                    backup_dir: backup_dir.display().to_string(),
                    restore_dir: restore_dir.display().to_string(),
                    accept: !reject,
                };
                match run_minimum_lovable_journey(&facade, &cfg) {
                    Ok(report) => {
                        if !report.is_honest_ready_base() {
                            return Err(fail_json(
                                "honesty_failed",
                                "journey report must be synthetic READY_BASE with release_ready=false",
                                json,
                            ));
                        }
                        if json {
                            println!("{}", serde_json::to_string_pretty(&report)?);
                        } else {
                            println!("journey_ok: steps={}", report.steps.len());
                            println!("workflow_ready_base: {}", report.workflow_ready_base);
                            println!("release_ready: {}", report.release_ready);
                            if let Some(sid) = &report.source_id {
                                println!("source_id: {sid}");
                            }
                            if let Some(aid) = &report.assertion_id {
                                println!("assertion_id: {aid}");
                            }
                            if let Some(did) = &report.disclosure_id {
                                println!("disclosure_id: {did}");
                            }
                            for step in &report.steps {
                                println!("step {:?}: ok={}", step.step, step.ok);
                            }
                        }
                        Ok(())
                    }
                    Err(e) => Err(fail_json("journey_failed", format!("{e:?}"), json)),
                }
            }
        },
        Commands::HostIpc { action } => match action {
            HostIpcCmd::Serve {
                vault_root,
                endpoint,
            } => {
                std::fs::create_dir_all(&vault_root)
                    .with_context(|| format!("create vault_root {}", vault_root.display()))?;
                let endpoint = endpoint.unwrap_or_else(|| endpoint_for_vault_root(&vault_root));
                let server = HostIpcServer::bind(&vault_root, &endpoint)
                    .with_context(|| format!("bind host-ipc endpoint={endpoint}"))?;
                println!(
                    "host_ipc_listening endpoint={} vault_root={} os_ipc_qualified=true multi_client_release_ready=false",
                    server.endpoint(),
                    server.vault_root().display()
                );
                loop {
                    if let Err(e) = server.serve_connection() {
                        eprintln!("host_ipc_connection_error: {e}");
                    }
                }
            }
            HostIpcCmd::Call {
                endpoint,
                request_json,
            } => {
                let bytes = std::fs::read(&request_json)
                    .with_context(|| format!("read {}", request_json.display()))?;
                let req: medscale_contracts::envelopes::AuthorityRequest =
                    serde_json::from_slice(&bytes).context("parse AuthorityRequest JSON")?;
                let mut client = HostIpcClient::connect(&endpoint)
                    .with_context(|| format!("connect endpoint={endpoint}"))?;
                let resp = client.dispatch(req).context("ipc dispatch")?;
                println!("{}", serde_json::to_string_pretty(&resp)?);
                Ok(())
            }
        },
        Commands::Project { action } => project::run_project(action),
        Commands::Experiment { action } => project::run_experiment(action),
        Commands::Collab { action } => collaboration::run_collab(*action),
        Commands::CollabWork { action } => collaboration::run_collab_work(*action),
        Commands::MedAgent { action } => medagent::run_medagent(*action),
        Commands::ModelFleet { action } => model_fleet::run_model_fleet(*action),
        Commands::Privacy { action } => privacy_gate::run_privacy(*action),
        Commands::Browse { action } => browse::run_browse(*action),
        Commands::Audio { action } => audio::run_audio(*action),
        Commands::Analytics { action } => analytics::run_analytics(*action),
        Commands::Knowledge { action } => knowledge::run_knowledge(*action),
        Commands::Hub { action } => hub::run_hub(*action),
        Commands::Compute { action } => compute::run_compute(*action),
        Commands::R { action } => r_workspace::run_r(*action),
        Commands::Extension { action } => extensions::run_extension(*action),
        Commands::Huddle { action } => huddles::run_huddle(*action),
        Commands::Pack { action } => research_packs::run_pack(*action),
        Commands::Adapter { action } => institutional::run_adapter(*action),
        Commands::Federation { action } => federation::run_federation(*action),
        Commands::DataSource { action } => data_source::run_data_source(*action),
        Commands::Snapshot { action } => data_source::run_snapshot(*action),
        Commands::DataView { action } => data_source::run_data_view(*action),
        Commands::DataTransform { action } => data_source::run_data_transform(*action),
        Commands::DataRelease { action } => data_source::run_data_release(*action),
    }
}

fn auth(err: medscale_contracts::envelopes::AuthorityError) -> anyhow::Error {
    anyhow::anyhow!("authority error: {err:?}")
}

/// Creates an encrypted vault and seals it before returning.
///
/// Each CLI invocation is its own process and Core session. While unlocked,
/// the encrypted vault works in a SQLCipher working database; Core seals it
/// into the encrypted meta file and removes it, its SQLite sidecars and the
/// writer lease only on `CloseEncryptedVault`. Returning without it leaves
/// those files on disk, and the next open discards everything written since
/// the last seal as a crash leftover.
fn vault_create_sealed(vault_id: &str, vault_root: &Path, passphrase: &str) -> Result<Vec<String>> {
    let mut session = CliSession::connect(vault_id).map_err(auth)?;
    let codes = session
        .create_encrypted_vault(&vault_root.display().to_string(), passphrase)
        .map_err(auth)?;
    session.close_encrypted_vault().map_err(auth).context(
        "vault was created but could not be sealed; do not use it until it is reopened and closed",
    )?;
    Ok(codes)
}

/// Opens (verifies the passphrase for) an encrypted vault and seals it again
/// before returning, for the same reason as [`vault_create_sealed`].
fn vault_open_sealed(vault_id: &str, vault_root: &Path, passphrase: &str) -> Result<()> {
    let mut session = CliSession::connect(vault_id).map_err(auth)?;
    session
        .open_encrypted_vault(&vault_root.display().to_string(), passphrase)
        .map_err(auth)?;
    session
        .close_encrypted_vault()
        .map_err(auth)
        .context("vault opened but could not be sealed")?;
    Ok(())
}

fn print_json_or_debug<T: serde::Serialize + std::fmt::Debug>(value: &T, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{value:?}");
    }
    Ok(())
}

fn assert_no_secret_markers(text: &str) -> Result<()> {
    for marker in [
        "BEGIN PRIVATE",
        "passphrase=",
        "VaultDek(",
        "MEDSCALE_PASSPHRASE=",
    ] {
        if text.contains(marker) {
            bail!("doctor/CLI output contained forbidden secret marker: {marker}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId};
    use medscale_core::CliSession;
    use std::fs;

    #[test]
    fn doctor_json_has_required_axes_and_no_secrets() {
        let report = build_doctor_report(None, false, false);
        let json = serde_json::to_string(&report).unwrap();
        for key in [
            "product_name",
            "version",
            "local_only",
            "product_runtime_egress",
            "synthetic_only",
            "real_phi_authorized",
            "sync_risk",
            "key_store",
            "privacy_proof_freshness",
            "tauri_admitted",
            "network_broker",
            "default_deny",
            "packs_runtime",
            "offline_only",
            "mobile",
            "apps_shipped",
            "controlled_actions",
            "outbox_restart_qualified",
            "nphies_authorized",
            "online_packs",
            "broker_required",
            "mesc_artifact",
            "artifact_admitted",
            "record_semantics",
            "precision_aware_time",
            "fhir_interchange",
            "fhir_support_matrix",
            "full_conformance_claimed",
            "validator_is_authority",
            "workflow",
            "workflow_ready_base",
            "release_qualification",
            "prep_ready_base",
            "macos_ci_present",
            "missing_evidence_classes",
            "perf_harness_present",
            "sbom_scaffold_present",
            "sbom_lock_bound",
            "release_sbom_qualified",
            "release_dry_run_verifier_present",
            "migration_recovery_ready_base",
            "package_upgrade_rollback_scaffold_present",
            "portable_release_package_qualified",
            "package_lifecycle_qualified",
            "host_perf_measurement_path_present",
            "required_checks_packet_synced",
            "accessibility",
            "fixture_cli_labels_checked",
            "cli_keyboard_path_documented",
            "disclosure_clarity_checked",
            "wcag_conformance_claimed",
            "pack_signer",
            "synthetic_trust_root",
            "anti_rollback",
            "os_sandbox",
            "linux_measured",
            "linux_landlock_composition_measured",
            "linux_seccomp_composition_measured",
            "windows_measured",
            "windows_appcontainer_fs_measured",
            "windows_appcontainer_network_measured",
            "windows_appcontainer_lpac_measured",
            "macos_measured",
            "macos_app_sandbox_entitlements_measured",
            "macos_app_sandbox_enforcement_measured",
            "composition_inventory_present",
            "composition_residuals_open",
            "platform_qualified",
            "os_keyring_available",
            "os_keyring_used",
            "private_data_ready",
            "probes_present",
            "swap_snapshot_honesty_present",
            "encrypted_authority_sync_qualified",
            "residual_risk_classes_open",
            "pagefile_existence",
            "hibernate_file_existence",
            "swap_existence",
            "snapshot_existence",
            "core_dump_config_existence",
            "pagefile_existence_honesty",
            "snapshot_protection_honesty",
            "notice_inventory_present",
            "rights_license_decision",
        ] {
            assert!(json.contains(key), "missing {key}");
        }
        assert_no_secret_markers(&json).unwrap();
        assert!(!report.tauri_admitted);
        assert!(!report.real_phi_authorized);
        assert!(report.network_broker.default_deny);
        assert!(!report.network_broker.live_partner_authorized);
        assert!(report.packs_runtime.offline_only);
        assert!(!report.packs_runtime.online_download_authorized);
        assert!(!report.mobile.apps_shipped);
        assert!(report.controlled_actions.is_honest_ready_base());
        assert!(!report.controlled_actions.nphies_authorized);
        assert!(!report.controlled_actions.unknown_blind_retry);
        assert!(!report.online_packs.online_download_authorized);
        assert!(report.online_packs.broker_required);
        assert!(!report.online_packs.hf_runtime_required);
        assert!(!report.mesc_artifact.artifact_admitted);
        assert!(!report.mesc_artifact.python_runtime_imported);
        assert!(!report.mesc_artifact.required);
        assert!(!report.mesc_artifact.blocks_release());
        assert!(report.mesc_artifact.is_honest_ready_base());
        assert!(!report.fhir_interchange.full_conformance_claimed);
        assert!(!report.fhir_interchange.validator_is_authority);
        assert!(!report.fhir_interchange.release_ready);
        assert!(report.fhir_support_matrix.is_honest_ready_base());
        assert!(report.workflow.workflow_ready_base);
        assert!(!report.workflow.release_ready);
        assert!(report.release_qualification.is_honest_prep());
        assert!(report.release_qualification.macos_ci_present);
        assert!(!report.release_qualification.macos_qualified);
        assert!(!report.release_qualification.release_ready);
        assert!(report.accessibility.is_honest_ready_base());
        assert!(!report.accessibility.wcag_conformance_claimed);
        assert!(report.pack_signer.is_honest_ready_base());
        assert!(report.os_sandbox.is_honest_ready_base());
    }

    #[test]
    fn fixture_ui_and_cli_help_surface_required_a11y_labels_without_wcag_claim() {
        use clap::CommandFactory;

        let report = build_doctor_report(None, false, false);
        let vm = FixtureUiViewModel::from_doctor(&report);
        assert!(vm.has_required_a11y_labels());
        assert!(vm.has_required_a11y_semantics());
        assert!(FixtureUiViewModel::required_surface_labels().contains(&vm.title.as_str()));
        assert!(!report.accessibility.wcag_conformance_claimed);
        assert!(!report.accessibility.release_ready);

        let mut help = Vec::new();
        Cli::command()
            .write_long_help(&mut help)
            .expect("render CLI help");
        let help_text = String::from_utf8(help).expect("utf8 help").to_lowercase();
        for required in [
            "doctor",
            "vault",
            "ingest",
            "timeline",
            "brief",
            "coverage",
            "privacy-proof",
            "fixture-ui",
            "journey",
            "host-ipc",
        ] {
            assert!(
                help_text.contains(required),
                "CLI help missing required label/path: {required}"
            );
        }

        // --json lives on subcommands (keyboard/operator path), not top-level about.
        let mut doctor_help = Vec::new();
        Cli::command()
            .find_subcommand_mut("doctor")
            .expect("doctor subcommand")
            .write_long_help(&mut doctor_help)
            .expect("render doctor help");
        let doctor_help = String::from_utf8(doctor_help).expect("utf8").to_lowercase();
        assert!(
            doctor_help.contains("--json"),
            "doctor help must expose --json for operator keyboard path"
        );

        let err = CliJsonError::new("vault_required", "open a vault first");
        assert_eq!(err.code, "vault_required");
        assert!(err.synthetic_only);
        assert_eq!(err.error, "cli_error");
    }

    #[test]
    fn fixture_ui_doctor_respects_phi_and_has_no_secrets() {
        let report = build_doctor_report(None, false, false);
        let vm = FixtureUiViewModel::from_doctor(&report);
        assert_eq!(vm.surface, FixtureUiSurface::Doctor);
        assert!(vm.respects_phi_boundary());
        let json = serde_json::to_string(&vm).unwrap();
        assert_no_secret_markers(&json).unwrap();
    }

    #[test]
    fn privacy_proof_limitations_include_tauri_and_no_systemwide_claim() {
        let proof = PrivacyProof::spec_006_baseline();
        let blob = serde_json::to_string(&proof).unwrap();
        assert!(
            blob.contains("Tauri")
                || blob.contains("tauri")
                || proof.limitations.iter().any(|l| l.contains("Tauri"))
        );
        assert!(
            proof
                .limitations
                .iter()
                .any(|l| l.contains("system-wide") || l.contains("zero packets"))
        );
        assert_eq!(
            proof.webview_cache_scan,
            medscale_contracts::doctor::WebViewScanStatus::NotApplicable
        );
    }

    #[test]
    fn cli_does_not_depend_on_storage_or_rusqlite() {
        let manifest =
            fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
        assert!(!manifest.contains("medscale-storage"));
        assert!(!manifest.contains("rusqlite"));
        assert!(!manifest.contains("tauri"));
    }

    #[test]
    fn cli_wire_enum_sizes_stay_stack_safe() {
        // Windows default main-thread stack is 1 MiB; every command parses on
        // it. This guard fails loudly instead of overflowing the binary.
        eprintln!("size Commands={}", std::mem::size_of::<Commands>());
        eprintln!(
            "size RequestBody={}",
            std::mem::size_of::<medscale_contracts::envelopes::RequestBody>()
        );
        eprintln!(
            "size ResponseBody={}",
            std::mem::size_of::<medscale_contracts::envelopes::ResponseBody>()
        );
        eprintln!(
            "size AuthorityRequest={}",
            std::mem::size_of::<medscale_contracts::envelopes::AuthorityRequest>()
        );
        assert!(std::mem::size_of::<Commands>() <= 4096);
        assert!(std::mem::size_of::<medscale_contracts::envelopes::RequestBody>() <= 4096);
        assert!(std::mem::size_of::<medscale_contracts::envelopes::ResponseBody>() <= 4096);
    }

    #[test]
    fn cli_product_paths_are_discoverable_and_structured() {
        use clap::CommandFactory;

        let mut command = Cli::command();
        let mut help = Vec::new();
        command.write_long_help(&mut help).expect("top-level help");
        let help = String::from_utf8(help).expect("utf8 help").to_lowercase();
        for required in [
            "status",
            "capabilities",
            "patient",
            "actions",
            "audit",
            "fhir",
            "host-ipc",
        ] {
            assert!(
                help.contains(required),
                "missing product CLI path: {required}"
            );
        }

        for (group, child) in [
            ("patient", "show"),
            ("actions", "outbox"),
            ("audit", "disclosures"),
            ("fhir", "support"),
        ] {
            let parent = command.find_subcommand_mut(group).expect("group command");
            let child = parent.find_subcommand_mut(child).expect("child command");
            let mut child_help = Vec::new();
            child
                .write_long_help(&mut child_help)
                .expect("render grouped help");
            let child_help = String::from_utf8(child_help).expect("utf8").to_lowercase();
            assert!(
                child_help.contains("--json"),
                "{group} {child} missing --json"
            );
        }
    }

    #[test]
    fn capability_map_is_explicit_about_visual_only_parity() {
        let rows = capability_rows();
        assert!(rows.iter().any(|row| {
            row.surface == "Population Insights" && row.status == "Desktop visual aggregation"
        }));
        assert!(rows.iter().any(|row| {
            row.surface == "Controlled actions" && row.authority.contains("EffectState")
        }));
        let json = serde_json::to_string(&rows).expect("capability json");
        assert_no_secret_markers(&json).expect("no secrets");
    }

    #[test]
    fn cli_read_parity_uses_existing_authority_contracts() {
        let mut session = CliSession::connect("cli-parity").expect("session");
        assert!(session.outbox().expect("outbox").is_empty());
        assert!(session.disclosures().expect("disclosures").is_empty());
        let matrix = session.fhir_support_matrix().expect("FHIR support");
        assert!(matrix.is_honest_ready_base());
        assert!(!matrix.full_conformance_claimed);
        assert!(!matrix.release_ready);
    }

    #[test]
    fn human_identity_is_terminal_native_and_json_contracts_stay_separate() {
        assert_eq!(human_heading("capability map"), "MedScale · capability map");
        let rows = capability_rows();
        let json = serde_json::to_string(&rows).expect("capability json");
        assert!(!json.contains("Evidence first. Action second."));
        assert!(!json.contains("MedScale · capability map"));
    }

    #[test]
    fn cli_longitudinal_wedge() {
        let root = std::env::temp_dir().join(format!("medscale-006-wedge-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let mut session = CliSession::connect("cli-wedge").unwrap();
        session
            .open_synthetic_vault(&root.display().to_string())
            .unwrap();

        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/synthetic/fhir/r4/presentation/patient-golden.json");
        let bytes = fs::read(&fixture).unwrap();
        let resource: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let source_id = session
            .ingest_fhir_file(&fixture.display().to_string())
            .unwrap();
        let subject = "subject-wedge";
        session
            .promote_fixture_as_assertion(source_id, subject, "patient", resource)
            .unwrap();

        let tl = session.timeline(subject).unwrap();
        assert!(!tl.events.is_empty());
        let brief = session.brief(subject).unwrap();
        assert!(!brief.sections.identity.is_empty());
        let cov = session.coverage(subject).unwrap();
        assert!(!cov.slots.is_empty());

        // Touch realm/scope types so wedge stays typed.
        let _ = RealmId::new("x");
        let _ = AuthorityScopeId::new("y");
        let _ = OpaqueId::new("z");
    }

    fn enc_tmp(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("medscale-cli-seal-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn vault_create_seals_and_leaves_no_work_db_or_lease() {
        let root = enc_tmp("create");
        let codes = vault_create_sealed("cli-seal-create", &root, "correct horse battery").unwrap();
        assert!(
            !codes.is_empty(),
            "recovery codes are issued once at creation"
        );
        assert!(
            root.join("meta.sealed").is_file(),
            "sealed meta file exists"
        );
        assert!(
            !root.join("meta.work.sqlite3").exists(),
            "working DB must be sealed and removed"
        );
        assert!(
            !root.join(".writer.lock").exists(),
            "writer lease must be released"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn vault_open_seals_again_and_rejects_a_wrong_passphrase() {
        let root = enc_tmp("open");
        vault_create_sealed("cli-seal-open", &root, "correct horse battery").unwrap();
        assert!(vault_open_sealed("cli-seal-open", &root, "wrong passphrase!").is_err());
        vault_open_sealed("cli-seal-open", &root, "correct horse battery").unwrap();
        assert!(
            !root.join("meta.work.sqlite3").exists(),
            "working DB must be sealed and removed after open"
        );
        assert!(
            !root.join(".writer.lock").exists(),
            "writer lease must be released after open"
        );
        // Repeatable: a second open is not blocked by a stale lease.
        vault_open_sealed("cli-seal-open", &root, "correct horse battery").unwrap();
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn writes_made_before_close_survive_reopen() {
        let root = enc_tmp("persist");
        let root_s = root.display().to_string();
        {
            let mut session = CliSession::connect("cli-seal-persist").unwrap();
            session
                .create_encrypted_vault(&root_s, "correct horse battery")
                .unwrap();
            session
                .project_create("Sealed study".to_owned(), None)
                .unwrap();
            session.close_encrypted_vault().unwrap();
        }
        let mut session = CliSession::connect("cli-seal-persist").unwrap();
        session
            .open_encrypted_vault(&root_s, "correct horse battery")
            .unwrap();
        let (projects, _) = session.project_list(None, None, None).unwrap();
        assert_eq!(
            projects.len(),
            1,
            "project written before close is listed after reopen"
        );
        session.close_encrypted_vault().unwrap();
        let _ = fs::remove_dir_all(&root);
    }
}
