// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Isolated apparent ICRF experiment using the analytic solar-deflection rate.
//! TDB input; light-time, solar deflection and annual aberration. No orientation,
//! civil time or emission-JD refinement. Accuracy, especially near the Sun, is
//! not established. This path is never selected by standard calculations.
use crate::aberration_state::{
    aberrate_light_time_state, AberratedGeocentricState, AberratedStateError,
    ObserverAccelerationSource,
};
use crate::de441::{Body, De441, JulianDateTdb};
use crate::light_deflection::{experimental_analytic_deflect_sun, AnalyticDeflectionError};
use crate::light_time::geocentric_light_time;
use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

#[derive(Debug)]
pub enum ExperimentalAnalyticApparentError {
    Base(AberratedStateError),
    Deflection(AnalyticDeflectionError),
}
impl fmt::Display for ExperimentalAnalyticApparentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Base(e) => write!(f, "{e}"),
            Self::Deflection(e) => write!(f, "{e}"),
        }
    }
}
impl Error for ExperimentalAnalyticApparentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Base(e) => Some(e),
            Self::Deflection(e) => Some(e),
        }
    }
}

/// Apparent Earth-centered ICRF direction in km and velocity in km/TDB day.
/// Uses the analytic deflection-rate experiment and unrefined emission JD.
/// A floor-boundary error is explicit; no fallback is performed.
pub fn experimental_analytic_apparent_icrf<R: Read + Seek>(
    eph: &mut De441<R>,
    target: Body,
    epoch: JulianDateTdb,
) -> Result<AberratedGeocentricState, ExperimentalAnalyticApparentError> {
    let earth = eph
        .state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, epoch)
        .map_err(|e| ExperimentalAnalyticApparentError::Base(e.into()))?;
    let lt = geocentric_light_time(eph, target, epoch)
        .map_err(|e| ExperimentalAnalyticApparentError::Base(e.into()))?;
    let deflected = experimental_analytic_deflect_sun(eph, &lt)
        .map_err(ExperimentalAnalyticApparentError::Deflection)?;
    let mut result = aberrate_light_time_state(
        &deflected,
        earth.velocity_km_per_day,
        earth.acceleration_km_per_day2,
    )
    .map_err(ExperimentalAnalyticApparentError::Base)?;
    result.observer_acceleration_source = ObserverAccelerationSource::AnalyticDe441Chebyshev;
    Ok(result)
}
