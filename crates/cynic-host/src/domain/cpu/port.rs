//! CPU observation port (4-layer report assembly).
//!
//! `observe_cpu` is implemented by `LocalCpuAdapter` (reads `/proc`/`/sys`)
//! and `RemoteCpuAdapter<R: ArgvRunner>` (SSH, injected runner). Tests use
//! `MapArgvRunner` against a fixed payload — no real `/proc` required.

use crate::domain::cpu::CpuReport;

pub type CpuObservationError = String;

pub trait CpuObservationPort {
    /// Observe the local or remote host CPU across the 4 layers and assemble
    /// a canonical `CpuReport`.
    fn observe_cpu(&self) -> Result<CpuReport, CpuObservationError>;
}
