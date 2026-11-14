//! Effect state machine (UNKNOWN fail-closed without reconcile).

use medscale_contracts::objects::EffectState;

/// Returns whether a transition is legal.
#[must_use]
pub fn can_transition(from: EffectState, to: EffectState, reconcile_token: Option<&str>) -> bool {
    use EffectState::{Confirmed, Failed, Pending, Sent, Unknown};
    match (from, to) {
        (Pending, Sent) => true,
        (Sent, Confirmed | Failed | Unknown) => true,
        (Unknown, Pending | Sent) => reconcile_token.map(|t| !t.is_empty()).unwrap_or(false),
        (Failed, Pending) => true, // explicit restart after failure
        (Confirmed, _) => false,
        (a, b) if a == b => true,
        _ => false,
    }
}

/// Applies a transition or returns false when illegal / UNKNOWN without reconcile.
#[must_use]
pub fn transition(
    from: EffectState,
    to: EffectState,
    reconcile_token: Option<&str>,
) -> Option<EffectState> {
    if can_transition(from, to, reconcile_token) {
        Some(to)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_requires_reconcile() {
        assert!(!can_transition(
            EffectState::Unknown,
            EffectState::Pending,
            None
        ));
        assert!(can_transition(
            EffectState::Unknown,
            EffectState::Pending,
            Some("reconciled")
        ));
    }
}
