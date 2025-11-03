use crate::domain::CliError;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;
use transaction::{Transaction, TransactionType};

/// Represents a row from the input CSV
#[derive(Debug, Deserialize)]
pub struct InputRow {
    #[serde(rename = "type")]
    pub tx_type: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

impl TryFrom<InputRow> for Transaction {
    type Error = CliError;

    fn try_from(row: InputRow) -> Result<Self, Self::Error> {
        let tx_type = match TransactionType::from_str(row.tx_type.trim()) {
            Ok(t) => t,
            Err(_) => {
                return Err(CliError::InvalidInput(format!(
                    "Invalid transaction type '{}'",
                    row.tx_type
                )));
            }
        };

        let amount = match tx_type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                if row.amount.is_none() {
                    return Err(CliError::InvalidInput(format!(
                        "{} transaction missing amount for tx {}",
                        tx_type, row.tx
                    )));
                }
                row.amount
            }
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => {
                None
            }
        };

        Ok(Transaction::new(tx_type, row.client, row.tx, amount))
    }
}
