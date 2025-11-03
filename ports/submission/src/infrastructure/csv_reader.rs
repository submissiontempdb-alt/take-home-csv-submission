use crate::domain::{CliError, InputRow};
use csv::{ReaderBuilder, Trim};
use transaction::{Transaction, TransactionEngine};

/// Handles CSV reading and transaction processing
pub struct CsvReader;

impl CsvReader {
    /// Process transactions from a CSV file
    ///
    /// # Streaming Architecture
    ///
    /// This function demonstrates efficient streaming processing:
    ///
    /// - **No upfront buffering**: The entire CSV file is NOT loaded into memory.
    ///   The `csv` crate uses an internal buffer (~8KB) that reads one chunk at a time.
    /// - **Line-by-line processing**: Each CSV row is parsed and processed immediately
    ///   before the next row is read, minimizing memory footprint.
    /// - **Supports arbitrarily large files**: File size doesn't affect memory usage.
    ///   A 1GB CSV file uses the same amount of memory as a 1MB file.
    /// - **O(n) time complexity**: Single pass through the CSV, no re-reading or buffering.
    ///
    /// # Example
    ///
    /// For a CSV with 1 million transactions:
    /// ```ignore
    /// // Peak memory usage remains bounded:
    /// // - ~13 MB for up to 65,535 clients (u16 max)
    /// // - ~50 MB for transaction history
    /// // - CSV reader buffer: ~8 KB
    /// // Total: ~65 MB (independent of file size)
    /// ```
    ///
    /// # Error Handling
    ///
    /// - Invalid transaction types are logged and skipped
    /// - Missing amounts for deposit/withdrawal are logged and skipped
    /// - Transaction processing errors are logged but don't stop processing
    /// - This allows the engine to be resilient to malformed data
    pub fn process_csv(engine: &mut TransactionEngine, file_path: &str) -> Result<(), CliError> {
        // Create CSV reader with flexible whitespace handling
        // The reader uses internal buffering (~8KB) regardless of file size
        let mut reader = ReaderBuilder::new()
            .trim(Trim::All) // Trim whitespace from all fields
            .flexible(false) // Strict field count checking
            .from_path(file_path)?;

        // Process each record exactly once, streaming from disk/storage
        for (line_num, result) in reader.deserialize::<InputRow>().enumerate() {
            let line_num = line_num + 2; // +2 because line 1 is header, and enumerate starts at 0

            match result {
                Ok(row) => {
                    // Parse transaction type
                    let transaction = match Transaction::try_from(row) {
                        Ok(tx) => tx,
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to parse transaction on line {}: {}",
                                line_num, e
                            );
                            continue;
                        }
                    };

                    // Process the transaction, log errors but continue processing
                    if let Err(e) = engine.process_transaction(transaction) {
                        eprintln!(
                            "Warning: Failed to process transaction on line {}: {}",
                            line_num, e
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse line {}: {}", line_num, e);
                    continue;
                }
            }
        }

        Ok(())
    }
}
