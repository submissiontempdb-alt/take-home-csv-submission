use crate::domain::{CliError, InputRow};
use csv::{ReaderBuilder, Trim};
use std::sync::Arc;
use tokio::sync::Mutex;
use transaction::{Transaction, TransactionEngine};

/// Transaction processing orchestration
pub struct Processor;

impl Processor {
    /// Async entry point for server deployments with shared engine
    ///
    /// This function demonstrates how the architecture scales to concurrent scenarios
    /// with a SINGLE shared TransactionEngine across multiple async tasks:
    ///
    /// 1. A single `Arc<Mutex<TransactionEngine>>` is created once
    /// 2. Multiple async tasks (one per TCP stream) clone the Arc
    /// 3. Each task acquires the lock when processing its CSV
    /// 4. Lock is held only during transaction processing
    /// 5. All transactions from all streams go into the same engine
    ///
    /// This approach is useful for:
    /// - **Multi-stream aggregation**: Combining transactions from multiple sources
    /// - **Shared ledger**: All streams contribute to a single global state
    /// - **Eventual consistency**: Each stream's transactions are immediately visible to others
    ///
    /// Tradeoffs:
    /// - ✓ Single engine = single source of truth, easier reasoning
    /// - ✓ Lock contention minimal if processing is fast
    /// - ✗ Lock bottleneck if streams process huge batches simultaneously
    /// - ✗ One slow stream can block others waiting for the lock
    ///
    /// For production scenarios with thousands of concurrent streams where contention is
    /// a concern, consider the isolated engine pattern (see `process_csv_isolated()`).
    #[allow(dead_code)]
    pub async fn process_csv_shared(
        engine: &Arc<Mutex<TransactionEngine>>,
        file_path: &str,
    ) -> Result<(), CliError> {
        let mut reader = ReaderBuilder::new()
            .trim(Trim::All)
            .flexible(false)
            .from_path(file_path)?;

        for (line_num, result) in reader.deserialize::<InputRow>().enumerate() {
            let line_num = line_num + 2;

            match result {
                Ok(row) => {
                    // Process the CSV line
                    let _ = Self::process_shared_csv_line(engine, row, line_num).await;
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse line {}: {}", line_num, e);
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Process a single CSV line with a shared engine
    async fn process_shared_csv_line(
        engine: &Arc<Mutex<TransactionEngine>>,
        row: InputRow,
        line_num: usize,
    ) -> Result<(), CliError> {
        // Create and process transaction
        let transaction = Transaction::try_from(row)?;

        // Acquire lock only for the actual transaction processing
        let mut locked_engine = engine.lock().await;
        if let Err(e) = locked_engine.process_transaction(transaction) {
            eprintln!(
                "Warning: Failed to process transaction on line {}: {}",
                line_num, e
            );
        }
        // Lock is released here when locked_engine is dropped

        Ok(())
    }
}

// Example: Multiple concurrent streams processing into a single shared engine
//
// ```ignore
// #[tokio::main]
// async fn main() {
//     // Create ONCE, share across all connections
//     let engine = Arc::new(Mutex::new(TransactionEngine::new()));
//
//     let listener = TcpListener::bind("127.0.0.1:8080").await?;
//
//     loop {
//         let (socket, _) = listener.accept().await?;
//
//         // Clone the Arc (cheap) for this stream
//         let engine = engine.clone();
//
//         // Spawn handler
//         tokio::spawn(async move {
//             let mut reader = BufReader::new(socket);
//             Processor::process_csv_shared(&engine, stream).await?;
//         });
//     }
// }
// ```
