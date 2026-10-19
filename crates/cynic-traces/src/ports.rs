//! Canonical ports — what the canonical layer needs from the outside world.
//!
//! Strict port of `apps/cynic-data/src/cynic_data/canonical/ports.py`.
//! The application layer never supplies session ids: where sessions live and
//! what identifies one is harness knowledge that belongs to the adapter.

use super::conservation::ConservationContract;
use super::session::NormalizedSession;

/// A harness adapter: one bound source -> canonical sessions.
///
/// The adapter, not the caller, decides what a session is. Only the
/// normalized domain object crosses the port.
pub trait NormalizerPort {
    /// Which harness this adapter reads (e.g. `"hermes"`).
    fn harness(&self) -> &str;

    /// Whether this adapter can normalize the given source reference.
    fn can_read(&self, source_ref: &str) -> bool;

    /// The conservation contract for this harness, or None if none exists yet
    /// (an explicit unknown, never an implicit "no loss").
    fn conservation(&self) -> Option<ConservationContract>;

    /// Enumerate this source's sessions. Empty when the source is absent.
    /// Returned values are opaque: they mean nothing outside this adapter.
    fn session_refs(&self) -> Vec<String>;

    /// Normalize one session of this adapter's bound source.
    fn normalize(&self, session_ref: &str) -> Result<NormalizedSession, String>;
}

/// Persistence for canonical entities. A store never invents data.
pub trait CanonicalStorePort {
    type Error: std::error::Error;

    fn write(&self, session: &NormalizedSession) -> Result<(), Self::Error>;

    fn read(&self, session_uid: &str) -> Result<Option<NormalizedSession>, Self::Error>;

    fn count(&self, table: &str) -> Result<i64, Self::Error>;
}
