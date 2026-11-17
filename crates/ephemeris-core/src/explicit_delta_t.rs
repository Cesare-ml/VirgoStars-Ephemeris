// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification 2026-11-17: explicit caller-provided UT1 to TT/TDB conversion.
//! No Delta T model is assumed. Input is UT1, not UTC or a local civil time.
//! Caller supplies TT-UT1 and its uncertainty for this individual epoch.

use crate::de441::JulianDateTdb;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExplicitDeltaTEpoch {
    pub jd_ut1: f64,
    pub jd_tt: f64,
    pub jd_tdb: JulianDateTdb,
    pub delta_t_seconds: f64,
    pub uncertainty_seconds: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplicitDeltaTError {
    NonFinite,
    NegativeUncertainty,
    Unrepresentable,
}
impl fmt::Display for ExplicitDeltaTError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::NonFinite => "UT1, Delta T and uncertainty must be finite",
                Self::NegativeUncertainty => "Delta T uncertainty must be nonnegative",
                Self::Unrepresentable => "converted epoch is not representable",
            }
        )
    }
}
impl Error for ExplicitDeltaTError {}

/// Convert UT1 to TT and approximate TDB. The approximation is the same leading
/// periodic TDB-TT term used by the existing apparent pipeline; no accuracy
/// claim for that approximation over distant epochs is made. Uncertainty is a
/// caller assertion and is not converted to an angular error.
pub fn ut1_to_tdb_with_explicit_delta_t(
    jd_ut1: f64,
    delta_t_seconds: f64,
    uncertainty_seconds: f64,
) -> Result<ExplicitDeltaTEpoch, ExplicitDeltaTError> {
    if ![jd_ut1, delta_t_seconds, uncertainty_seconds]
        .iter()
        .all(|x| x.is_finite())
    {
        return Err(ExplicitDeltaTError::NonFinite);
    }
    if uncertainty_seconds < 0.0 {
        return Err(ExplicitDeltaTError::NegativeUncertainty);
    }
    let jd_tt = jd_ut1 + delta_t_seconds / 86_400.0;
    let t = (jd_tt - 2_451_545.0) / 36_525.0;
    let mean_anomaly = (357.5291092 + 35_999.0502909 * t).to_radians();
    let jd_tdb = jd_tt + 0.001657 * mean_anomaly.sin() / 86_400.0;
    if ![jd_tt, mean_anomaly, jd_tdb].iter().all(|x| x.is_finite()) {
        return Err(ExplicitDeltaTError::Unrepresentable);
    }
    Ok(ExplicitDeltaTEpoch {
        jd_ut1,
        jd_tt,
        jd_tdb: JulianDateTdb(jd_tdb),
        delta_t_seconds,
        uncertainty_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn applies_caller_delta_t_and_retains_uncertainty() {
        let epoch = ut1_to_tdb_with_explicit_delta_t(2_451_545.0, 64.184, 0.5).unwrap();
        assert!(((epoch.jd_tt - epoch.jd_ut1) * 86_400.0 - 64.184).abs() < 0.0001);
        assert!((epoch.jd_tdb.0 - epoch.jd_tt).abs() * 86_400.0 < 0.002);
        assert_eq!(epoch.uncertainty_seconds, 0.5);
    }
    #[test]
    fn rejects_invalid_and_overflowed_inputs() {
        assert_eq!(
            ut1_to_tdb_with_explicit_delta_t(f64::NAN, 1.0, 1.0),
            Err(ExplicitDeltaTError::NonFinite)
        );
        assert_eq!(
            ut1_to_tdb_with_explicit_delta_t(1.0, f64::INFINITY, 1.0),
            Err(ExplicitDeltaTError::NonFinite)
        );
        assert_eq!(
            ut1_to_tdb_with_explicit_delta_t(1.0, 1.0, -1.0),
            Err(ExplicitDeltaTError::NegativeUncertainty)
        );
        assert_eq!(
            ut1_to_tdb_with_explicit_delta_t(f64::MAX, f64::MAX, 1.0),
            Err(ExplicitDeltaTError::Unrepresentable)
        );
    }
}
