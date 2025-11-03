//! Transaction data structure and types

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumCount, EnumIter, EnumString};

/// Represents the type of transaction
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,    // Automatic Display implementation
    EnumString, // Automatic FromStr implementation
    EnumIter,   // Iterator over all variants
    EnumCount,  // Count of variants
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)] // Case-insensitive parsing, lowercase serialization
pub enum TransactionType {
    /// Credit to client account
    Deposit,
    /// Debit from client account
    Withdrawal,
    /// Client disputes a transaction
    Dispute,
    /// Resolution of a disputed transaction
    Resolve,
    /// Final reversal of a disputed transaction
    Chargeback,
}

/// Represents a parsed transaction from CSV input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction type (deposit, withdrawal, etc.)
    pub tx_type: TransactionType,
    /// Client ID (u16)
    pub client: u16,
    /// Transaction ID (u32) - globally unique
    pub tx: u32,
    /// Amount (only for deposit/withdrawal; None for dispute/resolve/chargeback)
    pub amount: Option<Decimal>,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(tx_type: TransactionType, client: u16, tx: u32, amount: Option<Decimal>) -> Self {
        Transaction {
            tx_type,
            client,
            tx,
            amount,
        }
    }

    /// Check if this transaction is a money-moving transaction (deposit/withdrawal)
    pub fn is_money_moving(&self) -> bool {
        matches!(
            self.tx_type,
            TransactionType::Deposit | TransactionType::Withdrawal
        )
    }

    /// Check if this transaction is a dispute operation
    pub fn is_dispute_operation(&self) -> bool {
        matches!(
            self.tx_type,
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use strum::{EnumCount, IntoEnumIterator};

    #[test]
    fn test_transaction_type_parsing() {
        // Test FromStr (provided by EnumString derive)
        assert_eq!(
            TransactionType::from_str("deposit").unwrap(),
            TransactionType::Deposit
        );
        // Case-insensitive parsing
        assert_eq!(
            TransactionType::from_str("DEPOSIT").unwrap(),
            TransactionType::Deposit
        );
        // Whitespace needs to be trimmed by caller
        assert_eq!(
            TransactionType::from_str("deposit".trim()).unwrap(),
            TransactionType::Deposit
        );
        assert_eq!(
            TransactionType::from_str("withdrawal").unwrap(),
            TransactionType::Withdrawal
        );
        assert_eq!(
            TransactionType::from_str("dispute").unwrap(),
            TransactionType::Dispute
        );
        assert_eq!(
            TransactionType::from_str("resolve").unwrap(),
            TransactionType::Resolve
        );
        assert_eq!(
            TransactionType::from_str("chargeback").unwrap(),
            TransactionType::Chargeback
        );
        // Invalid input
        assert!(TransactionType::from_str("invalid").is_err());

        // Test that whitespace trimming works when caller trims
        assert_eq!(
            TransactionType::from_str("  deposit  ".trim()).unwrap(),
            TransactionType::Deposit
        );
    }

    #[test]
    fn test_transaction_type_display() {
        // Test Display (provided by Display derive)
        assert_eq!(TransactionType::Deposit.to_string(), "deposit");
        assert_eq!(TransactionType::Withdrawal.to_string(), "withdrawal");
        assert_eq!(TransactionType::Dispute.to_string(), "dispute");
        assert_eq!(TransactionType::Resolve.to_string(), "resolve");
        assert_eq!(TransactionType::Chargeback.to_string(), "chargeback");
    }

    #[test]
    fn test_transaction_type_iterator() {
        // Test EnumIter (provided by EnumIter derive)
        let all_types: Vec<TransactionType> = TransactionType::iter().collect();
        assert_eq!(all_types.len(), 5);
        assert_eq!(all_types[0], TransactionType::Deposit);
        assert_eq!(all_types[1], TransactionType::Withdrawal);
        assert_eq!(all_types[2], TransactionType::Dispute);
        assert_eq!(all_types[3], TransactionType::Resolve);
        assert_eq!(all_types[4], TransactionType::Chargeback);
    }

    #[test]
    fn test_transaction_type_count() {
        // Test EnumCount (provided by EnumCount derive)
        assert_eq!(TransactionType::COUNT, 5);
    }

    #[test]
    fn test_transaction_classification() {
        let deposit = Transaction::new(TransactionType::Deposit, 1, 1, Some(Decimal::from(100)));
        assert!(deposit.is_money_moving());
        assert!(!deposit.is_dispute_operation());

        let dispute = Transaction::new(TransactionType::Dispute, 1, 1, None);
        assert!(!dispute.is_money_moving());
        assert!(dispute.is_dispute_operation());
    }
}
