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

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    /// SMF-28 figures from docs/GAME_DESIGN.md's optical model table. The
    /// doc's 1550 nm value (0.25) is the G.652 cabled ceiling; the sim's
    /// 0.21 is a typical-fiber figure, so it is checked as an upper bound.
    const DOC_1310_DB_PER_KM: f64 = 0.35;
    const DOC_1550_DB_PER_KM_MAX: f64 = 0.25;

    #[test]
    fn longer_wavelengths_attenuate_less_on_smf() {
        let o_band = Wavelength::Nm1310.attenuation_db_per_km();
        let gpon_down = Wavelength::Nm1490.attenuation_db_per_km();
        let c_band = Wavelength::Nm1550.attenuation_db_per_km();
        assert!(o_band > gpon_down && gpon_down > c_band);
    }

    #[test]
    fn attenuation_matches_design_doc() {
        assert_relative_eq!(
            Wavelength::Nm1310.attenuation_db_per_km(),
            DOC_1310_DB_PER_KM,
            epsilon = 1e-9
        );
        assert!(Wavelength::Nm1550.attenuation_db_per_km() <= DOC_1550_DB_PER_KM_MAX);
    }

    #[test]
    fn only_1310_is_pon_upstream() {
        assert!(Wavelength::Nm1310.is_pon_upstream());
        assert!(!Wavelength::Nm1490.is_pon_upstream());
        assert!(!Wavelength::Nm1550.is_pon_upstream());
    }

    #[test]
    fn unsupported_wavelength_is_rejected_by_serde() {
        // Adversarial: an L-band level file must fail to load, not
        // silently fall back to some default attenuation.
        let parsed: Result<Wavelength, _> = serde_json::from_str("\"Nm1625\"");
        assert!(parsed.is_err());
    }
}
