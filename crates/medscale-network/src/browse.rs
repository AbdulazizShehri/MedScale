//! Governed Browse transport (Spec 080). The only code that can send a
//! Browse request to the network.
//!
//! Defense in depth against server-side request forgery:
//! 1. `validate_url` accepts only `https`, port 443, and a DNS host name
//!    (no IP literal, no userinfo);
//! 2. the live transport resolves names through `PublicOnlyResolver`, which
//!    drops every forbidden address at connect time (so a DNS answer cannot
//!    be swapped between check and connect);
//! 3. redirects are never followed here: each `Location` is returned to
//!    Core, which re-validates it as a new hop.
//!
//! Requests carry no cookies, credentials, proxy or ambient headers.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use medscale_contracts::browse::{
    BrowseDenyReason, URL_MAX_CHARS, is_dns_host_name, is_unambiguous_url_path,
};

/// A URL that passed `validate_url`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedUrl {
    /// Canonical form: `https://{host}{path}`.
    pub url: String,
    pub host: String,
    /// Path plus query, beginning with `/`.
    pub path: String,
}

/// Parses and checks a Browse URL. Fragment is dropped; userinfo, other
/// schemes and ports, IP-literal hosts, and paths a server could normalize
/// out from under an allowlist prefix (dot segments, backslashes, encoded
/// slashes) are refused.
pub fn validate_url(raw: &str) -> Result<ValidatedUrl, BrowseDenyReason> {
    let raw = raw.trim();
    if raw.is_empty() || raw.chars().count() > URL_MAX_CHARS {
        return Err(BrowseDenyReason::MalformedUrl);
    }
    if raw.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(BrowseDenyReason::MalformedUrl);
    }
    let lower = raw.to_ascii_lowercase();
    let rest = if lower.starts_with("https://") {
        &raw["https://".len()..]
    } else if lower.contains("://") || lower.starts_with("//") {
        return Err(BrowseDenyReason::SchemeNotHttps);
    } else {
        return Err(BrowseDenyReason::MalformedUrl);
    };
    let rest = rest.split('#').next().unwrap_or("");
    let (authority, path) = match rest.find(['/', '?']) {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    if authority.contains('@') {
        return Err(BrowseDenyReason::MalformedUrl);
    }
    if authority.starts_with('[') {
        return Err(BrowseDenyReason::IpLiteralHost);
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (h, Some(p)),
        None => (authority, None),
    };
    if let Some(port) = port
        && port != "443"
    {
        return Err(BrowseDenyReason::PortNotAllowed);
    }
    let host = host.to_ascii_lowercase();
    let host = host.strip_suffix('.').unwrap_or(&host).to_owned();
    if host.parse::<IpAddr>().is_ok() || looks_numeric_host(&host) {
        return Err(BrowseDenyReason::IpLiteralHost);
    }
    if !is_dns_host_name(&host) {
        return Err(BrowseDenyReason::MalformedUrl);
    }
    let path = if path.starts_with('?') {
        format!("/{path}")
    } else {
        path.to_owned()
    };
    if !is_unambiguous_url_path(&path) {
        return Err(BrowseDenyReason::MalformedUrl);
    }
    Ok(ValidatedUrl {
        url: format!("https://{host}{path}"),
        host,
        path,
    })
}

/// Numeric host forms some resolvers accept as IPv4 (`2130706433`,
/// `0x7f.1`, `127.1`).
fn looks_numeric_host(host: &str) -> bool {
    host.split('.').all(|label| {
        !label.is_empty()
            && (label.bytes().all(|b| b.is_ascii_digit())
                || (label.starts_with("0x") && label[2..].bytes().all(|b| b.is_ascii_hexdigit())))
    })
}

/// Resolves a `Location` header against the current URL. Only absolute
/// `https://` targets and absolute paths are supported; anything else is
/// returned unchanged for `validate_url` to refuse.
#[must_use]
pub fn resolve_redirect(current: &ValidatedUrl, location: &str) -> String {
    let location = location.trim();
    if location.starts_with('/') && !location.starts_with("//") {
        format!("https://{}{}", current.host, location)
    } else {
        location.to_owned()
    }
}

/// True for every address Browse must never connect to (decision register
/// Q45): loopback, private, link-local (incl. cloud metadata), shared/CGNAT,
/// multicast, broadcast, documentation, benchmarking, reserved, unspecified,
/// IPv6 unique-local/link-local/site-local, and IPv4-mapped/compatible or
/// NAT64 forms of any of these.
#[must_use]
pub fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => forbidden_v4(v4),
        IpAddr::V6(v6) => forbidden_v6(v6),
    }
}

fn forbidden_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.is_broadcast()
        || ip.is_documentation()
        || a == 0
        || (a == 100 && (64..=127).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 198 && (b == 18 || b == 19))
        || a >= 240
}

fn forbidden_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return forbidden_v4(v4);
    }
    let s = ip.segments();
    // 6to4 (2002::/16) embeds an IPv4 address; Teredo (2001::/32) and the
    // local-use NAT64 prefix (64:ff9b:1::/48) are refused outright.
    if s[0] == 0x2002 {
        let v4 = Ipv4Addr::new(
            (s[1] >> 8) as u8,
            (s[1] & 0xff) as u8,
            (s[2] >> 8) as u8,
            (s[2] & 0xff) as u8,
        );
        return forbidden_v4(v4);
    }
    if (s[0] == 0x2001 && s[1] == 0) || (s[0] == 0x64 && s[1] == 0xff9b && s[2] == 1) {
        return true;
    }
    // IPv4-compatible (::a.b.c.d, deprecated) and NAT64 (64:ff9b::/96).
    if s[..6] == [0, 0, 0, 0, 0, 0] || (s[0] == 0x64 && s[1] == 0xff9b && s[2..6] == [0, 0, 0, 0]) {
        let v4 = Ipv4Addr::new(
            (s[6] >> 8) as u8,
            (s[6] & 0xff) as u8,
            (s[7] >> 8) as u8,
            (s[7] & 0xff) as u8,
        );
        return ip.is_unspecified() || ip.is_loopback() || forbidden_v4(v4);
    }
    ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || (s[0] & 0xfe00) == 0xfc00 // unique local fc00::/7
        || (s[0] & 0xffc0) == 0xfe80 // link local fe80::/10
        || (s[0] & 0xffc0) == 0xfec0 // site local (deprecated)
        || (s[0] == 0x2001 && s[1] == 0x0db8) // documentation
}

/// One HTTP response as seen by Core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowseHttpResponse {
    pub status: u16,
    pub location: Option<String>,
    pub content_type: String,
    pub www_authenticate: bool,
    /// At most `limit + 1` bytes; more than `limit` means too large.
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowseTransportError {
    /// The target resolved only to forbidden addresses.
    ForbiddenAddress,
    Timeout,
    Failed,
}

/// Result of a streamed download ([`BrowseTransport::get_to_file`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowseFileResponse {
    pub status: u16,
    pub location: Option<String>,
    /// Length of the file after the call (resumed bytes included).
    pub file_len: u64,
}

/// The Browse egress seam. Core owns policy; the transport only performs a
/// single GET of an already validated URL, without following redirects.
pub trait BrowseTransport: Send + Sync {
    fn get(
        &self,
        url: &ValidatedUrl,
        max_body_bytes: usize,
        timeout: Duration,
    ) -> Result<BrowseHttpResponse, BrowseTransportError>;

    /// Streams one GET body into `path` without holding it in memory. With
    /// `resume_from > 0` a `Range: bytes=<resume_from>-` request is sent: a
    /// `206` response is appended to the existing file, and a `200` response
    /// (range ignored) replaces it. The file never grows beyond
    /// `max_total_bytes + 1` (one byte over means too large). Redirects are
    /// returned, never followed. When the connection fails mid-body, the bytes
    /// already received stay in the file so a later call can resume.
    /// Transports without streaming support refuse.
    fn get_to_file(
        &self,
        url: &ValidatedUrl,
        resume_from: u64,
        max_total_bytes: u64,
        path: &std::path::Path,
        timeout: Duration,
    ) -> Result<BrowseFileResponse, BrowseTransportError> {
        let _ = (url, resume_from, max_total_bytes, path, timeout);
        Err(BrowseTransportError::Failed)
    }
}

/// Writes `reader` into `path` after a `200` (replace) or `206` (append),
/// bounded to `max_total_bytes + 1` bytes in total.
fn stream_into_file(
    reader: &mut dyn std::io::Read,
    status: u16,
    resume_from: u64,
    max_total_bytes: u64,
    path: &std::path::Path,
) -> Result<u64, BrowseTransportError> {
    use std::io::{Read as _, Write as _};
    let append = status == 206 && resume_from > 0;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)
        .map_err(|_| BrowseTransportError::Failed)?;
    let start = if append { resume_from } else { 0 };
    let budget = max_total_bytes.saturating_add(1).saturating_sub(start);
    let mut limited = reader.take(budget);
    let mut buffer = vec![0u8; 64 * 1024];
    let mut written = start;
    loop {
        match limited.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                file.write_all(&buffer[..n])
                    .map_err(|_| BrowseTransportError::Failed)?;
                written += n as u64;
            }
            Err(e) => {
                // Keep what arrived; the caller may resume from here.
                let _ = file.flush();
                return Err(if e.kind() == std::io::ErrorKind::TimedOut {
                    BrowseTransportError::Timeout
                } else {
                    BrowseTransportError::Failed
                });
            }
        }
    }
    file.flush().map_err(|_| BrowseTransportError::Failed)?;
    Ok(written)
}

/// Live transport over `ureq` with the public-only resolver.
#[derive(Debug, Default)]
pub struct UreqBrowseTransport;

/// Wraps the default resolver and drops forbidden addresses. When the name
/// resolved but every answer was forbidden, `refused` is set so the caller
/// can tell a private-network answer from a name that does not resolve.
#[derive(Debug, Default)]
pub struct PublicOnlyResolver {
    inner: ureq::unversioned::resolver::DefaultResolver,
    refused: Arc<AtomicBool>,
}

impl ureq::unversioned::resolver::Resolver for PublicOnlyResolver {
    fn resolve(
        &self,
        uri: &ureq::http::Uri,
        config: &ureq::config::Config,
        timeout: ureq::unversioned::transport::NextTimeout,
    ) -> Result<ureq::unversioned::resolver::ResolvedSocketAddrs, ureq::Error> {
        let all = self.inner.resolve(uri, config, timeout)?;
        let mut kept = self.empty();
        for addr in all.iter().filter(|a| !is_forbidden_ip(a.ip())) {
            kept.push(*addr);
        }
        if kept.is_empty() {
            if !all.is_empty() {
                self.refused.store(true, Ordering::SeqCst);
            }
            return Err(ureq::Error::HostNotFound);
        }
        Ok(kept)
    }
}

impl UreqBrowseTransport {
    fn agent(timeout: Duration, refused: Arc<AtomicBool>) -> ureq::Agent {
        let config = ureq::Agent::config_builder()
            .max_redirects(0)
            .http_status_as_error(false)
            .https_only(true)
            .proxy(None)
            .timeout_global(Some(timeout))
            .user_agent("MedScale-GovernedBrowse/1")
            .build();
        ureq::Agent::with_parts(
            config,
            ureq::unversioned::transport::DefaultConnector::default(),
            PublicOnlyResolver {
                inner: ureq::unversioned::resolver::DefaultResolver::default(),
                refused,
            },
        )
    }
}

impl BrowseTransport for UreqBrowseTransport {
    fn get(
        &self,
        url: &ValidatedUrl,
        max_body_bytes: usize,
        timeout: Duration,
    ) -> Result<BrowseHttpResponse, BrowseTransportError> {
        let refused = Arc::new(AtomicBool::new(false));
        let agent = Self::agent(timeout, Arc::clone(&refused));
        let mut response = match agent.get(&url.url).call() {
            Ok(r) => r,
            // A name that does not resolve is a transport failure; only an
            // answer made entirely of forbidden addresses is a private target.
            Err(ureq::Error::HostNotFound) if refused.load(Ordering::SeqCst) => {
                return Err(BrowseTransportError::ForbiddenAddress);
            }
            Err(ureq::Error::Timeout(_)) => return Err(BrowseTransportError::Timeout),
            Err(_) => return Err(BrowseTransportError::Failed),
        };
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        let status = response.status().as_u16();
        let location = header("location");
        let content_type = header("content-type").unwrap_or_default();
        let www_authenticate = header("www-authenticate").is_some();
        let limit = u64::try_from(max_body_bytes)
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        let body = match response.body_mut().with_config().limit(limit).read_to_vec() {
            Ok(bytes) => bytes,
            Err(ureq::Error::BodyExceedsLimit(_)) => vec![0; max_body_bytes + 1],
            Err(ureq::Error::Timeout(_)) => return Err(BrowseTransportError::Timeout),
            Err(_) => return Err(BrowseTransportError::Failed),
        };
        Ok(BrowseHttpResponse {
            status,
            location,
            content_type,
            www_authenticate,
            body,
        })
    }

    fn get_to_file(
        &self,
        url: &ValidatedUrl,
        resume_from: u64,
        max_total_bytes: u64,
        path: &std::path::Path,
        timeout: Duration,
    ) -> Result<BrowseFileResponse, BrowseTransportError> {
        let refused = Arc::new(AtomicBool::new(false));
        let agent = Self::agent(timeout, Arc::clone(&refused));
        let mut request = agent.get(&url.url);
        if resume_from > 0 {
            request = request.header("Range", &format!("bytes={resume_from}-"));
        }
        let mut response = match request.call() {
            Ok(r) => r,
            Err(ureq::Error::HostNotFound) if refused.load(Ordering::SeqCst) => {
                return Err(BrowseTransportError::ForbiddenAddress);
            }
            Err(ureq::Error::Timeout(_)) => return Err(BrowseTransportError::Timeout),
            Err(_) => return Err(BrowseTransportError::Failed),
        };
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        if status != 200 && status != 206 {
            return Ok(BrowseFileResponse {
                status,
                location,
                file_len: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
            });
        }
        let limit = max_total_bytes.saturating_add(1);
        let mut reader = response.body_mut().with_config().limit(limit).reader();
        let file_len = stream_into_file(&mut reader, status, resume_from, max_total_bytes, path)?;
        Ok(BrowseFileResponse {
            status,
            location,
            file_len,
        })
    }
}

/// Offline scripted transport for hermetic tests and demos: no sockets.
/// Unknown URLs fail as transport errors; a scripted `ForbiddenAddress`
/// models a host whose DNS answer is private.
#[derive(Default)]
pub struct ScriptedBrowseTransport {
    routes: Vec<(String, Result<BrowseHttpResponse, BrowseTransportError>)>,
    /// Streamed downloads of these URLs stop with `Timeout` once the file
    /// reaches the given length (an interrupted connection).
    interruptions: Vec<(String, u64)>,
    /// When true, `Range` requests are answered with the full `200` body.
    ignore_range: bool,
}

impl fmt::Debug for ScriptedBrowseTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScriptedBrowseTransport")
            .field("routes", &self.routes.len())
            .finish()
    }
}

impl ScriptedBrowseTransport {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Scripts the answer for one canonical URL.
    #[must_use]
    pub fn route(
        mut self,
        url: &str,
        answer: Result<BrowseHttpResponse, BrowseTransportError>,
    ) -> Self {
        self.routes.push((url.to_owned(), answer));
        self
    }

    /// A 200 response with the given content type and body.
    pub fn ok(content_type: &str, body: &[u8]) -> Result<BrowseHttpResponse, BrowseTransportError> {
        Ok(BrowseHttpResponse {
            status: 200,
            location: None,
            content_type: content_type.to_owned(),
            www_authenticate: false,
            body: body.to_vec(),
        })
    }

    /// Streamed downloads of `url` stop with `Timeout` once the file reaches
    /// `offset` bytes.
    #[must_use]
    pub fn interrupt_after(mut self, url: &str, offset: u64) -> Self {
        self.interruptions.push((url.to_owned(), offset));
        self
    }

    /// Answer `Range` requests with the full body (`200`), as some servers do.
    #[must_use]
    pub fn ignoring_range(mut self) -> Self {
        self.ignore_range = true;
        self
    }

    /// A 302 redirect.
    pub fn redirect(location: &str) -> Result<BrowseHttpResponse, BrowseTransportError> {
        Ok(BrowseHttpResponse {
            status: 302,
            location: Some(location.to_owned()),
            content_type: String::new(),
            www_authenticate: false,
            body: Vec::new(),
        })
    }
}

impl BrowseTransport for ScriptedBrowseTransport {
    fn get(
        &self,
        url: &ValidatedUrl,
        max_body_bytes: usize,
        _timeout: Duration,
    ) -> Result<BrowseHttpResponse, BrowseTransportError> {
        match self.routes.iter().find(|(u, _)| u == &url.url) {
            Some((_, Ok(response))) => {
                let mut r = response.clone();
                r.body.truncate(max_body_bytes + 1);
                Ok(r)
            }
            Some((_, Err(e))) => Err(e.clone()),
            None => Err(BrowseTransportError::Failed),
        }
    }

    fn get_to_file(
        &self,
        url: &ValidatedUrl,
        resume_from: u64,
        max_total_bytes: u64,
        path: &std::path::Path,
        _timeout: Duration,
    ) -> Result<BrowseFileResponse, BrowseTransportError> {
        let response = match self.routes.iter().find(|(u, _)| u == &url.url) {
            Some((_, Ok(response))) => response.clone(),
            Some((_, Err(e))) => return Err(e.clone()),
            None => return Err(BrowseTransportError::Failed),
        };
        if response.status != 200 {
            return Ok(BrowseFileResponse {
                status: response.status,
                location: response.location,
                file_len: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
            });
        }
        let ranged = resume_from > 0 && !self.ignore_range;
        let (status, start) = if ranged { (206, resume_from) } else { (200, 0) };
        let from = usize::try_from(start)
            .unwrap_or(usize::MAX)
            .min(response.body.len());
        let body = &response.body[from..];
        let cut = self
            .interruptions
            .iter()
            .find(|(u, _)| u == &url.url)
            .map(|(_, offset)| *offset)
            .filter(|offset| *offset > start);
        let take = cut.map_or(body.len(), |c| {
            usize::try_from(c - start)
                .unwrap_or(usize::MAX)
                .min(body.len())
        });
        let mut reader: &[u8] = &body[..take];
        let file_len = stream_into_file(&mut reader, status, resume_from, max_total_bytes, path)?;
        if take < body.len() {
            return Err(BrowseTransportError::Timeout);
        }
        Ok(BrowseFileResponse {
            status,
            location: None,
            file_len,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_url_accepts_only_https_dns_hosts_on_443() {
        let ok = validate_url("https://Example.ORG/a/b?x=1#frag").unwrap();
        assert_eq!(ok.url, "https://example.org/a/b?x=1");
        assert_eq!(ok.host, "example.org");
        assert_eq!(ok.path, "/a/b?x=1");
        assert_eq!(validate_url("https://example.org").unwrap().path, "/");
        assert_eq!(
            validate_url("https://example.org?q=1").unwrap().path,
            "/?q=1"
        );
        assert!(validate_url("https://example.org:443/").is_ok());
        let cases = [
            ("http://example.org/", BrowseDenyReason::SchemeNotHttps),
            ("ftp://example.org/", BrowseDenyReason::SchemeNotHttps),
            ("file:///etc/passwd", BrowseDenyReason::SchemeNotHttps),
            ("//example.org/", BrowseDenyReason::SchemeNotHttps),
            (
                "https://example.org:8443/",
                BrowseDenyReason::PortNotAllowed,
            ),
            ("https://127.0.0.1/", BrowseDenyReason::IpLiteralHost),
            (
                "https://169.254.169.254/latest/meta-data",
                BrowseDenyReason::IpLiteralHost,
            ),
            ("https://[::1]/", BrowseDenyReason::IpLiteralHost),
            ("https://2130706433/", BrowseDenyReason::IpLiteralHost),
            ("https://0x7f.1/", BrowseDenyReason::IpLiteralHost),
            ("https://127.1/", BrowseDenyReason::IpLiteralHost),
            (
                "https://user:pw@example.org/",
                BrowseDenyReason::MalformedUrl,
            ),
            ("https://localhost/", BrowseDenyReason::MalformedUrl),
            ("https://exa mple.org/", BrowseDenyReason::MalformedUrl),
            ("example.org", BrowseDenyReason::MalformedUrl),
            (
                "https://example.org/docs/../admin",
                BrowseDenyReason::MalformedUrl,
            ),
            (
                "https://example.org/docs/%2e%2e/admin",
                BrowseDenyReason::MalformedUrl,
            ),
            (
                "https://example.org/docs/..;/admin",
                BrowseDenyReason::MalformedUrl,
            ),
            (
                "https://example.org/docs%2fadmin",
                BrowseDenyReason::MalformedUrl,
            ),
            (
                "https://example.org\\@evil.org/",
                BrowseDenyReason::MalformedUrl,
            ),
            (
                "https://evil.org\\.example.org/",
                BrowseDenyReason::MalformedUrl,
            ),
            ("https://%31%32%37.0.0.1/", BrowseDenyReason::MalformedUrl),
            ("https://ex\u{e4}mple.org/", BrowseDenyReason::MalformedUrl),
            (
                "https://example.org:443@evil.org/",
                BrowseDenyReason::MalformedUrl,
            ),
            ("https://example.org:/", BrowseDenyReason::PortNotAllowed),
            (
                "https://example.org:0443/",
                BrowseDenyReason::PortNotAllowed,
            ),
            ("https://0177.0.0.1/", BrowseDenyReason::IpLiteralHost),
            ("https:/example.org/", BrowseDenyReason::MalformedUrl),
            ("HTTP://example.org/", BrowseDenyReason::SchemeNotHttps),
            ("https://localhost./", BrowseDenyReason::MalformedUrl),
            ("", BrowseDenyReason::MalformedUrl),
        ];
        for (url, reason) in cases {
            assert_eq!(validate_url(url), Err(reason), "{url}");
        }
    }

    #[test]
    fn forbidden_addresses_cover_private_and_special_ranges() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "224.0.0.1",
            "255.255.255.255",
            "192.0.2.1",
            "198.18.0.1",
            "240.0.0.1",
            "192.0.0.8",
            "::1",
            "::",
            "fc00::1",
            "fd12::1",
            "fe80::1",
            "ff02::1",
            "2001:db8::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            "::127.0.0.1",
            "64:ff9b::a9fe:a9fe",
            "64:ff9b:1::1",
            "2002:7f00:1::1",
            "2002:a9fe:a9fe::1",
            "2001:0:4136:e378::1",
            "fec0::1",
            "::ffff:169.254.169.254",
            "::ffff:100.64.0.1",
        ] {
            assert!(is_forbidden_ip(ip.parse().unwrap()), "{ip}");
        }
        for ip in [
            "93.184.215.14",
            "1.1.1.1",
            "2606:4700:4700::1111",
            "::ffff:8.8.8.8",
            "2002:808:808::1",
        ] {
            assert!(!is_forbidden_ip(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn redirects_resolve_only_absolute_https_and_paths() {
        let cur = validate_url("https://example.org/a").unwrap();
        assert_eq!(resolve_redirect(&cur, "/b"), "https://example.org/b");
        assert_eq!(
            resolve_redirect(&cur, "https://other.org/c"),
            "https://other.org/c"
        );
        // Scheme-relative and relative forms go back unchanged and fail validation.
        assert!(validate_url(&resolve_redirect(&cur, "//evil.org/x")).is_err());
        assert!(validate_url(&resolve_redirect(&cur, "http://example.org/")).is_err());
    }

    #[test]
    fn scripted_transport_answers_only_scripted_urls() {
        let t = ScriptedBrowseTransport::new().route(
            "https://example.org/",
            ScriptedBrowseTransport::ok("text/plain", b"hello"),
        );
        let u = validate_url("https://example.org/").unwrap();
        let r = t.get(&u, 3, Duration::from_secs(1)).unwrap();
        assert_eq!(r.body.len(), 4, "truncated to limit + 1");
        let other = validate_url("https://other.org/").unwrap();
        assert_eq!(
            t.get(&other, 10, Duration::from_secs(1)),
            Err(BrowseTransportError::Failed)
        );
    }
}
