// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Opt-in long-term lunar points at the same explicit UT1/ΔT as planets.

use crate::apparent_ecliptic_of_date::ApparentFrame;
use crate::de441::De441;
use crate::explicit_delta_t::{
    ut1_to_tdb_with_explicit_delta_t, ExplicitDeltaTEpoch, ExplicitDeltaTError,
};
use crate::lunar_points::{
    experimental_long_term_lunar_point_with_provider, LunarPoint, LunarPointError, LunarPointState,
    LunarStateProvider,
};
use crate::mvp_planets::PlanetaryTime;
use std::{
    error::Error,
    fmt,
    io::{Read, Seek},
};

/// Ascending nodes and apogees supported by the existing lunar-point model.
pub const MVP_LUNAR_POINTS: [LunarPoint; 4] = [
    LunarPoint::MeanNode,
    LunarPoint::TrueNode,
    LunarPoint::MeanApogee,
    LunarPoint::OsculatingApogee,
];

#[derive(Debug)]
pub struct LunarPosition {
    pub point: LunarPoint,
    pub ecliptic: LunarPointState,
    pub equatorial: LunarPointState,
}

#[derive(Debug)]
pub struct LunarSet {
    pub epoch: ExplicitDeltaTEpoch,
    pub delta_t_source: String,
    pub positions: Vec<LunarPosition>,
}

#[derive(Debug)]
pub enum LunarSetError {
    InvalidTargets,
    InvalidSource,
    Time(ExplicitDeltaTError),
    Calculation {
        point: LunarPoint,
        source: LunarPointError,
    },
}
impl fmt::Display for LunarSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTargets => write!(f, "select 1–4 distinct supported lunar points"),
            Self::InvalidSource => write!(f, "a Delta T source label (1..200 bytes) is required"),
            Self::Time(error) => write!(f, "explicit time: {error}"),
            Self::Calculation { point, source } => write!(f, "{point:?}: {source}"),
        }
    }
}
impl Error for LunarSetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Time(e) => Some(e),
            Self::Calculation { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Calculate ecliptic and equatorial lunar points at the caller's UT1/ΔT.
/// This is experimental: distant-date mean elements, orientation and rates
/// have not been independently validated. No partial set is returned.
pub fn lunar_points_ut1<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    points: &[LunarPoint],
    time: PlanetaryTime<'_>,
) -> Result<LunarSet, LunarSetError> {
    lunar_points_ut1_with_provider(ephemeris, points, time)
}

/// The same explicit UT1/ΔT calculation with per-sample provider selection.
pub fn lunar_points_ut1_with_provider<P: LunarStateProvider + ?Sized>(
    ephemeris: &mut P,
    points: &[LunarPoint],
    time: PlanetaryTime<'_>,
) -> Result<LunarSet, LunarSetError> {
    if points.is_empty()
        || points.len() > MVP_LUNAR_POINTS.len()
        || points
            .iter()
            .enumerate()
            .any(|(i, point)| !MVP_LUNAR_POINTS.contains(point) || points[..i].contains(point))
    {
        return Err(LunarSetError::InvalidTargets);
    }
    let source = time.source.trim();
    if source.is_empty() || time.source.len() > 200 {
        return Err(LunarSetError::InvalidSource);
    }
    let epoch = ut1_to_tdb_with_explicit_delta_t(
        time.jd_ut1,
        time.delta_t_seconds,
        time.uncertainty_seconds,
    )
    .map_err(LunarSetError::Time)?;
    let mut positions = Vec::with_capacity(points.len());
    for &point in points {
        let calculate = |eph: &mut P, frame| {
            experimental_long_term_lunar_point_with_provider(eph, point, frame, epoch.jd_tdb.0)
                .map_err(|source| LunarSetError::Calculation { point, source })
        };
        positions.push(LunarPosition {
            point,
            ecliptic: calculate(ephemeris, ApparentFrame::TrueEclipticOfDate)?,
            equatorial: calculate(ephemeris, ApparentFrame::TrueEquatorOfDate)?,
        });
    }
    Ok(LunarSet {
        epoch,
        delta_t_source: source.to_owned(),
        positions,
    })
}
