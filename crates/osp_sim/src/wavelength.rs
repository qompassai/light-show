//! Wavelength-dependent behavior. Real single-mode fiber (ITU-T G.652 /
//! SMF-28-class) has different attenuation coefficients per operating
//! wavelength; PON systems additionally reserve specific wavelengths for
//! downstream video, downstream data, and upstream data.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Wavelength {
    /// 1310 nm — common O-band, PON upstream, historically first-window.
    Nm1310,
    /// 1490 nm — GPON downstream data.
    Nm1490,
    /// 1550 nm — C-band, GPON/RF video overlay downstream, long-haul.
    Nm1550,
}

impl Wavelength {
    /// Attenuation coefficient in dB/km for standard G.652 single-mode fiber.
    pub fn attenuation_db_per_km(&self) -> f64 {
        match self {
            Wavelength::Nm1310 => 0.35,
            Wavelength::Nm1490 => 0.28,
            Wavelength::Nm1550 => 0.21,
        }
    }

    /// Whether this wavelength is conventionally used for PON upstream
    /// traffic (affects which levels accept it as a valid choice).
    pub fn is_pon_upstream(&self) -> bool {
        matches!(self, Wavelength::Nm1310)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Wavelength::Nm1310 => "1310 nm (O-band / PON upstream)",
            Wavelength::Nm1490 => "1490 nm (GPON downstream data)",
            Wavelength::Nm1550 => "1550 nm (C-band / RF video overlay)",
        }
    }
}
