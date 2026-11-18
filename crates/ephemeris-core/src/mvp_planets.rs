// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! One explicit UT1 instant to an ordered set of apparent planetary results.
//!
//! This packages the existing experimental long-range path; it does not infer
//! civil time, supply a kernel, or change the standard modern APIs. ΔT is either
//! supplied by the caller or estimated using an explicitly selected model.
//! Rates follow the Swiss-compatibility candidates and are not derivatives of
//! the returned positions. Mars through Pluto are system barycentres.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::apparent_ecliptic_of_date::{
    experimental_three_rate_candidate, ApparentError, ExperimentalThreeRateCandidate,
};
use crate::calendar::{Calendar, JulianDate, TimeScale};
use crate::calendar_decimal_hours::{julday_f64, DecimalHourDateTime};
use crate::de441::{Body, De441};
use crate::delta_t_long_range::espenak_meeus_2006_extended;
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

/// Invalid input or an instant outside the selected estimate's stated range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelTimeError {
    InvalidInput,
    OutOfRange,
}

impl fmt::Display for ModelTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => write!(f, "UT1 and a nonnegative uncertainty must be finite"),
            Self::OutOfRange => write!(f, "UT1 date is outside years -1999 through 3000"),
        }
    }
}
impl Error for ModelTimeError {}

impl PlanetaryTime<'static> {
    /// Opt in to the Espenak–Meeus 2006 estimate for proleptic Gregorian
    /// astronomical years −1999 through 3000. The caller still supplies the
    /// uncertainty: the estimate is not an observation or a guaranteed bound.
    pub fn espenak_meeus_2006(
        jd_ut1: f64,
        uncertainty_seconds: f64,
    ) -> Result<Self, ModelTimeError> {
        if !jd_ut1.is_finite() || !uncertainty_seconds.is_finite() || uncertainty_seconds < 0.0 {
            return Err(ModelTimeError::InvalidInput);
        }
        let date = JulianDate::from_f64(jd_ut1, TimeScale::Ut1)
            .and_then(|jd| jd.to_calendar(Calendar::ProlepticGregorian))
            .map_err(|_| ModelTimeError::OutOfRange)?;
        if !(-1999..=3000).contains(&date.year) {
            return Err(ModelTimeError::OutOfRange);
        }
        let start = |year| {
            julday_f64(
                DecimalHourDateTime {
                    year,
                    month: 1,
                    day: 1,
                    decimal_hour: 0.0,
                    scale: TimeScale::Ut1,
                },
                Calendar::ProlepticGregorian,
            )
            .map_err(|_| ModelTimeError::OutOfRange)
        };
        let begin = start(date.year)?;
        let end = start(date.year + 1)?;
        let decimal_year = f64::from(date.year) + (jd_ut1 - begin) / (end - begin);
        let delta_t_seconds =
            espenak_meeus_2006_extended(decimal_year).map_err(|_| ModelTimeError::OutOfRange)?;
        Ok(Self {
            jd_ut1,
            delta_t_seconds,
            uncertainty_seconds,
            source: "Espenak-Meeus 2006",
        })
    }
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

    #[test]
    fn model_time_is_opt_in_bounded_and_preserves_uncertainty() {
        let year = |y| {
            julday_f64(
                DecimalHourDateTime {
                    year: y,
                    month: 1,
                    day: 1,
                    decimal_hour: 0.0,
                    scale: TimeScale::Ut1,
                },
                Calendar::ProlepticGregorian,
            )
            .unwrap()
        };
        let y2000 = PlanetaryTime::espenak_meeus_2006(year(2000), 3.0).unwrap();
        assert!((y2000.delta_t_seconds - 63.86).abs() < 0.01);
        assert_eq!(y2000.uncertainty_seconds, 3.0);
        assert_eq!(y2000.source, "Espenak-Meeus 2006");
        assert!(PlanetaryTime::espenak_meeus_2006(year(-1999), 0.0).is_ok());
        assert!(PlanetaryTime::espenak_meeus_2006(year(3000), 0.0).is_ok());
        assert!(matches!(
            PlanetaryTime::espenak_meeus_2006(year(-2000), 1.0),
            Err(ModelTimeError::OutOfRange)
        ));
        assert!(matches!(
            PlanetaryTime::espenak_meeus_2006(year(3001), 1.0),
            Err(ModelTimeError::OutOfRange)
        ));
        assert!(matches!(
            PlanetaryTime::espenak_meeus_2006(year(2000), -1.0),
            Err(ModelTimeError::InvalidInput)
        ));
    }
}
