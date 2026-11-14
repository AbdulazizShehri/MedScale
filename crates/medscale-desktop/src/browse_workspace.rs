//! Governed Browse view-models (Spec 080).
//!
//! Desktop reaches Browse only through the Core-owned `CliSession`; it holds
//! no network code. Every function maps one typed Core result to plain rows
//! plus an explicit status. Scope: the Project's allowlist (add/disable),
//! route availability, session history, and a fetch of one URL. Evidence is
//! shown as inert text; instruction-like content is labelled, never acted on.

use medscale_contracts::browse::{BrowseIntentKind, BrowseRequest, BrowseSessionView};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_core::CliSession;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AllowRowVm {
    pub id: String,
    pub target: String,
    pub enabled: bool,
    pub revision: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionRowVm {
    pub id: String,
    pub url: String,
    pub state: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BrowseOverviewVm {
    pub allowlist: Vec<AllowRowVm>,
    pub sessions: Vec<SessionRowVm>,
    pub routes: String,
}

/// Result of one fetch, shaped for display.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FetchResultVm {
    pub session: SessionRowVm,
    pub excerpt: String,
    pub flagged: bool,
    pub downloads: usize,
}

#[must_use]
pub fn status_message(err: &AuthorityError) -> &'static str {
    match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => "Denied by Core authority",
        AuthorityError::NotFound => "Missing: not found in this vault",
        AuthorityError::Conflict { .. } => "Conflict: stale revision or already final",
        AuthorityError::InvalidArgument { .. } => "Invalid: rejected before any request",
        AuthorityError::Corrupt { .. } => "Corrupt: integrity check failed",
        _ => "Unavailable: vault or Core not ready",
    }
}

fn session_row(v: &medscale_contracts::browse::BrowseSession) -> SessionRowVm {
    SessionRowVm {
        id: v.header.id.as_str().to_owned(),
        url: v
            .request
            .url
            .clone()
            .or_else(|| v.request.query.clone())
            .unwrap_or_default(),
        state: v.state.as_str().to_owned(),
        reason: v
            .decision
            .reason
            .map_or_else(String::new, |r| r.as_str().to_owned()),
    }
}

pub fn refresh(
    session: &mut CliSession,
    project_id: &str,
) -> Result<BrowseOverviewVm, AuthorityError> {
    let project = OpaqueId::new(project_id);
    let allowlist = session
        .browse_allowlist_list(project.clone())?
        .iter()
        .map(|e| AllowRowVm {
            id: e.header.id.as_str().to_owned(),
            target: format!("{}{}", e.host, e.path_prefix),
            enabled: e.enabled,
            revision: e.revision,
        })
        .collect();
    let sessions = session
        .browse_session_list(project)?
        .iter()
        .rev()
        .map(session_row)
        .collect();
    let routes = session
        .browse_routes()?
        .iter()
        .map(|r| {
            format!(
                "{} {}",
                r.route.as_str(),
                if r.available {
                    "available"
                } else {
                    "unavailable"
                }
            )
        })
        .collect::<Vec<_>>()
        .join(" · ");
    Ok(BrowseOverviewVm {
        allowlist,
        sessions,
        routes,
    })
}

pub fn add_allowed_host(
    session: &mut CliSession,
    project_id: &str,
    host: &str,
    path_prefix: &str,
) -> Result<AllowRowVm, AuthorityError> {
    let prefix = if path_prefix.trim().is_empty() {
        "/"
    } else {
        path_prefix.trim()
    };
    let e = session.browse_allowlist_add(
        OpaqueId::new(project_id),
        host.trim().to_owned(),
        prefix.to_owned(),
    )?;
    Ok(AllowRowVm {
        id: e.header.id.as_str().to_owned(),
        target: format!("{}{}", e.host, e.path_prefix),
        enabled: e.enabled,
        revision: e.revision,
    })
}

pub fn disable_allowed_host(
    session: &mut CliSession,
    entry_id: &str,
    expected_revision: u64,
) -> Result<(), AuthorityError> {
    session
        .browse_allowlist_disable(OpaqueId::new(entry_id), expected_revision)
        .map(|_| ())
}

fn fetch_vm(v: &BrowseSessionView) -> FetchResultVm {
    FetchResultVm {
        session: session_row(&v.session),
        excerpt: v
            .evidence
            .first()
            .map(|e| e.excerpt.clone())
            .unwrap_or_default(),
        flagged: v
            .evidence
            .iter()
            .any(|e| e.instruction_like_content_flagged),
        downloads: v.downloads.len(),
    }
}

pub fn fetch(
    session: &mut CliSession,
    project_id: &str,
    url: &str,
) -> Result<FetchResultVm, AuthorityError> {
    let v = session.browse_run(BrowseRequest {
        project_id: OpaqueId::new(project_id),
        intent: BrowseIntentKind::FetchUrl,
        url: Some(url.trim().to_owned()),
        query: None,
        context_artifact_id: None,
    })?;
    Ok(fetch_vm(&v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browse_view_models_flow_through_a_real_core_session() {
        let root =
            std::env::temp_dir().join(format!("medscale-080-desktop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut s = CliSession::connect("vault-080-desktop").unwrap();
        s.use_offline_browse_fixture();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let project = s
            .project_create("browse".to_owned(), None)
            .unwrap()
            .header
            .id;
        let p = project.as_str();

        let denied = fetch(&mut s, p, CliSession::OFFLINE_BROWSE_FIXTURE_URL).unwrap();
        assert_eq!(denied.session.state, "denied");
        assert_eq!(denied.session.reason, "empty_allowlist");
        let err = add_allowed_host(&mut s, p, "10.0.0.8", "/").unwrap_err();
        assert_eq!(status_message(&err), "Invalid: rejected before any request");
        let row = add_allowed_host(&mut s, p, "fixture.medscale.test", "").unwrap();
        assert_eq!(row.target, "fixture.medscale.test/");
        let ok = fetch(&mut s, p, CliSession::OFFLINE_BROWSE_FIXTURE_URL).unwrap();
        assert_eq!(ok.session.state, "completed");
        assert!(ok.flagged, "instruction-like fixture text is labelled");
        assert!(ok.excerpt.contains("Statin-associated muscle symptoms"));

        let overview = refresh(&mut s, p).unwrap();
        assert_eq!(overview.allowlist.len(), 1);
        assert_eq!(overview.sessions.len(), 2);
        assert_eq!(overview.sessions[0].state, "completed", "newest first");
        assert!(overview.routes.contains("http_fetch available"));
        assert!(overview.routes.contains("search unavailable"));

        disable_allowed_host(&mut s, &row.id, 1).unwrap();
        let again = fetch(&mut s, p, CliSession::OFFLINE_BROWSE_FIXTURE_URL).unwrap();
        assert_eq!(again.session.reason, "empty_allowlist");
        let err = disable_allowed_host(&mut s, &row.id, 1).unwrap_err();
        assert_eq!(
            status_message(&err),
            "Conflict: stale revision or already final"
        );
    }
}
