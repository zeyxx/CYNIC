//! Mathematical and golden-ratio constants governing CYNIC epistemic bounds.

pub const PHI: f64 = 1.618_033_988_749_895;
pub const PHI_INV: f64 = 0.618_033_988_749_895; // max confidence / crystallization gate
pub const PHI_INV2: f64 = 0.381_966_011_250_105; // anomaly threshold
pub const PHI_INV3: f64 = 0.236_067_977_499_790; // GROWL threshold
pub const PHI_INV4: f64 = 0.145_898_033_750_315; // convergence increment

pub const PHI_INV_BPS: u32 = 6180;
pub const PHI_INV_SQ_BPS: u32 = 3820;
pub const PHI_INV_CUBE_BPS: u32 = 2361;
