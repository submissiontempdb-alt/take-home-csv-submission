use crate::domain::CliError;
use crate::infrastructure::{CsvReader, CsvWriter};
use std::env;
use transaction::TransactionEngine;

/// CLI application handler
pub struct Cli;

impl Cli {
    /// Synchronous entry point for CLI usage
    ///
    /// For a server bundling this engine to handle concurrent TCP streams,
    /// you could use the async version with tokio like:
    /// ```ignore
    /// #[tokio::main]
    /// async fn main() {
    ///     if let Err(e) = Cli::run_async().await {
    ///         eprintln!("Error: {}", e);
    ///         process::exit(1);
    ///     }
    /// }
    /// ```
    pub fn run() -> Result<(), CliError> {
        // Get the input file path from command line arguments
        let args: Vec<String> = env::args().collect();
        if args.len() != 2 {
            return Err(CliError::InvalidInput(
                "Usage: cargo run -- transactions.csv".to_string(),
            ));
        }

        let input_file = &args[1];

        // Create a new transaction engine
        let mut engine = TransactionEngine::new();

        // Read and process transactions from CSV
        CsvReader::process_csv(&mut engine, input_file)?;

        // Write output to stdout
        CsvWriter::write_output(&engine)?;

        Ok(())
    }
}
