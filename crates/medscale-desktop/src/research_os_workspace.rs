//! Research OS operations view-models (Spec 093, Desktop parity for Specs
//! 084-091).
//!
//! Desktop reaches every plane only through the Core-owned `CliSession`;
//! it holds no storage, worker, transport or key code. This view is
//! lists each plane with an explicit state, and a plane that Core cannot
//! read is shown as unavailable, never as empty.
//!
//! Spec 094 adds row actions that are reversible and move no data: cancel a
//! queued or running Compute job, enable or disable an extension, suspend or
//! resume an institutional adapter. Terminal or data-moving actions (revoke,
//! send, export, import, publish, install, consent) stay in the CLI.

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::institutional::{AdapterActRequest, AdapterState};
use medscale_contracts::objects::OpaqueId;
use medscale_contracts::research_packs::CLINICAL_RESEARCH_PACK_ID;
use medscale_core::CliSession;

/// One row of the operations list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowVm {
    pub plane: String,
    pub id: String,
    pub state: String,
    pub detail: String,
    /// The one Desktop action this row offers, or empty.
    pub action: String,
}

/// The planes in display order.
pub const PLANES: [&str; 8] = [
    "Hub",
    "Compute",
    "R Workspace",
    "Extensions",
    "Huddles",
    "Research Packs",
    "Adapters",
    "Federation",
];

#[must_use]
pub fn status_message(err: &AuthorityError) -> &'static str {
    match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => "denied by Core authority",
        AuthorityError::NotFound => "none in this vault",
        AuthorityError::Corrupt { .. } => "corrupt: integrity check failed",
        _ => "unavailable: vault or Core not ready",
    }
}

/// The reversible action a row offers in Desktop, if any.
#[must_use]
pub fn row_action(plane: &str, state: &str) -> Option<&'static str> {
    match (plane, state) {
        ("Compute", "queued" | "running") => Some("Cancel"),
        ("Extensions", "enabled") => Some("Disable"),
        ("Extensions", "disabled") => Some("Enable"),
        ("Adapters", "active") => Some("Suspend"),
        ("Adapters", "suspended") => Some("Resume"),
        _ => None,
    }
}

fn row(plane: &str, id: &str, state: &str, detail: String) -> RowVm {
    RowVm {
        plane: plane.to_owned(),
        id: id.to_owned(),
        state: state.to_owned(),
        detail,
        action: row_action(plane, state).unwrap_or_default().to_owned(),
    }
}

/// Runs one row action through Core and returns the resulting state. A
/// refusal or an action the row does not offer is an error message; nothing
/// is changed then.
pub fn act(
    session: &mut CliSession,
    project_id: &str,
    plane: &str,
    id: &str,
    action: &str,
) -> Result<String, String> {
    let core = |err: AuthorityError| status_message(&err).to_owned();
    match (plane, action) {
        ("Compute", "Cancel") => session
            .compute_cancel(OpaqueId::new(id))
            .map(|view| view.job.state.as_str().to_owned())
            .map_err(core),
        ("Extensions", "Enable" | "Disable") => {
            let receipt = session
                .ext_set_enabled(OpaqueId::new(project_id), id.to_owned(), action == "Enable")
                .map_err(core)?;
            match (receipt.refusal, receipt.resulting_state) {
                (Some(refusal), _) => Err(format!("refused: {}", refusal.as_str())),
                (None, Some(state)) => Ok(state.as_str().to_owned()),
                (None, None) => Err("no resulting state recorded".to_owned()),
            }
        }
        ("Adapters", "Suspend" | "Resume") => {
            let state = if action == "Resume" {
                AdapterState::Active
            } else {
                AdapterState::Suspended
            };
            let result = session
                .adapter_act(AdapterActRequest::SetState {
                    adapter_id: OpaqueId::new(id),
                    state,
                })
                .map_err(core)?;
            match (result.receipt.refusal, result.adapter) {
                (Some(refusal), _) => Err(format!("refused: {}", refusal.as_str())),
                (None, Some(adapter)) => Ok(adapter.state.as_str().to_owned()),
                (None, None) => Err("no adapter returned".to_owned()),
            }
        }
        _ => Err(format!("{action} is not a Desktop action for {plane}")),
    }
}

fn unavailable(plane: &str, err: &AuthorityError) -> RowVm {
    row(plane, "-", "unavailable", status_message(err).to_owned())
}

fn plane_rows(session: &mut CliSession, project: &OpaqueId, plane: &str) -> Vec<RowVm> {
    let rows: Result<Vec<RowVm>, AuthorityError> = match plane {
        "Hub" => session.hub_links().map(|links| {
            links
                .iter()
                .map(|l| {
                    row(
                        plane,
                        l.header.id.as_str(),
                        "linked",
                        format!(
                            "device {} · project {}",
                            l.device_id.as_str(),
                            l.project_id.as_str()
                        ),
                    )
                })
                .collect()
        }),
        "Compute" => session.compute_jobs(project.clone()).map(|jobs| {
            jobs.iter()
                .map(|j| {
                    row(
                        plane,
                        j.header.id.as_str(),
                        j.state.as_str(),
                        format!("{} · closed job kind", j.manifest.kind.as_str()),
                    )
                })
                .collect()
        }),
        "R Workspace" => session.r_workspaces(project.clone()).map(|list| {
            list.iter()
                .map(|w| {
                    row(
                        plane,
                        w.id().as_str(),
                        "staged",
                        format!(
                            "{} · {} input(s) · class {} · managed R not admitted",
                            w.manifest.label,
                            w.manifest.inputs.len(),
                            w.manifest.data_class.as_str()
                        ),
                    )
                })
                .collect()
        }),
        "Extensions" => session.ext_list(project.clone()).map(|view| {
            view.installs
                .iter()
                .map(|i| {
                    row(
                        plane,
                        &i.extension_id,
                        i.state.as_str(),
                        format!("publisher {} · declarative only", i.publisher_id),
                    )
                })
                .collect()
        }),
        "Huddles" => session.huddle_list(project.clone()).map(|list| {
            list.iter()
                .map(|h| {
                    row(
                        plane,
                        h.header.id.as_str(),
                        h.state.as_str(),
                        format!("{} · retention {} days", h.title, h.retention_days),
                    )
                })
                .collect()
        }),
        "Research Packs" => session
            .pack_artifact_list(project.clone(), CLINICAL_RESEARCH_PACK_ID.to_owned())
            .map(|list| {
                list.iter()
                    .map(|a| {
                        row(
                            plane,
                            a.header.id.as_str(),
                            &a.workflow_state,
                            format!("{} · pack v{}", a.type_id, a.pack_version),
                        )
                    })
                    .collect()
            }),
        "Adapters" => session.adapter_list(project.clone()).map(|list| {
            list.iter()
                .map(|a| {
                    row(
                        plane,
                        a.header.id.as_str(),
                        a.state.as_str(),
                        format!(
                            "{} · ceiling {} · no network transport in this build",
                            a.config.destination,
                            a.config.data_class_ceiling.as_str()
                        ),
                    )
                })
                .collect()
        }),
        "Federation" => session.federation_get().map(|view| {
            view.peers
                .iter()
                .map(|p| {
                    row(
                        plane,
                        &p.institution_id,
                        p.state.as_str(),
                        format!(
                            "ceiling {} · last received #{}",
                            p.ceiling.as_str(),
                            p.last_received_seq
                        ),
                    )
                })
                .collect()
        }),
        _ => Ok(Vec::new()),
    };
    match rows {
        Ok(rows) if rows.is_empty() => {
            vec![row(plane, "-", "empty", "nothing recorded yet".to_owned())]
        }
        Ok(rows) => rows,
        Err(err) => vec![unavailable(plane, &err)],
    }
}

/// Every plane for one Project, in display order. A plane Core cannot read
/// is a single `unavailable` row.
pub fn rows(session: &mut CliSession, project_id: &str) -> Vec<RowVm> {
    let project = OpaqueId::new(project_id);
    PLANES
        .iter()
        .flat_map(|plane| plane_rows(session, &project, plane))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_plane_is_listed_and_empty_is_not_unavailable() {
        let dir = std::env::temp_dir().join(format!("medscale-093-rows-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut s = CliSession::connect("vault-093-desktop").unwrap();
        s.open_synthetic_vault(&dir.display().to_string()).unwrap();
        let project = s.project_create("ops".to_owned(), None).unwrap().header.id;
        let rows = rows(&mut s, project.as_str());
        for plane in PLANES {
            assert!(
                rows.iter().any(|r| r.plane == plane),
                "{plane} missing from the operations view"
            );
        }
        // A fresh vault has nothing recorded; nothing is reported as data.
        assert!(
            rows.iter()
                .all(|r| r.state == "empty" || r.state == "unavailable"),
            "{rows:?}"
        );
    }

    #[test]
    fn only_reversible_actions_are_offered() {
        assert_eq!(row_action("Compute", "queued"), Some("Cancel"));
        assert_eq!(row_action("Compute", "succeeded"), None);
        assert_eq!(row_action("Extensions", "enabled"), Some("Disable"));
        assert_eq!(row_action("Extensions", "quarantined"), None);
        assert_eq!(row_action("Adapters", "suspended"), Some("Resume"));
        assert_eq!(row_action("Adapters", "revoked"), None);
        for plane in [
            "Hub",
            "R Workspace",
            "Huddles",
            "Research Packs",
            "Federation",
        ] {
            assert_eq!(row_action(plane, "active"), None, "{plane}");
        }
    }

    #[test]
    fn actions_go_through_core_and_unknown_actions_change_nothing() {
        let dir = std::env::temp_dir().join(format!("medscale-094-act-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut s = CliSession::connect("vault-094-desktop").unwrap();
        s.open_synthetic_vault(&dir.display().to_string()).unwrap();
        let project = s.project_create("ops".to_owned(), None).unwrap().header.id;
        // Not offered: refused before Core is asked.
        assert!(act(&mut s, project.as_str(), "Federation", "peer", "Revoke").is_err());
        // Offered, but the object does not exist: Core refuses; nothing is
        // reported as done.
        assert!(act(&mut s, project.as_str(), "Compute", "job-missing", "Cancel").is_err());
        assert!(
            act(
                &mut s,
                project.as_str(),
                "Adapters",
                "adapter-missing",
                "Suspend"
            )
            .is_err()
        );
        let ext = act(
            &mut s,
            project.as_str(),
            "Extensions",
            "org.example.none",
            "Disable",
        );
        assert!(ext.is_err(), "{ext:?}");
    }

    #[test]
    fn an_adapter_is_suspended_and_resumed_through_core() {
        use medscale_contracts::institutional::{AdapterCapability, AdapterConfig, AdapterKind};
        use medscale_contracts::privacy_gate::DataClass;

        let dir = std::env::temp_dir().join(format!("medscale-094-adapter-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut s = CliSession::connect("vault-094-adapter").unwrap();
        s.open_synthetic_vault(&dir.display().to_string()).unwrap();
        let project = s.project_create("ops".to_owned(), None).unwrap().header.id;
        let adapter = s
            .adapter_act(AdapterActRequest::Register {
                project_id: project.clone(),
                name: "lab-s3".to_owned(),
                config: AdapterConfig {
                    kind: AdapterKind::ObjectStorage,
                    destination: "s3://lab-bucket/exports".to_owned(),
                    data_class_ceiling: DataClass::ExternalDeidentified,
                    credential_handle: Some("cred:lab-s3".to_owned()),
                    capabilities: vec![AdapterCapability::PutObject, AdapterCapability::HeadObject],
                },
            })
            .unwrap()
            .adapter
            .unwrap()
            .header
            .id;
        let listed = rows(&mut s, project.as_str());
        let row = listed
            .iter()
            .find(|r| r.plane == "Adapters" && r.id == adapter.as_str())
            .unwrap();
        assert_eq!(
            (row.state.as_str(), row.action.as_str()),
            ("active", "Suspend")
        );

        let state = act(
            &mut s,
            project.as_str(),
            "Adapters",
            adapter.as_str(),
            "Suspend",
        );
        assert_eq!(state.as_deref(), Ok("suspended"));
        let listed = rows(&mut s, project.as_str());
        let row = listed.iter().find(|r| r.id == adapter.as_str()).unwrap();
        assert_eq!(row.action, "Resume");
        let state = act(
            &mut s,
            project.as_str(),
            "Adapters",
            adapter.as_str(),
            "Resume",
        );
        assert_eq!(state.as_deref(), Ok("active"));

        // Every change left a Core receipt.
        let view = s.adapter_get(adapter).unwrap();
        assert!(view.receipts.len() >= 3, "{:?}", view.receipts);
    }

    #[test]
    fn errors_map_to_explicit_states() {
        assert_eq!(
            status_message(&AuthorityError::NotFound),
            "none in this vault"
        );
        assert_eq!(
            status_message(&AuthorityError::WrongScope),
            "denied by Core authority"
        );
    }
}
