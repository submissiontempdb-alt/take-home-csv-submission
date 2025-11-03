//! Client account data structure

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// ClientError represents errors related to client account operations
#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Cannot perform operation on locked account")]
    AccountLocked,

    #[error("Insufficient available funds: available {available}, requested {requested}")]
    InsufficientFunds {
        available: Decimal,
        requested: Decimal,
    },

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),
}

/// Represents a client's account state
///
/// Fields are private to prevent direct manipulation and ensure all state changes
/// go through validated business logic methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientAccount {
    /// Funds available for trading, withdrawal, etc. (total - held)
    available: Decimal,
    /// Funds held due to disputes
    held: Decimal,
    /// Total funds (available + held)
    total: Decimal,
    /// Whether the account is locked (due to chargeback)
    locked: bool,
}

impl ClientAccount {
    /// Create a new client account with zero balance
    pub fn new() -> Self {
        ClientAccount {
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            total: Decimal::ZERO,
            locked: false,
        }
    }

    // Getter methods to safely access private fields

    /// Get the available balance
    pub fn available(&self) -> Decimal {
        self.available
    }

    /// Get the held balance
    pub fn held(&self) -> Decimal {
        self.held
    }

    /// Get the total balance
    pub fn total(&self) -> Decimal {
        self.total
    }

    /// Check if the account is locked
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    // Business logic methods - these are the ONLY ways to modify account state

    /// Deposit funds into the account
    pub fn deposit(&mut self, amount: Decimal) -> Result<(), ClientError> {
        if self.locked {
            return Err(ClientError::AccountLocked);
        }
        if amount < Decimal::ZERO {
            return Err(ClientError::InvalidAmount(
                "Cannot deposit negative amount".to_string(),
            ));
        }

        self.available += amount;
        self.total += amount;
        Ok(())
    }

    /// Attempt to withdraw funds from the account
    pub fn withdraw(&mut self, amount: Decimal) -> Result<bool, ClientError> {
        if self.locked {
            return Err(ClientError::AccountLocked);
        }
        if amount < Decimal::ZERO {
            return Err(ClientError::InvalidAmount(
                "Cannot withdraw negative amount".to_string(),
            ));
        }

        if self.available >= amount {
            self.available -= amount;
            self.total -= amount;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Hold funds due to a dispute
    pub fn hold_funds(&mut self, amount: Decimal) -> Result<(), ClientError> {
        if amount < Decimal::ZERO {
            return Err(ClientError::InvalidAmount(
                "Cannot hold negative amount".to_string(),
            ));
        }
        if self.available < amount {
            return Err(ClientError::InsufficientFunds {
                available: self.available,
                requested: amount,
            });
        }

        self.available -= amount;
        self.held += amount;
        Ok(())
    }

    /// Release held funds due to dispute resolution
    pub fn release_held(&mut self, amount: Decimal) -> Result<(), ClientError> {
        if amount < Decimal::ZERO {
            return Err(ClientError::InvalidAmount(
                "Cannot release negative amount".to_string(),
            ));
        }
        if self.held < amount {
            return Err(ClientError::InsufficientFunds {
                available: self.held,
                requested: amount,
            });
        }

        self.held -= amount;
        self.available += amount;
        Ok(())
    }

    /// Remove held funds and decrease total (chargeback)
    pub fn chargeback(&mut self, amount: Decimal) -> Result<(), ClientError> {
        if amount < Decimal::ZERO {
            return Err(ClientError::InvalidAmount(
                "Cannot chargeback negative amount".to_string(),
            ));
        }
        if self.held < amount {
            return Err(ClientError::InsufficientFunds {
                available: self.held,
                requested: amount,
            });
        }

        self.held -= amount;
        self.total -= amount;
        self.locked = true;
        Ok(())
    }

    /// Lock the account
    pub fn lock(&mut self) {
        self.locked = true;
    }

    /// Check invariants (for testing/debugging)
    pub fn validate(&self) -> bool {
        // total should equal available + held
        (self.available + self.held - self.total).abs() < Decimal::new(1, 5)
    }
}

impl Default for ClientAccount {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_account() {
        let account = ClientAccount::new();
        assert_eq!(account.available(), Decimal::ZERO);
        assert_eq!(account.held(), Decimal::ZERO);
        assert_eq!(account.total(), Decimal::ZERO);
        assert!(!account.is_locked());
    }

    #[test]
    fn test_deposit() {
        let mut account = ClientAccount::new();
        assert!(account.deposit(Decimal::from(100)).is_ok());
        assert_eq!(account.available(), Decimal::from(100));
        assert_eq!(account.total(), Decimal::from(100));
        assert!(account.validate());
    }

    #[test]
    fn test_withdraw_success() {
        let mut account = ClientAccount::new();
        account.deposit(Decimal::from(100)).ok();
        let result = account.withdraw(Decimal::from(50));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        assert_eq!(account.available(), Decimal::from(50));
        assert_eq!(account.total(), Decimal::from(50));
        assert!(account.validate());
    }

    #[test]
    fn test_withdraw_insufficient_funds() {
        let mut account = ClientAccount::new();
        account.deposit(Decimal::from(100)).ok();
        let result = account.withdraw(Decimal::from(150));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
        assert_eq!(account.available(), Decimal::from(100));
        assert_eq!(account.total(), Decimal::from(100));
    }

    #[test]
    fn test_hold_and_release() {
        let mut account = ClientAccount::new();
        account.deposit(Decimal::from(100)).ok();
        assert!(account.hold_funds(Decimal::from(30)).is_ok());
        assert_eq!(account.available(), Decimal::from(70));
        assert_eq!(account.held(), Decimal::from(30));
        assert_eq!(account.total(), Decimal::from(100));
        assert!(account.validate());

        assert!(account.release_held(Decimal::from(30)).is_ok());
        assert_eq!(account.available(), Decimal::from(100));
        assert_eq!(account.held(), Decimal::ZERO);
        assert_eq!(account.total(), Decimal::from(100));
        assert!(account.validate());
    }

    #[test]
    fn test_chargeback() {
        let mut account = ClientAccount::new();
        account.deposit(Decimal::from(100)).ok();
        account.hold_funds(Decimal::from(30)).ok();
        assert!(account.chargeback(Decimal::from(30)).is_ok());
        assert_eq!(account.available(), Decimal::from(70));
        assert_eq!(account.held(), Decimal::ZERO);
        assert_eq!(account.total(), Decimal::from(70));
        assert!(account.is_locked());
        assert!(account.validate());
    }

    #[test]
    fn test_locked_account_prevents_deposit() {
        let mut account = ClientAccount::new();
        account.deposit(Decimal::from(100)).ok();
        account.hold_funds(Decimal::from(100)).ok();
        account.chargeback(Decimal::from(100)).ok();

        // Account is now locked, deposits should fail
        let result = account.deposit(Decimal::from(50));
        assert!(result.is_err());
        assert_eq!(account.total(), Decimal::ZERO);
    }

    #[test]
    fn test_locked_account_prevents_withdrawal() {
        let mut account = ClientAccount::new();
        account.deposit(Decimal::from(100)).ok();
        account.hold_funds(Decimal::from(50)).ok();
        account.chargeback(Decimal::from(50)).ok();

        // Account is now locked, withdrawals should fail
        let result = account.withdraw(Decimal::from(10));
        assert!(result.is_err());
        assert_eq!(account.available(), Decimal::from(50));
    }

    #[test]
    fn test_negative_amount_rejection() {
        let mut account = ClientAccount::new();

        assert!(account.deposit(Decimal::from(-100)).is_err());
        assert!(account.withdraw(Decimal::from(-50)).is_err());
        assert!(account.hold_funds(Decimal::from(-30)).is_err());
        assert!(account.release_held(Decimal::from(-20)).is_err());
        assert!(account.chargeback(Decimal::from(-10)).is_err());
    }

    #[test]
    fn test_insufficient_held_funds() {
        let mut account = ClientAccount::new();
        account.deposit(Decimal::from(100)).ok();
        account.hold_funds(Decimal::from(30)).ok();

        // Try to release more than held
        assert!(account.release_held(Decimal::from(50)).is_err());

        // Try to chargeback more than held
        assert!(account.chargeback(Decimal::from(50)).is_err());
    }
}
