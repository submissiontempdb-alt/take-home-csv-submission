use crate::domain::{CliError, OutputRow};
use csv::WriterBuilder;
use std::io;
use transaction::TransactionEngine;

/// Handles CSV writing for client account states
pub struct CsvWriter;

impl CsvWriter {
    /// Write client account states to stdout as CSV
    ///
    /// # Output Strategy
    ///
    /// - **Streaming output**: Results are written line-by-line to stdout without buffering
    ///   the entire output in memory first.
    /// - **Sorted by client ID**: Output is sorted for consistent, deterministic results
    ///   across runs (small overhead: O(c log c) where c = number of clients).
    /// - **Formatted precision**: All amounts are formatted to 4 decimal places as per spec.
    ///
    /// # Memory Characteristics
    ///
    /// - Output generation only requires O(c) memory where c = number of unique clients
    /// - Even with 65,535 clients, this is negligible compared to transaction processing
    /// - CSV writer buffers output internally (~8KB), not the entire result set
    pub fn write_output(engine: &TransactionEngine) -> Result<(), CliError> {
        let mut writer = WriterBuilder::new().from_writer(io::stdout());

        // Get all clients and sort by client ID for consistent output
        let mut clients: Vec<_> = engine.get_clients().iter().collect();
        clients.sort_by_key(|(id, _)| *id);

        // Write each client's account state
        for (client_id, account) in clients {
            let row = OutputRow {
                client: *client_id,
                // Format to 4 decimal places
                available: format!("{:.4}", account.available()),
                held: format!("{:.4}", account.held()),
                total: format!("{:.4}", account.total()),
                locked: account.is_locked(),
            };

            writer.serialize(row)?;
        }

        writer.flush()?;
        Ok(())
    }
}
