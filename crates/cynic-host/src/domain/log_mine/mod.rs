//! Log mining: pure parsers turning host logs into minable evidence.
//!
//! POURQUOI: operator logs (kern.log, journal, cynic traces) hold failure
//! signatures as raw bytes; unmined they are disk debt, mined they are cold
//! evidence feeding `evidence_journal` + `pattern_distill`.
//! COMMENT: pure line parsers, no I/O. Adapters read files, this module
//! classifies lines into signatures with counts and samples.
//! CE QU'IL NE FAIT PAS: never reads files, never writes the journal, never
//! deletes logs. Mining observes; retention evicts only after sealing.

pub mod probe;
pub mod types;

pub use probe::{SignatureCount, mine_log_lines};
pub use types::{LogMineReport, LogSignature};
