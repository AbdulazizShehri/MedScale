//! Spec 084 MedScale Hub foundation commands (CLI vertical slice through
//! Core).
//!
//! Hub operator commands and client commands dispatch typed Core requests
//! via `CliSession`; `serve`, `join` and `sync` reach a Hub only over the
//! Spec 024 local-socket IPC through Core (`medscale_core::hub_sync`). The
//! CLI never touches storage, keys or sockets itself.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::hub::{HubEvent, HubEventKind, HubInvitationCode, SyncIntent, SyncOutcome};
use medscale_contracts::objects::OpaqueId;
use medscale_core::hub_sync::{self, IpcHubTransport};

use super::{fail_json, print_json_or_debug};

fn hub_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
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
        AuthorityError::Unavailable { message } => ("unavailable", message.clone()),
        AuthorityError::LeaseRequired | AuthorityError::VaultRequired => ("unavailable", debug),
        _ => ("internal", debug),
    };
    fail_json(code, message, json)
}

fn open_session(
    vault_id: &str,
    vault_root: &std::path::Path,
    json: bool,
) -> anyhow::Result<crate::vault_session::VaultSession> {
    crate::vault_session::open(vault_id, vault_root).map_err(|err| hub_fail(&err, json))
}

fn outcome_label(o: &SyncOutcome) -> String {
    match o {
        SyncOutcome::Applied {
            object_id,
            revision,
        } => format!("applied {} r{revision}", object_id.as_str()),
        SyncOutcome::ConflictCopy { note_id, revision } => {
            format!("conflict_copy {} r{revision}", note_id.as_str())
        }
        SyncOutcome::Conflict { conflict } => format!(
            "conflict {} expected r{} current {}",
            conflict.object_id.as_str(),
            conflict.expected_revision,
            conflict
                .current_revision
                .map_or_else(|| "unknown".to_owned(), |r| format!("r{r}"))
        ),
        SyncOutcome::Refused { reason } => format!("refused {}", reason.as_str()),
    }
}

fn event_label(e: &HubEvent) -> String {
    let what = match &e.kind {
        HubEventKind::DeviceEnrolled { device_id, .. } => {
            format!("device_enrolled {}", device_id.as_str())
        }
        HubEventKind::DeviceRevoked { device_id } => {
            format!("device_revoked {}", device_id.as_str())
        }
        HubEventKind::Submission { envelope, outcome } => format!(
            "submission {}#{} {}",
            envelope.body.device_id.as_str(),
            envelope.body.seq,
            outcome_label(outcome)
        ),
    };
    format!(
        "{}\t{what}\tsha256:{}",
        e.cursor,
        e.checkpoint_digest.to_hex()
    )
}

#[derive(Subcommand, Debug)]
pub enum HubCmd {
    /// Give this vault its Hub role (once).
    Init {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Issue a one-time invitation to one Project; prints the invitation
    /// code (JSON) to hand to the person enrolling a device. Shown once.
    Invite {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        json: bool,
    },
    /// Revoke an unredeemed invitation.
    RevokeInvitation {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        invitation_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Revoke an enrolled device (its sessions end; nothing it sends later
    /// is applied).
    RevokeDevice {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        device_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Hub devices, invitations and verified event-chain heads.
    Status {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Serve this Hub over the local-socket IPC for N client connections.
    Serve {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        /// Directory for the IPC host lock (not the vault root).
        #[arg(long)]
        host_dir: PathBuf,
        #[arg(long)]
        endpoint: String,
        #[arg(long, default_value_t = 1)]
        connections: u32,
    },
    /// Enroll this vault's new device with a Hub by invitation code.
    Join {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        endpoint: String,
        /// The invitation code JSON printed by `hub invite`.
        #[arg(long)]
        code_json: String,
        #[arg(long)]
        json: bool,
    },
    /// Sign and queue one intent (works offline).
    Queue {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        link_id: String,
        /// One `SyncIntent` as JSON.
        #[arg(long)]
        intent_json: String,
        #[arg(long)]
        json: bool,
    },
    /// Submit queued intents and pull the Hub's events over local IPC.
    Sync {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        link_id: String,
        #[arg(long)]
        json: bool,
    },
    /// This vault's Hub links.
    Links {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// A link's queued and sent intents with their outcomes.
    Outbox {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        link_id: String,
        #[arg(long)]
        json: bool,
    },
    /// A link's mirrored Hub events (verified chain).
    Mirror {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        link_id: String,
        #[arg(long, default_value_t = 0)]
        after: u64,
        #[arg(long, default_value_t = 100)]
        limit: u32,
        #[arg(long)]
        json: bool,
    },
}

#[allow(clippy::too_many_lines)]
pub fn run_hub(cmd: HubCmd) -> anyhow::Result<()> {
    match cmd {
        HubCmd::Init {
            vault_id,
            vault_root,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let hub = s.hub_init().map_err(|err| hub_fail(&err, json))?;
            if json {
                return print_json_or_debug(&hub, true);
            }
            println!("hub_id: {}", hub.header.id.as_str());
            println!("protocol: {}", hub.protocol_version);
            Ok(())
        }
        HubCmd::Invite {
            vault_id,
            vault_root,
            project_id,
            name,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let (invitation, code) = s
                .hub_invite(OpaqueId::new(project_id), name)
                .map_err(|err| hub_fail(&err, json))?;
            if json {
                return print_json_or_debug(&(invitation, code), true);
            }
            println!("invitation_id: {}", invitation.header.id.as_str());
            println!("project_id: {}", invitation.project_id.as_str());
            println!("code (shown once): {}", serde_json::to_string(&code)?);
            Ok(())
        }
        HubCmd::RevokeInvitation {
            vault_id,
            vault_root,
            invitation_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let inv = s
                .hub_invitation_revoke(OpaqueId::new(invitation_id))
                .map_err(|err| hub_fail(&err, json))?;
            if json {
                return print_json_or_debug(&inv, true);
            }
            println!("{} {}", inv.header.id.as_str(), inv.status.as_str());
            Ok(())
        }
        HubCmd::RevokeDevice {
            vault_id,
            vault_root,
            device_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let device = s
                .hub_device_revoke(OpaqueId::new(device_id))
                .map_err(|err| hub_fail(&err, json))?;
            if json {
                return print_json_or_debug(&device, true);
            }
            println!("{} {}", device.header.id.as_str(), device.status.as_str());
            Ok(())
        }
        HubCmd::Status {
            vault_id,
            vault_root,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let status = s.hub_status().map_err(|err| hub_fail(&err, json))?;
            if json {
                return print_json_or_debug(&status, true);
            }
            println!("hub_id: {}", status.hub.header.id.as_str());
            for d in &status.devices {
                println!(
                    "device {}\t{}\tproject {}\t{}",
                    d.header.id.as_str(),
                    d.status.as_str(),
                    d.project_id.as_str(),
                    d.display_name.escape_debug()
                );
            }
            for i in &status.invitations {
                println!(
                    "invitation {}\t{}\tproject {}",
                    i.header.id.as_str(),
                    i.status.as_str(),
                    i.project_id.as_str()
                );
            }
            for c in &status.checkpoints {
                println!(
                    "chain {}\tcursor {}\t{}",
                    c.project_id.as_str(),
                    c.cursor,
                    c.digest
                        .as_ref()
                        .map_or_else(|| "empty".to_owned(), |d| format!("sha256:{}", d.to_hex()))
                );
            }
            Ok(())
        }
        HubCmd::Serve {
            vault_id,
            vault_root,
            host_dir,
            endpoint,
            connections,
        } => {
            hub_sync::serve_hub(&vault_id, &vault_root, &host_dir, &endpoint, connections)
                .map_err(|err| hub_fail(&err, false))?;
            Ok(())
        }
        HubCmd::Join {
            vault_id,
            vault_root,
            endpoint,
            code_json,
            json,
        } => {
            let code: HubInvitationCode = serde_json::from_str(&code_json).map_err(|e| {
                hub_fail(
                    &AuthorityError::InvalidArgument {
                        message: format!("invitation code: {e}"),
                    },
                    json,
                )
            })?;
            let s = open_session(&vault_id, &vault_root, json)?;
            let mut t = IpcHubTransport::connect(&endpoint).map_err(|err| hub_fail(&err, json))?;
            let link = hub_sync::join(&s.hub_client_context(), &mut t, endpoint, code)
                .map_err(|err| hub_fail(&err, json))?;
            if json {
                return print_json_or_debug(&link, true);
            }
            println!("link_id: {}", link.header.id.as_str());
            println!("device_id: {}", link.device_id.as_str());
            println!("project_id: {}", link.project_id.as_str());
            Ok(())
        }
        HubCmd::Queue {
            vault_id,
            vault_root,
            link_id,
            intent_json,
            json,
        } => {
            let intent: SyncIntent = serde_json::from_str(&intent_json).map_err(|e| {
                hub_fail(
                    &AuthorityError::InvalidArgument {
                        message: format!("intent: {e}"),
                    },
                    json,
                )
            })?;
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let entry = s
                .hub_queue(OpaqueId::new(link_id), intent)
                .map_err(|err| hub_fail(&err, json))?;
            if json {
                return print_json_or_debug(&entry, true);
            }
            println!("queued seq {}", entry.envelope.body.seq);
            Ok(())
        }
        HubCmd::Sync {
            vault_id,
            vault_root,
            link_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let link_id = OpaqueId::new(link_id);
            let link = s
                .hub_link(link_id.clone())
                .map_err(|err| hub_fail(&err, json))?;
            let mut t =
                IpcHubTransport::connect(&link.endpoint).map_err(|err| hub_fail(&err, json))?;
            let report = hub_sync::sync(&s.hub_client_context(), &mut t, &link_id)
                .map_err(|err| hub_fail(&err, json))?;
            if json {
                return print_json_or_debug(&report, true);
            }
            println!(
                "submitted={} applied={} conflicts={} refused={} pulled={} cursor={}",
                report.submitted,
                report.applied,
                report.conflicts,
                report.refused,
                report.pulled,
                report.cursor
            );
            Ok(())
        }
        HubCmd::Links {
            vault_id,
            vault_root,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let links = s.hub_links().map_err(|err| hub_fail(&err, json))?;
            if json {
                return print_json_or_debug(&links, true);
            }
            for l in &links {
                println!(
                    "{}\thub {}\tdevice {}\tproject {}\tseq {}\tcursor {}{}",
                    l.header.id.as_str(),
                    l.hub_id.as_str(),
                    l.device_id.as_str(),
                    l.project_id.as_str(),
                    l.last_seq,
                    l.cursor,
                    if l.revoked { "\trevoked" } else { "" }
                );
            }
            Ok(())
        }
        HubCmd::Outbox {
            vault_id,
            vault_root,
            link_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let entries = s
                .hub_outbox(OpaqueId::new(link_id), false)
                .map_err(|err| hub_fail(&err, json))?;
            if json {
                return print_json_or_debug(&entries, true);
            }
            for e in &entries {
                println!(
                    "{}\t{}\t{}",
                    e.envelope.body.seq,
                    e.state.as_str(),
                    e.outcome
                        .as_ref()
                        .map_or_else(|| "pending".to_owned(), outcome_label)
                );
            }
            Ok(())
        }
        HubCmd::Mirror {
            vault_id,
            vault_root,
            link_id,
            after,
            limit,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let events = s
                .hub_mirror(OpaqueId::new(link_id), after, limit)
                .map_err(|err| hub_fail(&err, json))?;
            if json {
                return print_json_or_debug(&events, true);
            }
            for e in &events {
                println!("{}", event_label(e));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_core::CliSession;

    #[test]
    fn hub_commands_run_through_core_over_local_ipc() {
        let root = std::env::temp_dir().join(format!("medscale-084-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let hub_root = root.join("hub");
        let client_root = root.join("client");
        let endpoint = format!("medscale-hub-cli-{}", std::process::id());

        // Operator: a Project, a room with a thread, the Hub role, an
        // invitation (through Core, fresh sessions).
        let mut s = CliSession::connect("vault-hub").unwrap();
        s.open_synthetic_vault(&hub_root.display().to_string())
            .unwrap();
        let project = s
            .project_create("study".to_owned(), None)
            .unwrap()
            .header
            .id;
        drop(s);
        for json in [true, false] {
            let _ = run_hub(HubCmd::Status {
                vault_id: "vault-hub".to_owned(),
                vault_root: hub_root.clone(),
                json,
            });
        }
        run_hub(HubCmd::Init {
            vault_id: "vault-hub".to_owned(),
            vault_root: hub_root.clone(),
            json: false,
        })
        .unwrap();
        let mut s = CliSession::connect("vault-hub").unwrap();
        s.open_synthetic_vault(&hub_root.display().to_string())
            .unwrap();
        let (_, code) = s.hub_invite(project.clone(), "Laptop".to_owned()).unwrap();
        drop(s);
        let code_json = serde_json::to_string(&code).unwrap();

        // The Hub serves three connections (join, sync, and a raw peer) in a
        // thread.
        let (vr, hd, ep) = (hub_root.clone(), root.join("host"), endpoint.clone());
        let server = std::thread::spawn(move || {
            run_hub(HubCmd::Serve {
                vault_id: "vault-hub".to_owned(),
                vault_root: vr,
                host_dir: hd,
                endpoint: ep,
                connections: 3,
            })
        });

        run_hub(HubCmd::Join {
            vault_id: "vault-client".to_owned(),
            vault_root: client_root.clone(),
            endpoint: endpoint.clone(),
            code_json,
            json: true,
        })
        .unwrap();
        let mut c = CliSession::connect("vault-client").unwrap();
        c.open_synthetic_vault(&client_root.display().to_string())
            .unwrap();
        let link = c.hub_links().unwrap().remove(0);
        drop(c);
        let link_id = link.header.id.as_str().to_owned();
        // A malformed intent is refused before anything is queued.
        assert!(
            run_hub(HubCmd::Queue {
                vault_id: "vault-client".to_owned(),
                vault_root: client_root.clone(),
                link_id: link_id.clone(),
                intent_json: "{\"kind\":\"room_delete\"}".to_owned(),
                json: true,
            })
            .is_err()
        );
        run_hub(HubCmd::Sync {
            vault_id: "vault-client".to_owned(),
            vault_root: client_root.clone(),
            link_id: link_id.clone(),
            json: false,
        })
        .unwrap();
        // A local peer without a session cannot use the Hub endpoint for
        // anything but Hub bootstrap and sync (no lease-holder reads).
        let mut peer = medscale_core::ipc::HostIpcClient::connect(&endpoint).unwrap();
        let status = peer
            .dispatch(medscale_contracts::envelopes::AuthorityRequest::new(
                OpaqueId::new("peer-1"),
                medscale_contracts::objects::VaultId::new("vault-hub"),
                medscale_contracts::objects::RealmId::new("cli-realm"),
                medscale_contracts::objects::AuthorityScopeId::new("cli-scope"),
                medscale_contracts::envelopes::Capability::HubRead,
                medscale_contracts::envelopes::RequestBody::HubStatus,
            ))
            .unwrap();
        assert_eq!(status.result, Err(AuthorityError::Unauthorized));
        drop(peer);
        server.join().unwrap().unwrap();

        for json in [true, false] {
            run_hub(HubCmd::Links {
                vault_id: "vault-client".to_owned(),
                vault_root: client_root.clone(),
                json,
            })
            .unwrap();
            run_hub(HubCmd::Outbox {
                vault_id: "vault-client".to_owned(),
                vault_root: client_root.clone(),
                link_id: link_id.clone(),
                json,
            })
            .unwrap();
            run_hub(HubCmd::Mirror {
                vault_id: "vault-client".to_owned(),
                vault_root: client_root.clone(),
                link_id: link_id.clone(),
                after: 0,
                limit: 100,
                json,
            })
            .unwrap();
            run_hub(HubCmd::Status {
                vault_id: "vault-hub".to_owned(),
                vault_root: hub_root.clone(),
                json,
            })
            .unwrap();
        }
        let mut c = CliSession::connect("vault-client").unwrap();
        c.open_synthetic_vault(&client_root.display().to_string())
            .unwrap();
        let mirror = c.hub_mirror(link.header.id.clone(), 0, 100).unwrap();
        assert_eq!(mirror.len(), 1);
        drop(c);
        // Revocation through the CLI.
        run_hub(HubCmd::RevokeDevice {
            vault_id: "vault-hub".to_owned(),
            vault_root: hub_root.clone(),
            device_id: link.device_id.as_str().to_owned(),
            json: true,
        })
        .unwrap();
        assert!(
            run_hub(HubCmd::RevokeDevice {
                vault_id: "vault-hub".to_owned(),
                vault_root: hub_root,
                device_id: link.device_id.as_str().to_owned(),
                json: false,
            })
            .is_err()
        );
        let _ = project;
    }
}
