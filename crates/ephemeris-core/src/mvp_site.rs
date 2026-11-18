// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! House angles and solar rise/set using the shared explicit UT1/ΔT input.
//! Existing modern orientation guards still apply: this does not claim ancient
//! house or horizon coverage. No civil-time conversion is performed.

use crate::apparent_ecliptic_of_date::{
    geocentric_apparent_six_fields_many_with_options, ApparentError, ApparentFrame, ApparentOptions,
};
use crate::de441::{Body, De441};
use crate::explicit_delta_t::{
    ut1_to_tdb_with_explicit_delta_t, ExplicitDeltaTEpoch, ExplicitDeltaTError,
};
use crate::house_angles::{house_angles, HouseAngles, HouseAnglesError};
use crate::mvp_planets::PlanetaryTime;
use crate::rise_transit::{
    find_event, Atmosphere, Disc, Event, Limb, Request, RiseError, Site, SkyPosition,
};
use std::{
    error::Error,
    fmt,
    io::{Read, Seek},
};

#[derive(Debug)]
pub enum SolarPositionError {
    Time(ExplicitDeltaTError),
    Apparent(ApparentError),
}
impl fmt::Display for SolarPositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Time(e) => write!(f, "{e}"),
            Self::Apparent(e) => write!(f, "{e}"),
        }
    }
}
impl Error for SolarPositionError {}

#[derive(Debug)]
pub enum SiteError {
    InvalidSource,
    InvalidEvent,
    Time(ExplicitDeltaTError),
    House(HouseAnglesError),
    Solar(RiseError<SolarPositionError>),
}
impl fmt::Display for SiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSource => write!(f, "a Delta T source label (1..200 bytes) is required"),
            Self::InvalidEvent => write!(f, "expected sunrise or sunset"),
            Self::Time(e) => write!(f, "{e}"),
            Self::House(e) => write!(f, "{e}"),
            Self::Solar(e) => write!(f, "{e}"),
        }
    }
}
impl Error for SiteError {}

fn resolve(time: &PlanetaryTime<'_>) -> Result<ExplicitDeltaTEpoch, SiteError> {
    if time.source.trim().is_empty() || time.source.len() > 200 {
        return Err(SiteError::InvalidSource);
    }
    ut1_to_tdb_with_explicit_delta_t(time.jd_ut1, time.delta_t_seconds, time.uncertainty_seconds)
        .map_err(SiteError::Time)
}

#[derive(Debug)]
pub struct ExplicitHouseAngles {
    pub epoch: ExplicitDeltaTEpoch,
    pub delta_t_source: String,
    pub angles: HouseAngles,
}

/// The astronomical inputs to house methods; cusps remain in astrological-functions.
/// Modern house orientation limitations remain in force.
pub fn house_angles_ut1(
    time: PlanetaryTime<'_>,
    latitude_deg: f64,
    longitude_deg_east: f64,
) -> Result<ExplicitHouseAngles, SiteError> {
    let epoch = resolve(&time)?;
    let angles = house_angles(epoch.jd_ut1, epoch.jd_tt, latitude_deg, longitude_deg_east)
        .map_err(SiteError::House)?;
    Ok(ExplicitHouseAngles {
        epoch,
        delta_t_source: time.source.trim().to_owned(),
        angles,
    })
}

#[derive(Debug)]
pub struct SolarEvent {
    /// `None` when no crossing occurs in the requested window (e.g. polar day).
    pub jd_ut1: Option<f64>,
    pub delta_t_seconds: f64,
    pub uncertainty_seconds: f64,
    pub delta_t_source: String,
}

/// Search sunrise or sunset after the supplied UT1 epoch using the existing
/// topocentric, upper-limb and atmospheric-refraction convention. ΔT is held
/// constant across the requested search window. Modern frame/orientation
/// guards remain in effect; no long-range accuracy is implied.
pub fn solar_event_ut1<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    time: PlanetaryTime<'_>,
    event: Event,
    site: &Site,
    atmosphere: &Atmosphere,
    search_days: f64,
) -> Result<SolarEvent, SiteError> {
    if !matches!(event, Event::Rise | Event::Set) {
        return Err(SiteError::InvalidEvent);
    }
    let epoch = resolve(&time)?;
    let request = Request {
        event,
        disc: Disc::Radius {
            km: 695_700.0,
            limb: Limb::Upper,
        },
        refraction: true,
        topocentric: true,
        fixed_centre_altitude_deg: None,
        search_days,
    };
    let options = ApparentOptions {
        frame: ApparentFrame::TrueEquatorOfDate,
        ..ApparentOptions::default()
    };
    let result = find_event(
        |ut| {
            let current = ut1_to_tdb_with_explicit_delta_t(
                ut,
                time.delta_t_seconds,
                time.uncertainty_seconds,
            )
            .map_err(SolarPositionError::Time)?;
            let rows = geocentric_apparent_six_fields_many_with_options(
                ephemeris,
                &[Body::Sun],
                current.jd_tdb,
                options,
            )
            .map_err(SolarPositionError::Apparent)?;
            let fields = rows[0].fields;
            Ok::<_, SolarPositionError>(SkyPosition {
                tt_jd: current.jd_tt,
                right_ascension_deg: fields.longitude_deg,
                declination_deg: fields.latitude_deg,
                distance_au: fields.distance_au,
            })
        },
        site,
        atmosphere,
        &request,
        epoch.jd_ut1,
    )
    .map_err(SiteError::Solar)?;
    Ok(SolarEvent {
        jd_ut1: result,
        delta_t_seconds: time.delta_t_seconds,
        uncertainty_seconds: time.uncertainty_seconds,
        delta_t_source: time.source.trim().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn time(jd_ut1: f64) -> PlanetaryTime<'static> {
        PlanetaryTime {
            jd_ut1,
            delta_t_seconds: 64.0,
            uncertainty_seconds: 2.0,
            source: "synthetic",
        }
    }
    #[test]
    fn house_inputs_use_shared_ut1_tt_and_keep_modern_guard() {
        let result = house_angles_ut1(time(2_451_545.0), 41.9, 12.5).unwrap();
        let expected = house_angles(result.epoch.jd_ut1, result.epoch.jd_tt, 41.9, 12.5).unwrap();
        assert_eq!(result.angles, expected);
        assert_eq!(result.delta_t_source, "synthetic");
        assert!(matches!(
            house_angles_ut1(time(1_721_060.0), 41.9, 12.5),
            Err(SiteError::House(_))
        ));
        assert!(matches!(
            house_angles_ut1(time(2_451_545.0), 90.0, 12.5),
            Err(SiteError::House(_))
        ));
    }
}
