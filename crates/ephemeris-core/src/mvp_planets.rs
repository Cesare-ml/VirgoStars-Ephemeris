// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! One explicit UT1 instant to an ordered set of apparent planetary results.
//!
//! This packages the existing experimental long-range path; it does not infer
//! civil time or ΔT, supply a kernel, or change the standard modern APIs.
//! Rates follow the Swiss-compatibility candidates and are not derivatives of
//! the returned positions. Mars through Pluto are system barycentres.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::apparent_ecliptic_of_date::{
    experimental_three_rate_candidate, ApparentError, ExperimentalThreeRateCandidate,
};
use crate::de441::{Body, De441};
use crate::explicit_delta_t::{
    ut1_to_tdb_with_explicit_delta_t, ExplicitDeltaTEpoch, ExplicitDeltaTError,
};

/// The complete planetary set supported by this API, in display order.
pub const MVP_PLANETS: [Body; 10] = [
    Body::Sun,
    Body::Moon,
    Body::Mercury,
    Body::Venus,
    Body::MarsBarycenter,
    Body::JupiterBarycenter,
    Body::SaturnBarycenter,
    Body::UranusBarycenter,
    Body::NeptuneBarycenter,
    Body::PlutoBarycenter,
];

/// Caller-provided UT1 and ΔT. Uncertainty is an assertion, not a position bound.
pub struct PlanetaryTime<'a> {
    pub jd_ut1: f64,
    pub delta_t_seconds: f64,
    pub uncertainty_seconds: f64,
    pub source: &'a str,
}

#[derive(Clone, Debug)]
pub struct PlanetaryPosition {
    pub body: Body,
    pub rates: ExperimentalThreeRateCandidate,
}

#[derive(Clone, Debug)]
pub struct PlanetarySet {
    pub epoch: ExplicitDeltaTEpoch,
    pub delta_t_source: String,
    pub positions: Vec<PlanetaryPosition>,
}

#[derive(Debug)]
pub enum PlanetaryError {
    InvalidTargets,
    InvalidSource,
    Time(ExplicitDeltaTError),
    Calculation { body: Body, source: ApparentError },
}

impl fmt::Display for PlanetaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTargets => write!(f, "select 1–10 distinct supported planets"),
            Self::InvalidSource => write!(f, "a Delta T source label (1..200 bytes) is required"),
            Self::Time(error) => write!(f, "explicit time: {error}"),
            Self::Calculation { body, source } => write!(f, "{body:?}: {source}"),
        }
    }
}

impl Error for PlanetaryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Time(error) => Some(error),
            Self::Calculation { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn validate_targets(targets: &[Body]) -> Result<(), PlanetaryError> {
    if targets.is_empty()
        || targets.len() > MVP_PLANETS.len()
        || targets
            .iter()
            .enumerate()
            .any(|(i, body)| !MVP_PLANETS.contains(body) || targets[..i].contains(body))
    {
        return Err(PlanetaryError::InvalidTargets);
    }
    Ok(())
}

/// Calculate one or more distinct supported bodies in the requested order.
/// Fails with the body that could not be calculated; never returns a partial set.
/// Input UT1 is neither UTC nor local time. DE441 coverage is controlled by the
/// caller's file and the path remains experimental outside modern epochs.
pub fn planets_ut1<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    targets: &[Body],
    time: PlanetaryTime<'_>,
) -> Result<PlanetarySet, PlanetaryError> {
    validate_targets(targets)?;
    let source = time.source.trim();
    if source.is_empty() || time.source.len() > 200 {
        return Err(PlanetaryError::InvalidSource);
    }
    let epoch = ut1_to_tdb_with_explicit_delta_t(
        time.jd_ut1,
        time.delta_t_seconds,
        time.uncertainty_seconds,
    )
    .map_err(PlanetaryError::Time)?;
    let mut positions = Vec::with_capacity(targets.len());
    for &body in targets {
        let rates = experimental_three_rate_candidate(ephemeris, body, epoch.jd_tdb)
            .map_err(|source| PlanetaryError::Calculation { body, source })?;
        positions.push(PlanetaryPosition { body, rates });
    }
    Ok(PlanetarySet {
        epoch,
        delta_t_source: source.to_owned(),
        positions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_planet_selection() {
        assert!(validate_targets(&MVP_PLANETS).is_ok());
        assert!(validate_targets(&[]).is_err());
        assert!(validate_targets(&[Body::Sun, Body::Sun]).is_err());
        assert!(validate_targets(&[Body::Earth]).is_err());
    }
}
