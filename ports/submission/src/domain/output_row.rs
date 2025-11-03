use serde::Serialize;

/// Represents a row for the output CSV
#[derive(Debug, Serialize)]
pub struct OutputRow {
    pub client: u16,
    pub available: String,
    pub held: String,
    pub total: String,
    pub locked: bool,
}
