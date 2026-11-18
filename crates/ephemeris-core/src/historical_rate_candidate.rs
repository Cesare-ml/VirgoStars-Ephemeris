// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Explicit UT1 and caller-supplied ΔT into the experimental three-rate path.
//! UT1 is not UTC or a civil time; no ΔT model or accuracy claim is implicit.
use crate::apparent_ecliptic_of_date::{
    experimental_three_rate_candidate, ApparentError, ExperimentalThreeRateCandidate,
};
use crate::de441::{Body, De441};
use crate::explicit_delta_t::{
    ut1_to_tdb_with_explicit_delta_t, ExplicitDeltaTEpoch, ExplicitDeltaTError,
};
use std::{
    error::Error,
    fmt,
    io::{Read, Seek},
};

/// The supplied uncertainty is retained as an assertion, not a position bound.
#[derive(Clone, Debug)]
pub struct HistoricalThreeRateCandidate {
    pub epoch: ExplicitDeltaTEpoch,
    pub rates: ExperimentalThreeRateCandidate,
    pub delta_t_source: String,
}

#[derive(Debug)]
pub enum HistoricalRateCandidateError {
    InvalidSource,
    Time(ExplicitDeltaTError),
    Calculation(ApparentError),
}
impl fmt::Display for HistoricalRateCandidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSource => write!(f, "a Delta T source label (1..200 bytes) is required"),
            Self::Time(e) => write!(f, "explicit time: {e}"),
            Self::Calculation(e) => write!(f, "experimental rates: {e}"),
        }
    }
}
impl Error for HistoricalRateCandidateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidSource => None,
            Self::Time(e) => Some(e),
            Self::Calculation(e) => Some(e),
        }
    }
}

/// Use caller-supplied UT1, TT−UT1 and its uncertainty with the three-rate
/// diagnostic. All rates are per TDB day and candidates are not derivatives.
/// The caller must provide a nonempty source label; none is selected or inferred.
pub fn historical_three_rate_candidate<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    jd_ut1: f64,
    delta_t_seconds: f64,
    uncertainty_seconds: f64,
    delta_t_source: &str,
) -> Result<HistoricalThreeRateCandidate, HistoricalRateCandidateError> {
    let source = validate_delta_t_source(delta_t_source)?;
    let epoch = ut1_to_tdb_with_explicit_delta_t(jd_ut1, delta_t_seconds, uncertainty_seconds)
        .map_err(HistoricalRateCandidateError::Time)?;
    let rates = experimental_three_rate_candidate(ephemeris, target, epoch.jd_tdb)
        .map_err(HistoricalRateCandidateError::Calculation)?;
    Ok(HistoricalThreeRateCandidate {
        epoch,
        rates,
        delta_t_source: source.to_owned(),
    })
}

fn validate_delta_t_source(source: &str) -> Result<&str, HistoricalRateCandidateError> {
    if source.trim().is_empty() || source.len() > 200 {
        return Err(HistoricalRateCandidateError::InvalidSource);
    }
    Ok(source.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_must_be_explicit_and_within_the_byte_limit() {
        assert_eq!(validate_delta_t_source(" example ").unwrap(), "example");
        assert!(validate_delta_t_source("  ").is_err());
        assert!(validate_delta_t_source(&"è".repeat(101)).is_err());
    }
}
