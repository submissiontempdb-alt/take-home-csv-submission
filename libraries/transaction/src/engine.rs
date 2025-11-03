//! Transaction engine for processing payments

use crate::{ClientAccount, DisputeState, Transaction, TransactionError, TransactionType};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Represents a money-moving transaction (deposit or withdrawal)
/// Needed for dispute lookups
#[derive(Debug, Clone)]
struct StoredTransaction {
    client: u16,
    amount: Decimal,
    tx_type: TransactionType,
}

/// The main transaction processing engine
#[derive(Debug)]
pub struct TransactionEngine {
    /// All client accounts (client_id -> account state)
    clients: HashMap<u16, ClientAccount>,
    /// All money-moving transactions for dispute lookups (tx_id -> transaction)
    transactions: HashMap<u32, StoredTransaction>,
    /// Dispute states (tx_id -> dispute state)
    disputes: HashMap<u32, DisputeState>,
}

impl TransactionEngine {
    /// Create a new transaction engine
    pub fn new() -> Self {
        TransactionEngine {
            clients: HashMap::new(),
            transactions: HashMap::new(),
            disputes: HashMap::new(),
        }
    }

    /// Process a single transaction
    pub fn process_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), TransactionError> {
        match transaction.tx_type {
            TransactionType::Deposit => self.handle_deposit(transaction),
            TransactionType::Withdrawal => self.handle_withdrawal(transaction),
            TransactionType::Dispute => self.handle_dispute(transaction),
            TransactionType::Resolve => self.handle_resolve(transaction),
            TransactionType::Chargeback => self.handle_chargeback(transaction),
        }
    }

    /// Handle a deposit transaction
    fn handle_deposit(&mut self, transaction: Transaction) -> Result<(), TransactionError> {
        let amount = transaction.amount.ok_or_else(|| {
            TransactionError::InvalidDecimalPrecision("Deposit must have an amount".to_string())
        })?;

        let client = self.clients.entry(transaction.client).or_default();

        client.deposit(amount)?;

        // Store the transaction for future dispute references
        self.transactions.insert(
            transaction.tx,
            StoredTransaction {
                client: transaction.client,
                amount,
                tx_type: TransactionType::Deposit,
            },
        );

        Ok(())
    }

    /// Handle a withdrawal transaction
    fn handle_withdrawal(&mut self, transaction: Transaction) -> Result<(), TransactionError> {
        let amount = transaction.amount.ok_or_else(|| {
            TransactionError::InvalidDecimalPrecision("Withdrawal must have an amount".to_string())
        })?;

        let client = self.clients.entry(transaction.client).or_default();

        // Withdrawal fails silently if insufficient funds or account is locked
        let _ = client.withdraw(amount);

        // Store the transaction for future dispute references (even if it failed)
        self.transactions.insert(
            transaction.tx,
            StoredTransaction {
                client: transaction.client,
                amount,
                tx_type: TransactionType::Withdrawal,
            },
        );

        Ok(())
    }

    /// Handle a dispute transaction
    fn handle_dispute(&mut self, transaction: Transaction) -> Result<(), TransactionError> {
        // Get the referenced transaction
        let stored_tx = self
            .transactions
            .get(&transaction.tx)
            .ok_or(TransactionError::TransactionNotFound(transaction.tx))?
            .clone();

        // Verify the client ID matches (security check)
        if stored_tx.client != transaction.client {
            return Err(TransactionError::InvalidTransactionType(format!(
                "Client ID mismatch: transaction {} belongs to client {}, not client {}",
                transaction.tx, stored_tx.client, transaction.client
            )));
        }

        // Only deposit/withdrawal can be disputed
        if !matches!(
            stored_tx.tx_type,
            TransactionType::Deposit | TransactionType::Withdrawal
        ) {
            return Err(TransactionError::InvalidTransactionType(
                "Only deposits and withdrawals can be disputed".to_string(),
            ));
        }

        // Check if transaction is already disputed
        if self.disputes.contains_key(&transaction.tx) {
            // Already disputed - ignore silently per spec
            return Ok(());
        }

        // Get the client and hold the funds
        let client = self
            .clients
            .get_mut(&stored_tx.client)
            .ok_or(TransactionError::ClientNotFound(stored_tx.client))?;

        // Hold funds may fail if insufficient available funds, but we ignore per spec
        let _ = client.hold_funds(stored_tx.amount);

        // Record the dispute
        self.disputes
            .insert(transaction.tx, DisputeState::new(stored_tx.amount));

        Ok(())
    }

    /// Handle a resolve transaction
    fn handle_resolve(&mut self, transaction: Transaction) -> Result<(), TransactionError> {
        // Get the dispute state
        let dispute_state = self
            .disputes
            .get_mut(&transaction.tx)
            .ok_or(TransactionError::TransactionNotDisputed(transaction.tx))?;

        // Check if dispute is active (not already resolved/chargebacked)
        if !dispute_state.is_active() {
            return Ok(());
        }

        let amount = dispute_state.amount;

        // Get the original transaction to find the client
        let stored_tx = self
            .transactions
            .get(&transaction.tx)
            .ok_or(TransactionError::TransactionNotFound(transaction.tx))?;

        // Verify the client ID matches (security check)
        if stored_tx.client != transaction.client {
            return Err(TransactionError::InvalidTransactionType(format!(
                "Client ID mismatch: transaction {} belongs to client {}, not client {}",
                transaction.tx, stored_tx.client, transaction.client
            )));
        }

        let client = self
            .clients
            .get_mut(&stored_tx.client)
            .ok_or(TransactionError::ClientNotFound(stored_tx.client))?;

        // Release the held funds
        client.release_held(amount)?;

        // Mark dispute as resolved
        dispute_state.resolve();

        Ok(())
    }

    /// Handle a chargeback transaction
    fn handle_chargeback(&mut self, transaction: Transaction) -> Result<(), TransactionError> {
        // Get the dispute state
        let dispute_state = self
            .disputes
            .get_mut(&transaction.tx)
            .ok_or(TransactionError::TransactionNotDisputed(transaction.tx))?;

        // Check if dispute is active (not already resolved/chargebacked)
        if !dispute_state.is_active() {
            return Ok(());
        }

        let amount = dispute_state.amount;

        // Get the original transaction to find the client
        let stored_tx = self
            .transactions
            .get(&transaction.tx)
            .ok_or(TransactionError::TransactionNotFound(transaction.tx))?;

        // Verify the client ID matches (security check)
        if stored_tx.client != transaction.client {
            return Err(TransactionError::InvalidTransactionType(format!(
                "Client ID mismatch: transaction {} belongs to client {}, not client {}",
                transaction.tx, stored_tx.client, transaction.client
            )));
        }

        let client = self
            .clients
            .get_mut(&stored_tx.client)
            .ok_or(TransactionError::ClientNotFound(stored_tx.client))?;

        // Process chargeback: remove held funds, decrease total, lock account
        client.chargeback(amount)?;

        // Mark dispute as resolved
        dispute_state.resolve();

        Ok(())
    }

    /// Get all clients and their account states
    pub fn get_clients(&self) -> &HashMap<u16, ClientAccount> {
        &self.clients
    }

    /// Get a specific client's account
    pub fn get_client(&self, client_id: u16) -> Option<&ClientAccount> {
        self.clients.get(&client_id)
    }

    /// Get the number of clients
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Get the number of transactions processed
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }

    /// Get the number of active disputes
    pub fn dispute_count(&self) -> usize {
        self.disputes.values().filter(|d| d.is_active()).count()
    }
}

impl Default for TransactionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit() {
        let mut engine = TransactionEngine::new();
        let tx = Transaction::new(TransactionType::Deposit, 1, 1, Some(Decimal::from(100)));
        assert!(engine.process_transaction(tx).is_ok());
        assert_eq!(engine.client_count(), 1);

        let client = engine.get_client(1).unwrap();
        assert_eq!(client.available(), Decimal::from(100));
        assert_eq!(client.total(), Decimal::from(100));
    }

    #[test]
    fn test_withdrawal_success() {
        let mut engine = TransactionEngine::new();
        engine
            .process_transaction(Transaction::new(
                TransactionType::Deposit,
                1,
                1,
                Some(Decimal::from(100)),
            ))
            .ok();

        engine
            .process_transaction(Transaction::new(
                TransactionType::Withdrawal,
                1,
                2,
                Some(Decimal::from(50)),
            ))
            .ok();

        let client = engine.get_client(1).unwrap();
        assert_eq!(client.available(), Decimal::from(50));
        assert_eq!(client.total(), Decimal::from(50));
    }

    #[test]
    fn test_withdrawal_insufficient_funds() {
        let mut engine = TransactionEngine::new();
        engine
            .process_transaction(Transaction::new(
                TransactionType::Deposit,
                1,
                1,
                Some(Decimal::from(100)),
            ))
            .ok();

        engine
            .process_transaction(Transaction::new(
                TransactionType::Withdrawal,
                1,
                2,
                Some(Decimal::from(150)),
            ))
            .ok();

        let client = engine.get_client(1).unwrap();
        assert_eq!(client.available(), Decimal::from(100));
        assert_eq!(client.total(), Decimal::from(100));
    }

    #[test]
    fn test_dispute_and_resolve() {
        let mut engine = TransactionEngine::new();
        engine
            .process_transaction(Transaction::new(
                TransactionType::Deposit,
                1,
                1,
                Some(Decimal::from(100)),
            ))
            .ok();

        engine
            .process_transaction(Transaction::new(TransactionType::Dispute, 1, 1, None))
            .ok();

        let client = engine.get_client(1).unwrap();
        assert_eq!(client.available(), Decimal::from(0));
        assert_eq!(client.held(), Decimal::from(100));
        assert_eq!(client.total(), Decimal::from(100));

        engine
            .process_transaction(Transaction::new(TransactionType::Resolve, 1, 1, None))
            .ok();

        let client = engine.get_client(1).unwrap();
        assert_eq!(client.available(), Decimal::from(100));
        assert_eq!(client.held(), Decimal::ZERO);
        assert_eq!(client.total(), Decimal::from(100));
    }

    #[test]
    fn test_dispute_and_chargeback() {
        let mut engine = TransactionEngine::new();
        engine
            .process_transaction(Transaction::new(
                TransactionType::Deposit,
                1,
                1,
                Some(Decimal::from(100)),
            ))
            .ok();

        engine
            .process_transaction(Transaction::new(TransactionType::Dispute, 1, 1, None))
            .ok();

        engine
            .process_transaction(Transaction::new(TransactionType::Chargeback, 1, 1, None))
            .ok();

        let client = engine.get_client(1).unwrap();
        assert_eq!(client.available(), Decimal::ZERO);
        assert_eq!(client.held(), Decimal::ZERO);
        assert_eq!(client.total(), Decimal::ZERO);
        assert!(client.is_locked());
    }

    #[test]
    fn test_nonexistent_transaction_dispute() {
        let mut engine = TransactionEngine::new();
        let tx = Transaction::new(TransactionType::Dispute, 1, 999, None);
        assert!(engine.process_transaction(tx).is_err());
    }

    #[test]
    fn test_client_id_mismatch_in_dispute() {
        let mut engine = TransactionEngine::new();
        // Client 1 makes a deposit
        engine
            .process_transaction(Transaction::new(
                TransactionType::Deposit,
                1,
                1,
                Some(Decimal::from(100)),
            ))
            .ok();

        // Client 2 tries to dispute client 1's transaction - should fail
        let result =
            engine.process_transaction(Transaction::new(TransactionType::Dispute, 2, 1, None));
        assert!(result.is_err());
    }

    #[test]
    fn test_locked_account_blocks_operations() {
        let mut engine = TransactionEngine::new();
        // Setup: deposit, dispute, chargeback to lock account
        engine
            .process_transaction(Transaction::new(
                TransactionType::Deposit,
                1,
                1,
                Some(Decimal::from(100)),
            ))
            .ok();

        engine
            .process_transaction(Transaction::new(TransactionType::Dispute, 1, 1, None))
            .ok();

        engine
            .process_transaction(Transaction::new(TransactionType::Chargeback, 1, 1, None))
            .ok();

        let client = engine.get_client(1).unwrap();
        assert!(client.is_locked());

        // Try to deposit into locked account - should be processed but deposit will fail internally
        engine
            .process_transaction(Transaction::new(
                TransactionType::Deposit,
                1,
                2,
                Some(Decimal::from(50)),
            ))
            .ok();

        // Account should still be at zero
        let client = engine.get_client(1).unwrap();
        assert_eq!(client.total(), Decimal::ZERO);
    }
}
