// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification 2026-11-17: explicit caller-provided UT1 to TT/TDB conversion.
// Modification 2026-11-18: convert three explicit Delta T scenarios safely.
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

/// Three sampled conversions at a fixed UT1, not an uncertainty interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExplicitDeltaTScenarios {
    pub lower: ExplicitDeltaTEpoch,
    pub nominal: ExplicitDeltaTEpoch,
    pub upper: ExplicitDeltaTEpoch,
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

/// Return the leading periodic TDB−TT term in seconds before Julian-date
/// rounding. This is an approximation, not a full relativistic time model.
pub fn approximate_tdb_minus_tt_seconds(jd_tt: f64) -> Result<f64, ExplicitDeltaTError> {
    if !jd_tt.is_finite() {
        return Err(ExplicitDeltaTError::NonFinite);
    }
    let t = (jd_tt - 2_451_545.0) / 36_525.0;
    let mean_anomaly = (357.5291092 + 35_999.0502909 * t).to_radians();
    let correction = 0.001657 * mean_anomaly.sin();
    if !mean_anomaly.is_finite() || !correction.is_finite() {
        return Err(ExplicitDeltaTError::Unrepresentable);
    }
    Ok(correction)
}

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
    if !jd_tt.is_finite() {
        return Err(ExplicitDeltaTError::Unrepresentable);
    }
    let correction = approximate_tdb_minus_tt_seconds(jd_tt)?;
    let jd_tdb = jd_tt + correction / 86_400.0;
    if !jd_tdb.is_finite() {
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

/// Convert caller-provided Delta T and its plus/minus perturbations at a fixed
/// UT1. A nonzero perturbation must remain distinguishable in Delta T, TT and
/// TDB f64 values. The three samples are not bounds on intermediate values.
pub fn ut1_to_tdb_delta_t_scenarios(
    jd_ut1: f64,
    delta_t_seconds: f64,
    uncertainty_seconds: f64,
) -> Result<ExplicitDeltaTScenarios, ExplicitDeltaTError> {
    let nominal = ut1_to_tdb_with_explicit_delta_t(jd_ut1, delta_t_seconds, uncertainty_seconds)?;
    let lower_delta = delta_t_seconds - uncertainty_seconds;
    let upper_delta = delta_t_seconds + uncertainty_seconds;
    if !lower_delta.is_finite() || !upper_delta.is_finite() {
        return Err(ExplicitDeltaTError::Unrepresentable);
    }
    let lower = ut1_to_tdb_with_explicit_delta_t(jd_ut1, lower_delta, uncertainty_seconds)?;
    let upper = ut1_to_tdb_with_explicit_delta_t(jd_ut1, upper_delta, uncertainty_seconds)?;
    if uncertainty_seconds > 0.0
        && !(lower_delta < delta_t_seconds
            && delta_t_seconds < upper_delta
            && lower.jd_tt < nominal.jd_tt
            && nominal.jd_tt < upper.jd_tt
            && lower.jd_tdb.0 < nominal.jd_tdb.0
            && nominal.jd_tdb.0 < upper.jd_tdb.0)
    {
        return Err(ExplicitDeltaTError::Unrepresentable);
    }
    Ok(ExplicitDeltaTScenarios {
        lower,
        nominal,
        upper,
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

    #[test]
    fn scenarios_preserve_fixed_ut1_and_reject_collapsed_samples() {
        let set = ut1_to_tdb_delta_t_scenarios(2_451_545.0, 64.0, 10.0).unwrap();
        assert_eq!(set.lower.delta_t_seconds, 54.0);
        assert_eq!(set.nominal.delta_t_seconds, 64.0);
        assert_eq!(set.upper.delta_t_seconds, 74.0);
        assert_eq!(set.lower.jd_ut1, set.upper.jd_ut1);
        assert!(set.lower.jd_tt < set.nominal.jd_tt && set.nominal.jd_tt < set.upper.jd_tt);
        let zero = ut1_to_tdb_delta_t_scenarios(2_451_545.0, 64.0, 0.0).unwrap();
        assert_eq!(zero.lower, zero.nominal);
        assert_eq!(zero.upper, zero.nominal);
        assert_eq!(
            ut1_to_tdb_delta_t_scenarios(2_451_545.0, 64.0, -1.0),
            Err(ExplicitDeltaTError::NegativeUncertainty)
        );
        assert_eq!(
            ut1_to_tdb_delta_t_scenarios(2_451_545.0, 64.0, 1e-12),
            Err(ExplicitDeltaTError::Unrepresentable)
        );
        assert_eq!(
            ut1_to_tdb_delta_t_scenarios(2_451_545.0, f64::MAX, f64::MAX),
            Err(ExplicitDeltaTError::Unrepresentable)
        );
    }
}
