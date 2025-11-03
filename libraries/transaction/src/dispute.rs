//! Dispute state tracking

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Represents the state of a disputed transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeState {
    /// Amount that is disputed
    pub amount: Decimal,
    /// Whether this dispute has been resolved (false = still disputed, true = resolved)
    pub is_resolved: bool,
}

impl DisputeState {
    /// Create a new dispute state
    pub fn new(amount: Decimal) -> Self {
        DisputeState {
            amount,
            is_resolved: false,
        }
    }

    /// Mark the dispute as resolved
    pub fn resolve(&mut self) {
        self.is_resolved = true;
    }

    /// Check if the dispute is currently active (not resolved, not chargebacked)
    pub fn is_active(&self) -> bool {
        !self.is_resolved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_dispute() {
        let dispute = DisputeState::new(Decimal::from(100));
        assert_eq!(dispute.amount, Decimal::from(100));
        assert!(!dispute.is_resolved);
        assert!(dispute.is_active());
    }

    #[test]
    fn test_resolve_dispute() {
        let mut dispute = DisputeState::new(Decimal::from(100));
        dispute.resolve();
        assert!(dispute.is_resolved);
        assert!(!dispute.is_active());
    }
}
