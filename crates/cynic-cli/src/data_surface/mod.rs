//! Data domain CLI surface: schema validation, drift detection, contract templates, and SQL query projection.

pub mod history;
pub mod query;
pub mod render;
pub mod validate;

use clap::Subcommand;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Subcommand, Debug, Clone)]
pub enum DataCommand {
    /// Validate a CSV or JSONL file against a TabularSchema contract
    Validate {
        /// Path to the JSON schema file (TabularSchema)
        #[arg(long, short)]
        schema: PathBuf,
        /// Path to the tabular data file (CSV or JSONL)
        #[arg(long, short)]
        file: PathBuf,
        /// Output validation report as JSON
        #[arg(long)]
        json: bool,
        /// Record validation run in evidence history (evidence/data_validation_history.jsonl)
        #[arg(long)]
        record: bool,
    },
    /// Emit a canonical TabularSchema contract for reference
    Schema {
        /// Schema kind: 'asset' (IT Asset Management) or 'claim' (NatCat Claims)
        #[arg(long, default_value = "asset")]
        kind: String,
    },
    /// View historical tabular validation runs from evidence history
    History {
        /// Limit number of historical records to display
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Output history as JSON
        #[arg(long)]
        json: bool,
        /// Verify the cryptographic integrity of the cold-evidence journal chain
        #[arg(long)]
        verify: bool,
        /// Execute arbitrary SQL query against in-memory projection of history reports and journal entries
        #[arg(long)]
        sql: Option<String>,
    },
    /// Query a validated CSV or JSONL file with arbitrary SQL using in-memory SQLite projection
    Query {
        /// Path to the tabular data file (CSV or JSONL). Optional when querying --history.
        #[arg(long, short)]
        file: Option<PathBuf>,
        /// Path to the JSON schema file, or canonical schema name ('session', 'turn', 'asset', 'natcat', 'chunk', 'claim', 'history', 'journal')
        #[arg(long, short)]
        schema: Option<String>,
        /// Query historical validation runs and journal proofs instead of a local file
        #[arg(long)]
        history: bool,
        /// SQL query to execute (the table is accessible via its schema name or simply 'data')
        #[arg(long)]
        sql: String,
        /// Output query results as structured JSON
        #[arg(long)]
        json: bool,
    },
}

pub fn execute_data(command: DataCommand) -> ExitCode {
    match command {
        DataCommand::Validate { schema, file, json, record } => {
            validate::execute_validate(&schema, &file, json, record)
        }
        DataCommand::Schema { kind } => validate::execute_schema(&kind),
        DataCommand::History { limit, json, verify, sql } => {
            history::execute_history(limit, json, verify, sql.as_deref())
        }
        DataCommand::Query { file, schema, history, sql, json } => {
            query::execute_query(file.as_deref(), schema.as_deref(), history, &sql, json)
        }
    }
}
