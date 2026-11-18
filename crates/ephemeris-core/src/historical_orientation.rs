// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Experimental long-term sidereal and house orientation, with explicit UT1/TT.
//!
//! The mean Sun is constructed from the Simon et al. (1994), section 5.8.3,
//! Earth mean longitude referred to J2000, then projected onto the Vondrák et
//! al. (2011) mean ecliptic of date. The UT1 fraction supplies the local solar
//! hour angle. A fixed 1 AU light time and the existing experimental nutation
//! supply approximate apparent corrections. This extrapolates the secular mean
//! elements far outside their fit interval and is not a physical accuracy
//! guarantee. It is separate from the guarded modern orientation.
//!
//! References: https://articles.adsabs.harvard.edu/pdf/1994A%26A...282..663S
//! and https://doi.org/10.1051/0004-6361/201117274.

use crate::earth_orientation::{
    experimental_extrapolated_nutation, matrix_vector_multiply, OrientationError,
};
use crate::house_angles::{
    angles_from_armc, extra_angles_from_armc, HouseAngles, HouseAnglesError,
};
use crate::long_term_precession::{experimental_direct_obliquity, mean_date_matrices};
use std::f64::consts::{PI, TAU};

const J2000: f64 = 2_451_545.0;
const AU_LIGHT_DAYS: f64 = 0.005_775_518_3;

/// Approximate Greenwich apparent sidereal angle in radians, [0, 2π).
/// UT1 and TT are explicit and may differ by a caller-selected ΔT. The long-term
/// mean solar elements and nutation are extrapolations, not validated accuracy.
pub fn experimental_gast(ut1_jd: f64, tt_jd: f64) -> Result<f64, OrientationError> {
    if !ut1_jd.is_finite() {
        return Err(OrientationError::NonFinite { field: "ut1_jd" });
    }
    let (equator, _) = mean_date_matrices(tt_jd)?;
    let eps = experimental_direct_obliquity(tt_jd)?;
    let nutation = experimental_extrapolated_nutation(tt_jd)?;
    let t = (tt_jd - J2000 - AU_LIGHT_DAYS) / 365_250.0;
    let mean_sun = (280.466_456_83
        + (1_295_977_422.834_29 * t - 2.044_11 * t * t - 0.005_23 * t * t * t) / 3600.0)
        .to_radians();
    let eps0 = experimental_direct_obliquity(J2000)?;
    let ecliptic_j2000 = [
        mean_sun.cos(),
        mean_sun.sin() * eps0.cos(),
        mean_sun.sin() * eps0.sin(),
    ];
    let mean_equator = matrix_vector_multiply(equator, ecliptic_j2000);
    // Use the same direct-obliquity realization as the historical planet path.
    let ecliptic_x = mean_equator[0];
    let ecliptic_y = mean_equator[1] * eps.cos() + mean_equator[2] * eps.sin();
    let longitude = ecliptic_y.atan2(ecliptic_x);
    let ut_fraction = (ut1_jd + 0.5).rem_euclid(1.0);
    Ok((longitude + ut_fraction * TAU - PI + nutation.d_psi_rad * eps.cos()).rem_euclid(TAU))
}

/// Experimental house geometry at an explicit UT1/TT and east-positive site.
/// House cusps themselves remain the responsibility of astrological-functions.
pub fn experimental_house_angles(
    ut1_jd: f64,
    tt_jd: f64,
    latitude_deg: f64,
    longitude_deg_east: f64,
) -> Result<HouseAngles, HouseAnglesError> {
    if !latitude_deg.is_finite()
        || latitude_deg.abs() >= 90.0
        || !longitude_deg_east.is_finite()
        || longitude_deg_east.abs() > 360.0
    {
        return Err(HouseAnglesError::InvalidSite);
    }
    let armc =
        (experimental_gast(ut1_jd, tt_jd)?.to_degrees() + longitude_deg_east).rem_euclid(360.0);
    let eps = experimental_direct_obliquity(tt_jd)?
        + experimental_extrapolated_nutation(tt_jd)?.d_epsilon_rad;
    let obliquity = eps.to_degrees();
    let (ascendant, midheaven) = angles_from_armc(armc, obliquity, latitude_deg);
    Ok(HouseAngles {
        ut1_jd,
        tt_jd,
        latitude_deg,
        longitude_deg_east,
        armc_deg: armc,
        true_obliquity_deg: obliquity,
        ascendant_deg: ascendant,
        midheaven_deg: midheaven,
        extra: extra_angles_from_armc(armc, obliquity, latitude_deg, midheaven),
    })
}
