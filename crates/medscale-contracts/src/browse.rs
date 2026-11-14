//! Governed Browse contracts (Spec 080).
//!
//! Read-only, evidence-bearing web retrieval through the admitted network
//! broker. No type here holds a raw credential, cookie or ambient header;
//! `CredentialHandleRef` is an opaque handle that this spec never resolves.
//! Fetched content is evidence only: it can never change instructions,
//! capabilities or authority (`security.md`).

use serde::{Deserialize, Serialize};

use crate::objects::{DigestSha256, ObjectHeader, OpaqueId};
use crate::project_graph::{ProjectRevision, check_revision, initial_revision};

/// Durable schema version for every Browse object (Spec 080 v1).
pub const BROWSE_SCHEMA_VERSION: u32 = 1;

pub const MAX_REDIRECTS: usize = 5;
pub const MAX_RESPONSE_BYTES: usize = 2_097_152;
pub const EXCERPT_MAX_CHARS: usize = 4_000;
pub const REQUEST_TIMEOUT_MS: u64 = 15_000;
pub const QUERY_MAX_CHARS: usize = 512;
pub const URL_MAX_CHARS: usize = 2_048;
pub const HOST_MAX_CHARS: usize = 253;
pub const PATH_PREFIX_MAX_CHARS: usize = 512;

macro_rules! closed_vocabulary {
    ($name:ident, $what:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            pub fn parse(value: &str) -> Result<Self, String> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    other => Err(format!(concat!("unknown ", $what, " {}"), other)),
                }
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowseIntentKind {
    FetchUrl,
    Search,
}

closed_vocabulary!(BrowseIntentKind, "browse intent", {
    FetchUrl => "fetch_url",
    Search => "search",
});

/// Routing order (decision register Q43): brokered HTTP first; search,
/// deterministic browser and agentic browser report `Unavailable` until an
/// admitted provider / bounded worker exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowseRoute {
    HttpFetch,
    Search,
    DeterministicBrowser,
    AgenticBrowser,
}

closed_vocabulary!(BrowseRoute, "browse route", {
    HttpFetch => "http_fetch",
    Search => "search",
    DeterministicBrowser => "deterministic_browser",
    AgenticBrowser => "agentic_browser",
});

impl BrowseRoute {
    /// Whether this build can execute the route.
    #[must_use]
    pub const fn available(self) -> bool {
        matches!(self, Self::HttpFetch)
    }

    /// Why an unavailable route is unavailable (fixed text).
    #[must_use]
    pub const fn unavailable_reason(self) -> Option<&'static str> {
        match self {
            Self::HttpFetch => None,
            Self::Search => Some("no admitted search provider"),
            Self::DeterministicBrowser | Self::AgenticBrowser => {
                Some("browser execution requires the bounded worker (Spec 085)")
            }
        }
    }
}

/// Whether a route can run in this build, and why not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowseRouteStatus {
    pub route: BrowseRoute,
    pub available: bool,
    pub reason: Option<String>,
}

impl BrowseRouteStatus {
    /// The status of every route, in Q43 order.
    #[must_use]
    pub fn all() -> Vec<Self> {
        BrowseRoute::ALL
            .iter()
            .map(|route| Self {
                route: *route,
                available: route.available(),
                reason: route.unavailable_reason().map(str::to_owned),
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowseDenyReason {
    EmptyAllowlist,
    HostNotAllowlisted,
    SchemeNotHttps,
    PortNotAllowed,
    IpLiteralHost,
    PrivateNetworkTarget,
    MalformedUrl,
    SensitiveSpanInRequest,
    PrivacyGateDenied,
    RouteUnavailable,
    RedirectLimit,
    RedirectTargetDenied,
    ResponseTooLarge,
    ContentTypeNotAllowed,
    Timeout,
    TransportFailed,
}

closed_vocabulary!(BrowseDenyReason, "browse deny reason", {
    EmptyAllowlist => "empty_allowlist",
    HostNotAllowlisted => "host_not_allowlisted",
    SchemeNotHttps => "scheme_not_https",
    PortNotAllowed => "port_not_allowed",
    IpLiteralHost => "ip_literal_host",
    PrivateNetworkTarget => "private_network_target",
    MalformedUrl => "malformed_url",
    SensitiveSpanInRequest => "sensitive_span_in_request",
    PrivacyGateDenied => "privacy_gate_denied",
    RouteUnavailable => "route_unavailable",
    RedirectLimit => "redirect_limit",
    RedirectTargetDenied => "redirect_target_denied",
    ResponseTooLarge => "response_too_large",
    ContentTypeNotAllowed => "content_type_not_allowed",
    Timeout => "timeout",
    TransportFailed => "transport_failed",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowseOutcome {
    Allow,
    Deny,
}

closed_vocabulary!(BrowseOutcome, "browse outcome", {
    Allow => "allow",
    Deny => "deny",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowsePolicyDecision {
    pub outcome: BrowseOutcome,
    pub reason: Option<BrowseDenyReason>,
    /// The Spec 079 egress decision consulted for Project content, if any.
    pub privacy_decision_id: Option<OpaqueId>,
}

impl BrowsePolicyDecision {
    #[must_use]
    pub fn allow(privacy_decision_id: Option<OpaqueId>) -> Self {
        Self {
            outcome: BrowseOutcome::Allow,
            reason: None,
            privacy_decision_id,
        }
    }

    #[must_use]
    pub fn deny(reason: BrowseDenyReason, privacy_decision_id: Option<OpaqueId>) -> Self {
        Self {
            outcome: BrowseOutcome::Deny,
            reason: Some(reason),
            privacy_decision_id,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        match (self.outcome, self.reason) {
            (BrowseOutcome::Allow, None) | (BrowseOutcome::Deny, Some(_)) => Ok(()),
            _ => Err("allow carries no reason; deny carries exactly one".to_owned()),
        }
    }
}

/// One operator-approved destination. Empty allowlist denies everything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowseAllowlistEntry {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    /// Lowercase DNS host name (never an IP literal).
    pub host: String,
    /// Path prefix, beginning with `/`.
    pub path_prefix: String,
    pub enabled: bool,
}

impl BrowseAllowlistEntry {
    pub fn new(
        header: ObjectHeader,
        project_id: OpaqueId,
        host: String,
        path_prefix: String,
    ) -> Result<Self, String> {
        let entry = Self {
            header,
            revision: initial_revision(),
            project_id,
            host: host.to_ascii_lowercase(),
            path_prefix,
            enabled: true,
        };
        entry.validate()?;
        Ok(entry)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !is_dns_host_name(&self.host) {
            return Err("allowlist host must be a DNS host name".to_owned());
        }
        if !self.path_prefix.starts_with('/')
            || self.path_prefix.len() > PATH_PREFIX_MAX_CHARS
            || self
                .path_prefix
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err("allowlist path prefix must start with / and be bounded".to_owned());
        }
        if self.path_prefix.contains(['?', '#']) || !is_unambiguous_url_path(&self.path_prefix) {
            return Err(
                "allowlist path prefix must not carry a query, fragment or dot segment".to_owned(),
            );
        }
        Ok(())
    }

    /// True when `host`/`path` fall under this enabled entry. The prefix
    /// matches whole segments: `/docs` covers `/docs`, `/docs/a` and
    /// `/docs?q`, never `/docs-private`.
    #[must_use]
    pub fn matches(&self, host: &str, path: &str) -> bool {
        if !self.enabled || self.host != host {
            return false;
        }
        let Some(rest) = path.strip_prefix(self.path_prefix.as_str()) else {
            return false;
        };
        self.path_prefix.ends_with('/')
            || rest.is_empty()
            || rest.starts_with('/')
            || rest.starts_with('?')
    }

    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }
}

/// True when a URL path (query ignored) means the same thing to every server,
/// so an allowlist prefix check on it cannot be bypassed by normalization.
/// Refused: backslashes, dot segments (`.`/`..`, also percent-encoded,
/// double-encoded or with `;params`), and encoded `/` or `\`.
#[must_use]
pub fn is_unambiguous_url_path(path: &str) -> bool {
    let path = path.split('?').next().unwrap_or("");
    if path.contains('\\') {
        return false;
    }
    path.split('/').all(|segment| {
        let decoded = percent_decode_once(segment);
        let core = decoded.split(';').next().unwrap_or("").to_ascii_lowercase();
        !decoded.contains(['/', '\\'])
            && !matches!(
                core.as_str(),
                "." | ".." | "%2e" | "%2e%2e" | ".%2e" | "%2e."
            )
    })
}

fn percent_decode_once(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && bytes[i + 1].is_ascii_hexdigit()
            && bytes[i + 2].is_ascii_hexdigit()
        {
            let hex = |b: u8| (b as char).to_digit(16).unwrap_or(0) as u8;
            out.push(hex(bytes[i + 1]) * 16 + hex(bytes[i + 2]));
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A lowercase DNS host name with at least one dot and no IP-literal shape.
#[must_use]
pub fn is_dns_host_name(host: &str) -> bool {
    if host.is_empty() || host.len() > HOST_MAX_CHARS || !host.contains('.') {
        return false;
    }
    if host.split('.').any(|label| {
        label.is_empty() || label.len() > 63 || label.starts_with('-') || label.ends_with('-')
    }) {
        return false;
    }
    if !host
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
    {
        return false;
    }
    // All-numeric labels would make an IPv4 literal (or a confusable form).
    let last = host.rsplit('.').next().unwrap_or("");
    !last.bytes().all(|b| b.is_ascii_digit())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowseRequest {
    pub project_id: OpaqueId,
    pub intent: BrowseIntentKind,
    pub url: Option<String>,
    pub query: Option<String>,
    /// Project content that would be sent out (for example a search query
    /// derived from an artifact). Gated by the Spec 079 egress decision.
    pub context_artifact_id: Option<OpaqueId>,
}

impl BrowseRequest {
    pub fn validate(&self) -> Result<(), String> {
        match self.intent {
            BrowseIntentKind::FetchUrl => {
                let url = self.url.as_deref().ok_or("fetch_url needs a url")?;
                if url.is_empty() || url.chars().count() > URL_MAX_CHARS {
                    return Err("url must be 1..=2048 chars".to_owned());
                }
                if self.query.is_some() {
                    return Err("fetch_url takes no query".to_owned());
                }
            }
            BrowseIntentKind::Search => {
                let query = self.query.as_deref().ok_or("search needs a query")?;
                if query.trim().is_empty() || query.chars().count() > QUERY_MAX_CHARS {
                    return Err("query must be 1..=512 chars".to_owned());
                }
                if self.url.is_some() {
                    return Err("search takes no url".to_owned());
                }
            }
        }
        for text in [self.url.as_deref(), self.query.as_deref()]
            .into_iter()
            .flatten()
        {
            if text.chars().any(char::is_control) {
                return Err("request text must not contain control characters".to_owned());
            }
        }
        Ok(())
    }

    /// The route this intent uses (Q43 order).
    #[must_use]
    pub const fn route(&self) -> BrowseRoute {
        match self.intent {
            BrowseIntentKind::FetchUrl => BrowseRoute::HttpFetch,
            BrowseIntentKind::Search => BrowseRoute::Search,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowseSessionState {
    Completed,
    Denied,
    Failed,
    AwaitingHumanTakeover,
    Cancelled,
}

closed_vocabulary!(BrowseSessionState, "browse session state", {
    Completed => "completed",
    Denied => "denied",
    Failed => "failed",
    AwaitingHumanTakeover => "awaiting_human_takeover",
    Cancelled => "cancelled",
});

impl BrowseSessionState {
    /// Only a session awaiting human takeover may still change (to
    /// `Cancelled`); every other state is final.
    #[must_use]
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!((self, next), (Self::AwaitingHumanTakeover, Self::Cancelled))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowseNavigationStep {
    pub seq: u32,
    pub url: String,
    pub http_status: Option<u16>,
    pub redirect_to: Option<String>,
    pub decision: BrowsePolicyDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HumanTakeoverReason {
    LoginRequired,
    AccessForbidden,
}

closed_vocabulary!(HumanTakeoverReason, "human takeover reason", {
    LoginRequired => "login_required",
    AccessForbidden => "access_forbidden",
});

/// A login or access boundary a human must handle outside MedScale. No
/// credential is ever supplied by Browse itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanTakeoverRequest {
    pub url: String,
    pub reason: HumanTakeoverReason,
}

/// Opaque, origin-bound credential handle (decision register Q44). This
/// spec defines the type only; nothing resolves it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialHandleRef {
    pub handle_id: OpaqueId,
    pub origin: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowseSession {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    pub request: BrowseRequest,
    pub route: BrowseRoute,
    pub state: BrowseSessionState,
    pub decision: BrowsePolicyDecision,
    pub steps: Vec<BrowseNavigationStep>,
    pub takeover: Option<HumanTakeoverRequest>,
}

impl BrowseSession {
    pub fn validate(&self) -> Result<(), String> {
        self.request.validate()?;
        self.decision.validate()?;
        if self.route != self.request.route() {
            return Err("session route does not match its request".to_owned());
        }
        if self.steps.len() > MAX_REDIRECTS + 1 {
            return Err("too many navigation steps".to_owned());
        }
        for (i, step) in self.steps.iter().enumerate() {
            step.decision.validate()?;
            if step.seq as usize != i + 1 {
                return Err("navigation steps must be numbered 1..n".to_owned());
            }
        }
        match (self.state, &self.takeover) {
            (BrowseSessionState::AwaitingHumanTakeover, None) => {
                return Err("awaiting takeover needs a takeover request".to_owned());
            }
            (BrowseSessionState::Completed | BrowseSessionState::Denied, Some(_)) => {
                return Err("only takeover sessions carry a takeover request".to_owned());
            }
            _ => {}
        }
        if self.state == BrowseSessionState::Denied && self.decision.outcome == BrowseOutcome::Allow
        {
            let last_denied = self
                .steps
                .last()
                .is_some_and(|s| s.decision.outcome == BrowseOutcome::Deny);
            if !last_denied {
                return Err("a denied session records the denying decision".to_owned());
            }
        }
        Ok(())
    }

    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }
}

/// Inert evidence from one fetched page. `excerpt` is display text only;
/// instruction-like content is flagged, never followed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowseEvidenceItem {
    pub header: ObjectHeader,
    pub session_id: OpaqueId,
    pub final_url: String,
    pub content_type: String,
    pub byte_length: u64,
    pub content_digest: DigestSha256,
    pub excerpt: String,
    pub instruction_like_content_flagged: bool,
}

impl BrowseEvidenceItem {
    pub fn validate(&self) -> Result<(), String> {
        if self.excerpt.chars().count() > EXCERPT_MAX_CHARS {
            return Err("excerpt exceeds bound".to_owned());
        }
        if self.byte_length > MAX_RESPONSE_BYTES as u64 {
            return Err("evidence exceeds response bound".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadCandidateStatus {
    Quarantined,
}

closed_vocabulary!(DownloadCandidateStatus, "download candidate status", {
    Quarantined => "quarantined",
});

/// Bytes held for the normal Spec 075 import/validation path; never a
/// trusted artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowseDownloadCandidate {
    pub header: ObjectHeader,
    pub session_id: OpaqueId,
    pub final_url: String,
    pub content_type: String,
    pub byte_length: u64,
    pub content_digest: DigestSha256,
    pub status: DownloadCandidateStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowseLimitation {
    /// Evidence is what one server returned once; it is not verified truth.
    ContentUnverified,
    /// Only the brokered HTTP route ran; no page script executed.
    NoScriptExecution,
}

closed_vocabulary!(BrowseLimitation, "browse limitation", {
    ContentUnverified => "content_unverified",
    NoScriptExecution => "no_script_execution",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowseReceipt {
    pub header: ObjectHeader,
    pub session_id: OpaqueId,
    pub project_id: OpaqueId,
    pub request_digest: DigestSha256,
    pub route: BrowseRoute,
    pub final_state: BrowseSessionState,
    pub step_count: u32,
    pub evidence_ids: Vec<OpaqueId>,
    pub download_ids: Vec<OpaqueId>,
    pub limitations: Vec<BrowseLimitation>,
}

/// A session with everything it produced, as returned to surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowseSessionView {
    pub session: BrowseSession,
    pub receipt: BrowseReceipt,
    pub evidence: Vec<BrowseEvidenceItem>,
    pub downloads: Vec<BrowseDownloadCandidate>,
}

/// Canonical digest of a request (JSON of the typed value).
#[must_use]
pub fn request_digest(request: &BrowseRequest) -> DigestSha256 {
    let bytes = serde_json::to_vec(request).unwrap_or_default();
    DigestSha256::of(&bytes)
}

/// Content types the brokered route keeps as evidence text.
pub const EVIDENCE_CONTENT_TYPES: &[&str] = &["text/html", "text/plain", "application/json"];
/// Content types kept only as quarantined download candidates.
pub const DOWNLOAD_CONTENT_TYPES: &[&str] = &["application/pdf", "text/csv"];

/// The media type without parameters, lowercased.
#[must_use]
pub fn media_type(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{AuthorityScopeId, RealmId};

    fn h(id: &str) -> ObjectHeader {
        ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: BROWSE_SCHEMA_VERSION,
            realm_id: RealmId::new("realm"),
            authority_scope_id: AuthorityScopeId::new("scope"),
        }
    }

    fn fetch(url: &str) -> BrowseRequest {
        BrowseRequest {
            project_id: OpaqueId::new("p"),
            intent: BrowseIntentKind::FetchUrl,
            url: Some(url.to_owned()),
            query: None,
            context_artifact_id: None,
        }
    }

    macro_rules! assert_round_trips {
        ($($ty:ty),+) => {
            $(for value in <$ty>::ALL {
                assert_eq!(<$ty>::parse(value.as_str()).unwrap(), *value);
                assert_eq!(serde_json::to_string(value).unwrap(), format!("\"{}\"", value.as_str()));
            }
            assert!(<$ty>::parse("nope").is_err());)+
        };
    }

    #[test]
    fn vocabularies_round_trip_and_are_closed() {
        assert_round_trips!(
            BrowseIntentKind,
            BrowseRoute,
            BrowseDenyReason,
            BrowseOutcome,
            BrowseSessionState,
            HumanTakeoverReason,
            DownloadCandidateStatus,
            BrowseLimitation
        );
    }

    #[test]
    fn only_http_fetch_is_available() {
        for route in BrowseRoute::ALL {
            assert_eq!(route.available(), *route == BrowseRoute::HttpFetch);
            assert_eq!(route.unavailable_reason().is_none(), route.available());
        }
    }

    #[test]
    fn dns_host_names_exclude_ip_literals_and_junk() {
        assert!(is_dns_host_name("pubmed.ncbi.nlm.nih.gov"));
        assert!(is_dns_host_name("example.org"));
        for bad in [
            "",
            "localhost",
            "127.0.0.1",
            "10.0.0.1",
            "1.2.3.4",
            "[::1]",
            "Example.org",
            "exa mple.org",
            "-bad.org",
            "bad-.org",
            "a..b",
            "host.123",
        ] {
            assert!(!is_dns_host_name(bad), "{bad}");
        }
    }

    #[test]
    fn allowlist_entries_validate_and_match() {
        let e = BrowseAllowlistEntry::new(
            h("a"),
            OpaqueId::new("p"),
            "Example.ORG".to_owned(),
            "/docs/".to_owned(),
        )
        .unwrap();
        assert_eq!(e.host, "example.org");
        assert!(e.matches("example.org", "/docs/a"));
        assert!(!e.matches("example.org", "/other"));
        assert!(!e.matches("evil.org", "/docs/a"));
        // Prefixes match whole segments.
        let bare = BrowseAllowlistEntry::new(
            h("d"),
            OpaqueId::new("p"),
            "example.org".to_owned(),
            "/docs".to_owned(),
        )
        .unwrap();
        for ok in ["/docs", "/docs/a", "/docs?q=1"] {
            assert!(bare.matches("example.org", ok), "{ok}");
        }
        assert!(!bare.matches("example.org", "/docs-private"));
        assert!(!bare.matches("example.org", "/docsx/a"));
        // Prefixes cannot carry dot segments, queries or backslashes.
        for bad in [
            "/docs/../admin",
            "/docs/%2e%2e/",
            "/a\\b",
            "/docs?x",
            "/docs#x",
        ] {
            assert!(
                BrowseAllowlistEntry::new(
                    h("e"),
                    OpaqueId::new("p"),
                    "example.org".to_owned(),
                    bad.to_owned()
                )
                .is_err(),
                "{bad}"
            );
        }
        let mut off = e.clone();
        off.enabled = false;
        assert!(!off.matches("example.org", "/docs/a"));
        assert!(
            BrowseAllowlistEntry::new(
                h("b"),
                OpaqueId::new("p"),
                "127.0.0.1".to_owned(),
                "/".to_owned()
            )
            .is_err()
        );
        assert!(
            BrowseAllowlistEntry::new(
                h("c"),
                OpaqueId::new("p"),
                "example.org".to_owned(),
                "docs".to_owned()
            )
            .is_err()
        );
    }

    #[test]
    fn url_paths_with_normalization_ambiguity_are_refused() {
        for ok in [
            "/",
            "/docs/a.b",
            "/docs/..x",
            "/a/.well-known/x",
            "/q?next=../x",
            "/%41bc",
            "/100%",
        ] {
            assert!(is_unambiguous_url_path(ok), "{ok}");
        }
        for bad in [
            "/docs/../admin",
            "/docs/./a",
            "/docs/..",
            "/docs/%2e%2e/admin",
            "/docs/%2E%2E/admin",
            "/docs/.%2e/admin",
            "/docs/%252e%252e/admin",
            "/docs/..;/admin",
            "/docs/..\\admin",
            "/docs%2fadmin",
            "/docs%5cadmin",
        ] {
            assert!(!is_unambiguous_url_path(bad), "{bad}");
        }
    }

    #[test]
    fn requests_validate_by_intent() {
        assert!(fetch("https://example.org/").validate().is_ok());
        assert!(fetch("").validate().is_err());
        assert!(fetch(&"a".repeat(URL_MAX_CHARS + 1)).validate().is_err());
        assert!(fetch("https://example.org/\u{7}").validate().is_err());
        let mut both = fetch("https://example.org/");
        both.query = Some("x".to_owned());
        assert!(both.validate().is_err());
        let search = BrowseRequest {
            project_id: OpaqueId::new("p"),
            intent: BrowseIntentKind::Search,
            url: None,
            query: Some("statin myopathy".to_owned()),
            context_artifact_id: None,
        };
        assert!(search.validate().is_ok());
        assert_eq!(search.route(), BrowseRoute::Search);
        assert_eq!(
            fetch("https://example.org/").route(),
            BrowseRoute::HttpFetch
        );
        assert_ne!(
            request_digest(&search),
            request_digest(&fetch("https://example.org/"))
        );
    }

    #[test]
    fn decisions_and_sessions_hold_their_invariants() {
        assert!(BrowsePolicyDecision::allow(None).validate().is_ok());
        assert!(
            BrowsePolicyDecision::deny(BrowseDenyReason::Timeout, None)
                .validate()
                .is_ok()
        );
        let mut bad = BrowsePolicyDecision::allow(None);
        bad.reason = Some(BrowseDenyReason::Timeout);
        assert!(bad.validate().is_err());

        let mut s = BrowseSession {
            header: h("s"),
            revision: 1,
            project_id: OpaqueId::new("p"),
            request: fetch("https://example.org/"),
            route: BrowseRoute::HttpFetch,
            state: BrowseSessionState::Completed,
            decision: BrowsePolicyDecision::allow(None),
            steps: vec![BrowseNavigationStep {
                seq: 1,
                url: "https://example.org/".to_owned(),
                http_status: Some(200),
                redirect_to: None,
                decision: BrowsePolicyDecision::allow(None),
            }],
            takeover: None,
        };
        assert!(s.validate().is_ok());
        s.state = BrowseSessionState::AwaitingHumanTakeover;
        assert!(s.validate().is_err());
        s.takeover = Some(HumanTakeoverRequest {
            url: "https://example.org/".to_owned(),
            reason: HumanTakeoverReason::LoginRequired,
        });
        assert!(s.validate().is_ok());
        s.route = BrowseRoute::Search;
        assert!(s.validate().is_err());
        s.route = BrowseRoute::HttpFetch;
        s.steps[0].seq = 2;
        assert!(s.validate().is_err());
        assert!(
            BrowseSessionState::AwaitingHumanTakeover
                .can_transition_to(BrowseSessionState::Cancelled)
        );
        assert!(!BrowseSessionState::Completed.can_transition_to(BrowseSessionState::Cancelled));
    }

    #[test]
    fn media_type_strips_parameters() {
        assert_eq!(media_type("Text/HTML; charset=utf-8"), "text/html");
        assert_eq!(media_type(""), "");
    }
}
