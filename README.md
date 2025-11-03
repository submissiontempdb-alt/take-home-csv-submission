# Take Home CSV Submission

A simple Rust application to process a series of financial transactions from a CSV file and output the resulting account states to stdout. 

## Design Decisions 

- Pureposefully did not include any kind of comprehensive logging to ensure automated tests could easily validate output.
- Given that this is a "simple toy" project, I have not implemented any persistent storage. All data is held in memory.
- Used `rust_decimal` crate for accurate decimal arithmetic to avoid floating-point precision issues.
- Focused heavily on writing clear and descriptive comments to explain the logic, flow of the application and my thought process.
- Used `thiserror` crate for error handling to provide clear and concise error messages.
- Though this application has got concurrency support, it is not strictly necessary for the provided requirements. It is included to demonstrate the ability to handle multiple CSV streams concurrently.
- Focused on readability and maintainability of the codebase over performance optimisations, given the scope of the project.

## Features

- Processes transactions in the order they are received.
- Supports deposits, withdrawals, disputes, resolves, and chargebacks.
- Handles edge cases such as insufficient funds, non-existent transactions, and locked accounts.
- Contains an in-memory representation of accounts and transactions.
- Provides a CLI interface for reading from a CSV file and outputting account states to stdout.

## Usage

### Rust Docs

To generate and view the Rust documentation for this project, run the following command in the project root:

```bash
cargo doc --open
```

### Build and Run

To build the project, navigate to the project root and run:

```bash
cargo build
```

To run the application with a CSV file named `transactions.csv` and output the results to `accounts.csv`, use the following command:

```bash
cargo run -- transactions.csv > accounts.csv
```

## Testing

To run the tests for this project, use the following command:

```bash
cargo test
```

## Project Structure

- `libraries/`: Contains the core libraries for the application.
    - `transaction/`: Core transaction processing logic and domain models.
- `ports/`: Contains the application ports, including the submission port which handles CSV input/output.
    - `submission/`: Handles CSV reading and writing, and orchestrates the transaction processing.

## Safety Notes

- I've decided to reject a dispute if the client does not have sufficient available funds to cover the held amount. This is to prevent negative balances and maintain the integrity of the account states. In a real-world scenario, further business rules and validations would be necessary to handle such cases appropriately.

## AI Usage

### Assisted by AI Explicitly

- Helped write Rust comments/docs to make them more descriptive and clearer. Provided clear guidance on the context and purpose of each comment.
- Generated the supporting files that I used to test out the CLI. These can be found in the `supporting_files` directory.
- Helped write unit tests for the submission layer.

### Not Assisted by AI

- Any kind of application logic or core functionality.
- Error handling and overall application structure.
- Enums/Structs definitions and implementations.
- Any kind of design decisions.
- This README file.
- etc.