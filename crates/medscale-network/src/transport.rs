//! Broker transport trait + fixture / ureq backends.

use medscale_contracts::network::EgressPurpose;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct TransportRequest {
    pub host: String,
    pub path: String,
    pub purpose: EgressPurpose,
    pub fixture_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("external gate required for live partner")]
    ExternalGateRequired,
    #[error("transport timeout: {0}")]
    Timeout(String),
    #[error("transport failed: {0}")]
    Failed(String),
}

pub trait BrokerTransport: Send + Sync {
    fn send(&self, req: &TransportRequest) -> Result<String, TransportError>;
}

/// Offline fixture transport — no sockets.
///
/// Synthetic failure fixture ids (Spec 037):
/// - `fail:timeout` → `TransportError::Timeout`
/// - `fail:transport` → `TransportError::Failed`
#[derive(Debug, Default)]
pub struct FixtureTransport;

impl BrokerTransport for FixtureTransport {
    fn send(&self, req: &TransportRequest) -> Result<String, TransportError> {
        let id = req.fixture_id.as_deref().unwrap_or("default");
        match id {
            "fail:timeout" => Err(TransportError::Timeout(
                "synthetic fixture timeout".to_owned(),
            )),
            "fail:transport" => Err(TransportError::Failed(
                "synthetic fixture transport failure".to_owned(),
            )),
            _ => Ok(serde_json::json!({
                "fixture": true,
                "host": req.host,
                "path": req.path,
                "purpose": format!("{:?}", req.purpose),
                "fixture_id": id,
            })
            .to_string()),
        }
    }
}

/// Real ureq transport — only used when broker Allow decision already passed.
#[derive(Debug, Default)]
pub struct UreqTransport;

impl BrokerTransport for UreqTransport {
    fn send(&self, req: &TransportRequest) -> Result<String, TransportError> {
        // Live partner hosts are gated; Spec 013 refuses non-fixture live calls at adapter layer.
        // This backend remains available for future gated use; deny by default for safety.
        let _ = req;
        Err(TransportError::ExternalGateRequired)
    }
}
