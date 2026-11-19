// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification (2026-11-18): expose long-term apparent equatorial directions.
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
    experimental_three_rate_candidate, experimental_three_rate_with_provider, ApparentError,
    ExperimentalLongTermStateProvider, ExperimentalProviderApparentError,
    ExperimentalThreeRateCandidate,
};
use crate::calendar::{Calendar, JulianDate, TimeScale};
use crate::calendar_decimal_hours::{julday_f64, DecimalHourDateTime};
use crate::de441::{Body, De441};
use crate::delta_t_long_range::espenak_meeus_2006_extended;
use crate::earth_orientation::{experimental_extrapolated_nutation, OrientationError};
use crate::explicit_delta_t::{
    ut1_to_tdb_with_explicit_delta_t, ExplicitDeltaTEpoch, ExplicitDeltaTError,
};
use crate::long_term_precession::experimental_direct_obliquity;

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
#[derive(Clone, Copy, Debug)]
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
    /// Geocentric apparent equatorial coordinates of date, degrees.
    pub equatorial: EquatorialPosition,
}

/// Equatorial direction of the same apparent ecliptic position. No rates are
/// supplied: the experimental ecliptic rates use a compatibility candidate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EquatorialPosition {
    pub right_ascension_deg: f64,
    pub declination_deg: f64,
}

#[derive(Debug)]
pub enum EquatorialError {
    InvalidDirection,
    UndefinedRightAscension,
    Orientation(OrientationError),
}
impl fmt::Display for EquatorialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDirection => write!(f, "invalid ecliptic direction"),
            Self::UndefinedRightAscension => write!(f, "right ascension is undefined at the pole"),
            Self::Orientation(error) => write!(f, "{error}"),
        }
    }
}
impl Error for EquatorialError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Orientation(error) => Some(error),
            Self::InvalidDirection | Self::UndefinedRightAscension => None,
        }
    }
}

/// Rotate an apparent ecliptic direction of date into the apparent equator of
/// the same date using the experimental long-term true obliquity. Input is TT.
pub fn experimental_equatorial_from_ecliptic(
    longitude_deg: f64,
    latitude_deg: f64,
    jd_tt: f64,
) -> Result<EquatorialPosition, EquatorialError> {
    if !longitude_deg.is_finite() || !latitude_deg.is_finite() || latitude_deg.abs() > 90.0 {
        return Err(EquatorialError::InvalidDirection);
    }
    let eps = experimental_direct_obliquity(jd_tt).map_err(EquatorialError::Orientation)?
        + experimental_extrapolated_nutation(jd_tt)
            .map_err(EquatorialError::Orientation)?
            .d_epsilon_rad;
    let (lon, lat) = (longitude_deg.to_radians(), latitude_deg.to_radians());
    let (x, y, z) = (lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin());
    let (y_eq, z_eq) = (y * eps.cos() - z * eps.sin(), y * eps.sin() + z * eps.cos());
    if x.hypot(y_eq) < 1e-14 {
        return Err(EquatorialError::UndefinedRightAscension);
    }
    let ra = y_eq.atan2(x).to_degrees().rem_euclid(360.0);
    Ok(EquatorialPosition {
        right_ascension_deg: if ra >= 360.0 { 0.0 } else { ra },
        declination_deg: z_eq.atan2(x.hypot(y_eq)).to_degrees(),
    })
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
    Orientation(EquatorialError),
    Calculation { body: Body, source: ApparentError },
}

impl fmt::Display for PlanetaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTargets => write!(f, "select 1–10 distinct supported planets"),
            Self::InvalidSource => write!(f, "a Delta T source label (1..200 bytes) is required"),
            Self::Time(error) => write!(f, "explicit time: {error}"),
            Self::Orientation(error) => write!(f, "equatorial orientation: {error}"),
            Self::Calculation { body, source } => write!(f, "{body:?}: {source}"),
        }
    }
}

impl Error for PlanetaryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Time(error) => Some(error),
            Self::Orientation(error) => Some(error),
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
        let equatorial = experimental_equatorial_from_ecliptic(
            rates.base.longitude_deg,
            rates.base.latitude_deg,
            epoch.jd_tt,
        )
        .map_err(PlanetaryError::Orientation)?;
        positions.push(PlanetaryPosition {
            body,
            rates,
            equatorial,
        });
    }
    Ok(PlanetarySet {
        epoch,
        delta_t_source: source.to_owned(),
        positions,
    })
}

/// Explicit UT1/ΔT planetary result using a per-epoch provider. The
/// experimental model and its limitations are the same as `planets_ut1`.
#[derive(Debug)]
pub enum PlanetaryProviderError<E> {
    Request(PlanetaryError),
    Calculation {
        body: Body,
        source: ExperimentalProviderApparentError<E>,
    },
}

pub fn planets_ut1_with_provider<P: ExperimentalLongTermStateProvider>(
    provider: &mut P,
    targets: &[Body],
    time: PlanetaryTime<'_>,
) -> Result<PlanetarySet, PlanetaryProviderError<P::Error>> {
    validate_targets(targets).map_err(PlanetaryProviderError::Request)?;
    let source = time.source.trim();
    if source.is_empty() || time.source.len() > 200 {
        return Err(PlanetaryProviderError::Request(
            PlanetaryError::InvalidSource,
        ));
    }
    let epoch = ut1_to_tdb_with_explicit_delta_t(
        time.jd_ut1,
        time.delta_t_seconds,
        time.uncertainty_seconds,
    )
    .map_err(|e| PlanetaryProviderError::Request(PlanetaryError::Time(e)))?;
    let mut positions = Vec::with_capacity(targets.len());
    for &body in targets {
        let rates = experimental_three_rate_with_provider(provider, body, epoch.jd_tdb)
            .map_err(|source| PlanetaryProviderError::Calculation { body, source })?;
        let equatorial = experimental_equatorial_from_ecliptic(
            rates.base.longitude_deg,
            rates.base.latitude_deg,
            epoch.jd_tt,
        )
        .map_err(|e| PlanetaryProviderError::Request(PlanetaryError::Orientation(e)))?;
        positions.push(PlanetaryPosition {
            body,
            rates,
            equatorial,
        });
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
    fn equatorial_rotation_preserves_equinox_and_solstice_directions() {
        let jd_tt = 2_451_545.0;
        let equinox = experimental_equatorial_from_ecliptic(0.0, 0.0, jd_tt).unwrap();
        assert_eq!(equinox.right_ascension_deg, 0.0);
        assert_eq!(equinox.declination_deg, 0.0);
        let north = experimental_equatorial_from_ecliptic(90.0, 0.0, jd_tt).unwrap();
        let south = experimental_equatorial_from_ecliptic(270.0, 0.0, jd_tt).unwrap();
        assert!((north.right_ascension_deg - 90.0).abs() < 1e-12);
        assert!((south.right_ascension_deg - 270.0).abs() < 1e-12);
        assert!((23.0..24.0).contains(&north.declination_deg));
        assert!((north.declination_deg + south.declination_deg).abs() < 1e-12);
        assert!(matches!(
            experimental_equatorial_from_ecliptic(0.0, 91.0, jd_tt),
            Err(EquatorialError::InvalidDirection)
        ));
        assert!(matches!(
            experimental_equatorial_from_ecliptic(f64::NAN, 0.0, jd_tt),
            Err(EquatorialError::InvalidDirection)
        ));
        assert!(matches!(
            experimental_equatorial_from_ecliptic(0.0, 0.0, f64::NAN),
            Err(EquatorialError::Orientation(_))
        ));
        let eps = experimental_direct_obliquity(jd_tt).unwrap()
            + experimental_extrapolated_nutation(jd_tt)
                .unwrap()
                .d_epsilon_rad;
        assert!(matches!(
            experimental_equatorial_from_ecliptic(90.0, 90.0 - eps.to_degrees(), jd_tt),
            Err(EquatorialError::UndefinedRightAscension)
        ));
    }

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
