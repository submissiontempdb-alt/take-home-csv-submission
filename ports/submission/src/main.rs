//! CSV transaction processing CLI
//!
//! This binary reads transactions from a CSV file, processes them through the transaction engine,
//! and outputs the final client account states to stdout.
//!
//! # Usage
//! ```bash
//! cargo run -- transactions.csv > accounts.csv
//! ```
//!
//! # Resource Efficiency & Scalability
//!
//! This implementation is designed with system resources and scalability in mind:
//!
//! ## Memory Efficiency
//! - **Streaming CSV Processing**: Transactions are processed line-by-line as they're read from the
//!   CSV file, not loaded into memory upfront. The CSV reader buffers only a small window of data
//!   (~8KB by default), regardless of file size.
//! - **In-Memory Data Structures**: Only essential data is kept in memory:
//!   - Client accounts: ~200 bytes per client × u16 max (65,535 clients) = ~13 MB max
//!   - Transaction history: ~50 bytes per transaction × u32 max = millions of transactions possible
//!   - Estimated total for 1M transactions: ~65-100 MB (well within system resources)
//! - **HashMap O(1) Lookups**: Disputes reference past transactions by ID; HashMaps provide
//!   constant-time lookups regardless of dataset size.
//!
//! ## Large Dataset Support
//! - Supports up to 2³² (4.3 billion) unique transactions without performance degradation
//! - Client IDs are u16 (max 65,535), so memory scales with unique clients, not transaction count
//! - Line-by-line processing means memory usage is independent of CSV file size
//!
//! ## Concurrent Server Scenarios
//! - The `TransactionEngine` is designed to be stateless per transaction and thread-safe
//! - Multiple concurrent requests can each create their own `TransactionEngine` instance
//! - For a server bundling these CSVs from thousands of concurrent TCP streams:
//!   - Each stream gets its own engine instance (isolated state)
//!   - Async runtime (tokio) coordinates I/O efficiently without blocking threads
//!   - Each stream's memory footprint is independent and bounded
//!   - No shared state = no contention, locks, or coordination overhead
//! - Example: 10,000 concurrent streams × 1MB per stream = 10GB total (manageable with proper resource limits)
//!
//! # Performance Characteristics
//! - **Time Complexity**: O(n) where n = number of transactions (single pass through CSV)
//! - **Space Complexity**: O(c + t) where c = unique clients, t = stored transactions
//!   - c ≤ 65,535 (u16 limit)
//!   - t ≤ 4,294,967,295 (u32 limit, but practically limited by available memory)
//! - **Dispute Operations**: O(1) constant-time lookups for dispute/resolve/chargeback

mod application;
mod domain;
mod infrastructure;

use application::Cli;
use std::process;

fn main() {
    if let Err(e) = Cli::run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use crate::infrastructure::CsvReader;
    use rust_decimal::Decimal;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use transaction::TransactionEngine;

    /// Helper function to create a temporary CSV file with given content
    fn create_test_csv(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    /// Helper function to process CSV and return the engine
    fn process_csv_content(content: &str) -> TransactionEngine {
        let file = create_test_csv(content);
        let mut engine = TransactionEngine::new();
        CsvReader::process_csv(&mut engine, file.path().to_str().unwrap()).unwrap();
        engine
    }

    mod basic_transactions {
        use super::*;

        #[test]
        fn test_single_deposit() {
            let engine = process_csv_content("type,client,tx,amount\ndeposit,1,1,100.0");

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.available(), Decimal::from(100));
            assert_eq!(client.total(), Decimal::from(100));
            assert_eq!(client.held(), Decimal::ZERO);
            assert!(!client.is_locked());
        }

        #[test]
        fn test_multiple_deposits() {
            let engine = process_csv_content(
                "type,client,tx,amount\ndeposit,1,1,100.0\ndeposit,1,2,50.0\ndeposit,1,3,25.5",
            );

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.available(), Decimal::new(1755, 1)); // 175.5
            assert_eq!(client.total(), Decimal::new(1755, 1));
        }

        #[test]
        fn test_deposit_and_withdrawal() {
            let engine = process_csv_content(
                "type,client,tx,amount\ndeposit,1,1,100.0\nwithdrawal,1,2,50.0",
            );

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.available(), Decimal::from(50));
            assert_eq!(client.total(), Decimal::from(50));
        }

        #[test]
        fn test_insufficient_funds_withdrawal() {
            let engine = process_csv_content(
                "type,client,tx,amount\ndeposit,1,1,100.0\nwithdrawal,1,2,150.0",
            );

            let client = engine.get_client(1).unwrap();
            // Withdrawal should be rejected, balance remains
            assert_eq!(client.available(), Decimal::from(100));
            assert_eq!(client.total(), Decimal::from(100));
        }

        #[test]
        fn test_exact_balance_withdrawal() {
            let engine = process_csv_content(
                "type,client,tx,amount\ndeposit,1,1,100.0\nwithdrawal,1,2,100.0",
            );

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.available(), Decimal::ZERO);
            assert_eq!(client.total(), Decimal::ZERO);
        }

        #[test]
        fn test_multiple_clients() {
            let engine = process_csv_content(
                "type,client,tx,amount\n\
                 deposit,1,1,100.0\n\
                 deposit,2,2,200.0\n\
                 deposit,3,3,300.0\n\
                 withdrawal,1,4,25.0\n\
                 withdrawal,2,5,50.0",
            );

            let client1 = engine.get_client(1).unwrap();
            assert_eq!(client1.available(), Decimal::from(75));

            let client2 = engine.get_client(2).unwrap();
            assert_eq!(client2.available(), Decimal::from(150));

            let client3 = engine.get_client(3).unwrap();
            assert_eq!(client3.available(), Decimal::from(300));
        }
    }

    mod dispute_resolution {
        use super::*;

        #[test]
        fn test_dispute_and_resolve() {
            let engine = process_csv_content(
                "type,client,tx,amount\ndeposit,1,1,100.0\ndispute,1,1,\nresolve,1,1,",
            );

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.available(), Decimal::from(100));
            assert_eq!(client.held(), Decimal::ZERO);
            assert_eq!(client.total(), Decimal::from(100));
            assert!(!client.is_locked());
        }

        #[test]
        fn test_dispute_holds_funds() {
            let engine =
                process_csv_content("type,client,tx,amount\ndeposit,1,1,100.0\ndispute,1,1,");

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.available(), Decimal::ZERO);
            assert_eq!(client.held(), Decimal::from(100));
            assert_eq!(client.total(), Decimal::from(100));
        }

        #[test]
        fn test_dispute_nonexistent_transaction() {
            let engine =
                process_csv_content("type,client,tx,amount\ndeposit,1,1,100.0\ndispute,1,999,");

            let client = engine.get_client(1).unwrap();
            // Dispute should be ignored
            assert_eq!(client.available(), Decimal::from(100));
            assert_eq!(client.held(), Decimal::ZERO);
        }

        #[test]
        fn test_resolve_without_dispute() {
            let engine =
                process_csv_content("type,client,tx,amount\ndeposit,1,1,100.0\nresolve,1,1,");

            let client = engine.get_client(1).unwrap();
            // Resolve should be ignored
            assert_eq!(client.available(), Decimal::from(100));
            assert_eq!(client.held(), Decimal::ZERO);
        }

        #[test]
        fn test_multiple_disputes_same_transaction() {
            let engine = process_csv_content(
                "type,client,tx,amount\ndeposit,1,1,100.0\ndispute,1,1,\ndispute,1,1,",
            );

            let client = engine.get_client(1).unwrap();
            // Second dispute should be ignored
            assert_eq!(client.available(), Decimal::ZERO);
            assert_eq!(client.held(), Decimal::from(100));
            assert_eq!(client.total(), Decimal::from(100));
        }

        #[test]
        fn test_multiple_resolves_same_transaction() {
            let engine = process_csv_content(
                "type,client,tx,amount\ndeposit,1,1,100.0\ndispute,1,1,\nresolve,1,1,\nresolve,1,1,",
            );

            let client = engine.get_client(1).unwrap();
            // Second resolve should be ignored
            assert_eq!(client.available(), Decimal::from(100));
            assert_eq!(client.held(), Decimal::ZERO);
        }

        #[test]
        fn test_partial_withdrawal_then_dispute() {
            let engine = process_csv_content(
                "type,client,tx,amount\n\
                 deposit,1,1,100.0\n\
                 withdrawal,1,2,30.0\n\
                 dispute,1,1,",
            );

            let client = engine.get_client(1).unwrap();
            // After withdrawal, available is 70, total is 70
            // The dispute is rejected/ignored since funds have been withdrawn
            assert_eq!(client.available(), Decimal::from(70));
            assert_eq!(client.held(), Decimal::ZERO); // Dispute rejected
            assert_eq!(client.total(), Decimal::from(70));
        }
    }

    mod chargeback {
        use super::*;

        #[test]
        fn test_chargeback_locks_account() {
            let engine = process_csv_content(
                "type,client,tx,amount\ndeposit,1,1,100.0\ndispute,1,1,\nchargeback,1,1,",
            );

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.total(), Decimal::ZERO);
            assert_eq!(client.held(), Decimal::ZERO);
            assert_eq!(client.available(), Decimal::ZERO);
            assert!(client.is_locked());
        }

        #[test]
        fn test_chargeback_without_dispute() {
            let engine =
                process_csv_content("type,client,tx,amount\ndeposit,1,1,100.0\nchargeback,1,1,");

            let client = engine.get_client(1).unwrap();
            // Chargeback should be ignored without prior dispute
            assert_eq!(client.available(), Decimal::from(100));
            assert!(!client.is_locked());
        }

        #[test]
        fn test_transactions_after_chargeback_ignored() {
            let engine = process_csv_content(
                "type,client,tx,amount\n\
                 deposit,1,1,100.0\n\
                 dispute,1,1,\n\
                 chargeback,1,1,\n\
                 deposit,1,2,50.0\n\
                 withdrawal,1,3,25.0",
            );

            let client = engine.get_client(1).unwrap();
            // Account is locked, subsequent transactions should be ignored
            assert_eq!(client.total(), Decimal::ZERO);
            assert!(client.is_locked());
        }

        #[test]
        fn test_multiple_chargebacks_same_transaction() {
            let engine = process_csv_content(
                "type,client,tx,amount\ndeposit,1,1,100.0\ndispute,1,1,\nchargeback,1,1,\nchargeback,1,1,",
            );

            let client = engine.get_client(1).unwrap();
            // Second chargeback should be ignored
            assert_eq!(client.total(), Decimal::ZERO);
            assert!(client.is_locked());
        }
    }

    mod decimal_precision {
        use super::*;

        #[test]
        fn test_four_decimal_places() {
            let engine = process_csv_content("type,client,tx,amount\ndeposit,1,1,100.1234");

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.available(), Decimal::new(1001234, 4));
        }

        #[test]
        fn test_small_amounts() {
            let engine = process_csv_content(
                "type,client,tx,amount\ndeposit,1,1,0.0001\ndeposit,1,2,0.0001",
            );

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.available(), Decimal::new(2, 4)); // 0.0002
        }

        #[test]
        fn test_mixed_precision() {
            let engine = process_csv_content(
                "type,client,tx,amount\n\
                 deposit,1,1,100\n\
                 deposit,1,2,50.5\n\
                 deposit,1,3,25.25\n\
                 deposit,1,4,10.125",
            );

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.available(), Decimal::new(185875, 3)); // 185.875
        }

        #[test]
        fn test_rounding_precision() {
            let engine = process_csv_content("type,client,tx,amount\ndeposit,1,1,0.12345");

            let client = engine.get_client(1).unwrap();
            // Should be truncated/rounded to 4 decimal places: 0.1234 or 0.1235
            let balance = client.available();
            assert!(balance >= Decimal::new(1234, 4) && balance <= Decimal::new(1235, 4));
        }
    }

    mod csv_format_handling {
        use super::*;

        #[test]
        fn test_whitespace_in_fields() {
            let engine =
                process_csv_content("type,  client, tx, amount\n  deposit,  1,  1,  100.0  ");

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.available(), Decimal::from(100));
        }

        #[test]
        fn test_empty_amount_for_dispute() {
            let engine =
                process_csv_content("type,client,tx,amount\ndeposit,1,1,100.0\ndispute,1,1,");

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.held(), Decimal::from(100));
        }

        #[test]
        fn test_malformed_lines_ignored() {
            let file = create_test_csv(
                "type,client,tx,amount\n\
                 deposit,1,1,100.0\n\
                 invalid_type,1,2,50.0\n\
                 deposit,1,3,25.0",
            );

            let mut engine = TransactionEngine::new();
            // Should not panic, invalid lines are ignored
            let _ = CsvReader::process_csv(&mut engine, file.path().to_str().unwrap());

            let client = engine.get_client(1).unwrap();
            // Should have processed valid transactions
            assert_eq!(client.available(), Decimal::from(125));
        }

        #[test]
        fn test_empty_file() {
            let engine = process_csv_content("type,client,tx,amount\n");

            // Should not panic, no clients created
            assert!(engine.get_client(1).is_none());
        }

        #[test]
        fn test_only_header() {
            let file = create_test_csv("type,client,tx,amount");
            let mut engine = TransactionEngine::new();
            let result = CsvReader::process_csv(&mut engine, file.path().to_str().unwrap());

            assert!(result.is_ok());
            assert!(engine.get_client(1).is_none());
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn test_zero_amount_deposit() {
            let engine = process_csv_content("type,client,tx,amount\ndeposit,1,1,0.0");

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.available(), Decimal::ZERO);
        }

        #[test]
        fn test_withdrawal_from_empty_account() {
            let engine = process_csv_content("type,client,tx,amount\nwithdrawal,1,1,50.0");

            // Client might not exist or have zero balance
            if let Some(client) = engine.get_client(1) {
                assert_eq!(client.available(), Decimal::ZERO);
            }
        }

        #[test]
        fn test_dispute_withdrawal_transaction() {
            let engine = process_csv_content(
                "type,client,tx,amount\n\
                 deposit,1,1,100.0\n\
                 withdrawal,1,2,50.0\n\
                 dispute,1,2,",
            );

            let client = engine.get_client(1).unwrap();
            // Disputing a withdrawal should increase held funds
            assert_eq!(client.total(), Decimal::from(50));
        }

        #[test]
        fn test_large_transaction_ids() {
            let engine = process_csv_content(
                "type,client,tx,amount\n\
                 deposit,1,4294967295,100.0\n\
                 deposit,1,4294967294,50.0",
            );

            let client = engine.get_client(1).unwrap();
            assert_eq!(client.available(), Decimal::from(150));
        }

        #[test]
        fn test_large_client_ids() {
            let engine = process_csv_content(
                "type,client,tx,amount\n\
                 deposit,65535,1,100.0\n\
                 deposit,65534,2,200.0",
            );

            let client1 = engine.get_client(65535).unwrap();
            assert_eq!(client1.available(), Decimal::from(100));

            let client2 = engine.get_client(65534).unwrap();
            assert_eq!(client2.available(), Decimal::from(200));
        }

        #[test]
        fn test_complex_multi_client_scenario() {
            let engine = process_csv_content(
                "type,client,tx,amount\n\
                 deposit,1,1,100.0\n\
                 deposit,2,2,200.0\n\
                 deposit,1,3,50.0\n\
                 withdrawal,2,4,75.0\n\
                 dispute,1,1,\n\
                 deposit,2,5,25.0\n\
                 resolve,1,1,\n\
                 withdrawal,1,6,30.0\n\
                 dispute,2,4,\n\
                 chargeback,2,4,",
            );

            let client1 = engine.get_client(1).unwrap();
            assert_eq!(client1.available(), Decimal::from(120)); // 100 + 50 - 30
            assert!(!client1.is_locked());

            let client2 = engine.get_client(2).unwrap();
            assert!(client2.is_locked()); // Chargeback occurred
        }
    }

    mod integration {
        use super::*;

        #[test]
        fn test_realistic_transaction_sequence() {
            let engine = process_csv_content(
                "type,client,tx,amount\n\
                 deposit,1,1,1000.0\n\
                 deposit,1,2,500.0\n\
                 withdrawal,1,3,250.0\n\
                 withdrawal,1,4,100.0\n\
                 deposit,1,5,75.50\n\
                 withdrawal,1,6,25.25",
            );

            let client = engine.get_client(1).unwrap();
            // 1000 + 500 - 250 - 100 + 75.50 - 25.25 = 1200.25
            assert_eq!(client.available(), Decimal::new(120025, 2));
            assert_eq!(client.total(), Decimal::new(120025, 2));
            assert_eq!(client.held(), Decimal::ZERO);
            assert!(!client.is_locked());
        }

        #[test]
        fn test_concurrent_disputes_different_transactions() {
            let engine = process_csv_content(
                "type,client,tx,amount\n\
                 deposit,1,1,100.0\n\
                 deposit,1,2,200.0\n\
                 deposit,1,3,300.0\n\
                 dispute,1,1,\n\
                 dispute,1,2,\n\
                 resolve,1,1,\n\
                 dispute,1,3,",
            );

            let client = engine.get_client(1).unwrap();
            // Total: 600, Held: 200 + 300 = 500, Available: 100
            assert_eq!(client.total(), Decimal::from(600));
            assert_eq!(client.held(), Decimal::from(500)); // tx 2 and 3
            assert_eq!(client.available(), Decimal::from(100));
        }
    }
}
