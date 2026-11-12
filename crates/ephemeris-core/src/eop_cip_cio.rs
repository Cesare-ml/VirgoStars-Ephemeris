// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

//! Apply observed/predicted celestial pole offsets dX/dY from an
//! externally provided CelesTrak EOP CSV to an independently evaluated
//! IAU 2006/2000A model at the **same TT epoch**.
//!
//! The caller MUST supply a correctly evaluated model X,Y and the
//! date-dependent CIO series quantity q = s + X*Y/2 in radians.
//! After correcting the CIP, s = q - (X+dX)*(Y+dY)/2. This matches
//! the conventional representation of SOFA/ERFA s06 and avoids
//! incorrectly reusing uncorrected s with corrected X/Y.
//!
//! No precession/nutation series, source authentication, external
//! downloads, or physical IERS realization validation are implied.
//! Original VirgoStars implementation, 2026-11-12.

use std::error::Error;
use std::fmt;

use crate::calendar::{JulianDate, TimeScale};
use crate::eop_observer_gcrs::{
    wgs84_gcrs_from_eop_utc_and_cip_cio, EopWgs84GcrsError,
    EopWgs84GcrsObserver, ExternalCipCioAtTt,
};
use crate::eop_polar_csv::{
    CelestrakEopPolarCatalog, EopCsvError, EopPolarQuality, EopPredictionPolicy,
};
use crate::eop_ut1_era::{eop_ut1_era_from_utc_si_seconds, EopUt1EraError};
use crate::observer_gcrs::Wgs84ObserverSite;

/// Externally evaluated, nominal **IAU 2006/2000A** CIP/CIO model at
/// an explicit TT epoch. The source label is an audit identifier,
/// not proof that the inputs were really produced by that model.
///
/// q = s + X*Y/2 is the date-dependent CIO series quantity; caller
/// may compute it from a validated same-TT model's (X,Y,s).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModeledCipCioAtTt<'a> {
    pub tt_epoch: JulianDate,
    pub model_x: f64,
    pub model_y: f64,
    pub cio_s_plus_xy_half_rad: f64,
    pub source_label: &'a str,
}

/// Corrected CIP/CIO quantities with original model and EOP offsets
/// available for science review. All are in radians except the
/// dimensionless CIP X and Y unit-vector components.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EopCorrectedCipCio {
    pub model_x: f64,
    pub model_y: f64,
    pub eop_dx_rad: f64,
    pub eop_dy_rad: f64,
    pub corrected_x: f64,
    pub corrected_y: f64,
    pub corrected_cio_s_rad: f64,
    pub quality: EopPolarQuality,
}

/// Geocentric observer position derived using the corrected pole
/// with EOP and celestial-model provenance kept distinct.
#[derive(Clone, Debug, PartialEq)]
pub struct EopCorrectedWgs84GcrsObserver {
    pub corrected_cip_cio: EopCorrectedCipCio,
    pub observer: EopWgs84GcrsObserver,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EopCipCioError {
    MissingModelSource,
    ModelEpochRequiresTt(TimeScale),
    ModelEpochMismatch,
    InvalidModeledCoordinates,
    InvalidCorrectedCoordinates,
    Eop(EopUt1EraError),
    Offsets(EopCsvError),
    Observer(EopWgs84GcrsError),
}

impl fmt::Display for EopCipCioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingModelSource => write!(f, "CIP/CIO model source label is missing"),
            Self::ModelEpochRequiresTt(scale) => {
                write!(f, "CIP/CIO source must be TT, got {scale:?}")
            }
            Self::ModelEpochMismatch => write!(f, "CIP/CIO model TT epoch differs from EOP-derived TT"),
            Self::InvalidModeledCoordinates => write!(f, "invalid external model X, Y or s+XY/2"),
            Self::InvalidCorrectedCoordinates => write!(f, "invalid EOP-corrected CIP/CIO quantities"),
            Self::Eop(error) => write!(f, "EOP UTC/UT1/TT: {error}"),
            Self::Offsets(error) => write!(f, "EOP celestial pole offsets: {error}"),
            Self::Observer(error) => write!(f, "EOP-adjusted WGS84 GCRS observer: {error}"),
        }
    }
}

impl Error for EopCipCioError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Eop(error) => Some(error),
            Self::Offsets(error) => Some(error),
            Self::Observer(error) => Some(error),
            _ => None,
        }
    }
}

/// Construct a GCRS site from a **separately evaluated IAU 2006/2000A
/// model** corrected by the matching CelesTrak dX/dY measurements.
///
/// Calls the existing leap-SI-second-aware EOP/UT1/TT engine first.
/// Requires the model's exact tagged TT instant and a nonblank
/// externally accountable source. Reuses the EOP CSV's same UTC-day
/// interpolation and forecast opt-in, without extracting another
/// set of timestamps. The EOP correction convention is
/// dX = X_observed - X_model, dY = Y_observed - Y_model; therefore
///
/// X_corr = X_model + dX, Y_corr = Y_model + dY
/// s_corr = q_model(TT) - X_corr*Y_corr/2
///
/// where q_model is the caller's independently evaluated
/// date-dependent IAU CIO series s+X*Y/2. This **does not implement**
/// q_model or the precession/nutation series. It does not imply that
/// a source label authenticates the model, EOP, leap schedule or
/// compatible reference frame. The ITRS/GCRS result is position
/// only, and not a DE441 BCRS state or an apparent sky position.
pub fn wgs84_gcrs_with_eop_corrected_iau_cip(
    catalog: &CelestrakEopPolarCatalog,
    utc_mjd: i64,
    elapsed_utc_si_seconds: f64,
    policy: EopPredictionPolicy,
    site: Wgs84ObserverSite,
    modeled: ModeledCipCioAtTt<'_>,
) -> Result<EopCorrectedWgs84GcrsObserver, EopCipCioError> {
    if modeled.source_label.trim().is_empty() {
        return Err(EopCipCioError::MissingModelSource);
    }
    if modeled.tt_epoch.scale() != TimeScale::Tt {
        return Err(EopCipCioError::ModelEpochRequiresTt(modeled.tt_epoch.scale()));
    }
    if !modeled.model_x.is_finite()
        || !modeled.model_y.is_finite()
        || !modeled.cio_s_plus_xy_half_rad.is_finite()
        || modeled.model_x * modeled.model_x + modeled.model_y * modeled.model_y >= 1.0
    {
        return Err(EopCipCioError::InvalidModeledCoordinates);
    }
    let eop = eop_ut1_era_from_utc_si_seconds(
        catalog, utc_mjd, elapsed_utc_si_seconds, policy,
    )
    .map_err(EopCipCioError::Eop)?;
    if modeled.tt_epoch != eop.tt_epoch {
        return Err(EopCipCioError::ModelEpochMismatch);
    }

    let poles = catalog
        .interpolate(utc_mjd, eop.fraction_of_utc_day, policy)
        .map_err(EopCipCioError::Offsets)?;
    let corrected_x = modeled.model_x + poles.cip_dx_rad;
    let corrected_y = modeled.model_y + poles.cip_dy_rad;
    let corrected_cio_s_rad =
        modeled.cio_s_plus_xy_half_rad - corrected_x * corrected_y / 2.0;
    if !corrected_x.is_finite()
        || !corrected_y.is_finite()
        || !corrected_cio_s_rad.is_finite()
        || corrected_x * corrected_x + corrected_y * corrected_y >= 1.0
    {
        return Err(EopCipCioError::InvalidCorrectedCoordinates);
    }
    let observer = wgs84_gcrs_from_eop_utc_and_cip_cio(
        catalog,
        utc_mjd,
        elapsed_utc_si_seconds,
        policy,
        site,
        ExternalCipCioAtTt {
            tt_epoch: modeled.tt_epoch,
            cip_x: corrected_x,
            cip_y: corrected_y,
            cio_locator_s_rad: corrected_cio_s_rad,
            source_label: modeled.source_label,
        },
    )
    .map_err(EopCipCioError::Observer)?;
    Ok(EopCorrectedWgs84GcrsObserver {
        corrected_cip_cio: EopCorrectedCipCio {
            model_x: modeled.model_x,
            model_y: modeled.model_y,
            eop_dx_rad: poles.cip_dx_rad,
            eop_dy_rad: poles.cip_dy_rad,
            corrected_x,
            corrected_y,
            corrected_cio_s_rad,
            quality: poles.quality,
        },
        observer,
    })
}
