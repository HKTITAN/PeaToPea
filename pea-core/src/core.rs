//! Host-driven API: PeaPodCore receives events from host, returns actions.
//! Full implementation in later work.

use crate::identity::DeviceId;

/// Stub for the main coordinator. Host passes in events (request, peer joined/left, message);
/// core returns actions (chunk assignments, messages to send, WAN requests).
pub struct PeaPodCore;

impl PeaPodCore {
    pub fn new() -> Self {
        Self
    }

    /// Stub: on incoming request metadata (URL, range). Returns whether to accelerate or fallback.
    pub fn on_incoming_request(&mut self, _url: &str, _range: Option<(u64, u64)>) -> Action {
        Action::Fallback
    }

    /// Stub: peer joined. Full impl would update peer list and session.
    pub fn on_peer_joined(&mut self, _peer_id: DeviceId, _public_key: &crate::identity::PublicKey) {
    }

    /// Stub: peer left. Full impl would redistribute chunks.
    pub fn on_peer_left(&mut self, _peer_id: DeviceId) {}

    /// Stub: tick for heartbeats and timeouts.
    pub fn tick(&mut self) -> Vec<OutboundAction> {
        vec![]
    }
}

impl Default for PeaPodCore {
    fn default() -> Self {
        Self::new()
    }
}

/// Action after host passes request metadata.
pub enum Action {
    /// Accelerate: host should request chunks per core's plan.
    Accelerate,
    /// Fall back to normal path (ineligible or no peers).
    Fallback,
}

/// Action for host to perform (send message, issue WAN request, etc.).
pub enum OutboundAction {
    /// Send a protocol message to a peer.
    SendMessage(DeviceId, Vec<u8>),
}
