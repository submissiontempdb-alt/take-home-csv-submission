//! Transaction engine library
//!
//! This library provides the core data structures and logic for processing financial transactions.
//! It handles deposits, withdrawals, disputes, resolutions, and chargebacks with proper
//! decimal precision and type safety.

use rust_decimal::Decimal;
use thiserror::Error;

mod client;
mod dispute;
mod engine;
mod transaction;

pub use client::ClientAccount;
pub use dispute::DisputeState;
pub use engine::TransactionEngine;
pub use transaction::{Transaction, TransactionType};

use crate::client::ClientError;

/// Errors that can occur during transaction processing
#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("Insufficient funds for withdrawal: available {available}, requested {requested}")]
    InsufficientFunds {
        available: Decimal,
        requested: Decimal,
    },

    #[error("Transaction not found: {0}")]
    TransactionNotFound(u32),

    #[error("Transaction is not disputed: {0}")]
    TransactionNotDisputed(u32),

    #[error("Client not found: {0}")]
    ClientNotFound(u16),

    #[error("Invalid transaction type: {0}")]
    InvalidTransactionType(String),

    #[error("Invalid decimal precision: {0}")]
    InvalidDecimalPrecision(String),

    #[error("CSV parsing error: {0}")]
    CsvError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Client error: {0}")]
    ClientError(#[from] ClientError),
}
