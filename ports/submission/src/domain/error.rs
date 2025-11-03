use thiserror::Error;
use transaction::TransactionError;

/// Errors that can occur during CSV processing
#[derive(Error, Debug)]
pub enum CliError {
    #[error("Transaction error: {0}")]
    Transaction(#[from] TransactionError),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Failed to parse transaction type: {0}")]
    ParseTransactionType(#[from] strum::ParseError),
}
