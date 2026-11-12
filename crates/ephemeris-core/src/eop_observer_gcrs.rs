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

//! Quality-aware EOP/UTC to WGS84 ground-observer GCRS position.
//!
//! The caller supplies a CelesTrak EOP snapshot, physical SI seconds
//! into an explicitly selected UTC civil day, a WGS84 location and
//! independently prepared CIP X/Y plus celestial CIO locator s,
//! explicitly tagged to the *same* TT epoch computed from EOP DAT.
//!
//! No CIP/CIO model, EOP authenticity check, automatic download,
//! GCRS→DE441 ICRF mapping or observer velocity is inferred.
//! Original VirgoStars integration. 2026-11-12.

use std::error::Error;
use std::fmt;

use crate::calendar::{JulianDate, TimeScale};
use crate::eop_polar_csv::{CelestrakEopPolarCatalog, EopPredictionPolicy};
use crate::eop_ut1_era::{eop_ut1_era_from_utc_si_seconds, EopUt1EraError, EopUt1EraOrientation};
use crate::observer_gcrs::{
    wgs84_observer_to_gcrs, ObserverGcrsError, ObserverGcrsPosition, ObserverOrientationInputs,
    Wgs84ObserverSite,
};

/// Explicit externally sourced CIP/CIO orientation at a **TT** epoch.
///
/// The fields X,Y are dimensionless components of the CIP's GCRS
/// unit direction; s is the **celestial CIO** locator in radians.
/// The provenance string identifies the adopted independent model,
/// but neither authenticates it nor guarantees numerical accuracy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExternalCipCioAtTt<'a> {
    pub tt_epoch: JulianDate,
    pub cip_x: f64,
    pub cip_y: f64,
    pub cio_locator_s_rad: f64,
    pub source_label: &'a str,
}

/// The matching EOP and Earth-centered position-only observer results.
///
/// The EOP's source label and its observed/predicted quality remain
/// available via `eop`. The GCRS vector is **not** a DE441 BCRS/ICRF
/// barycentric state; no motion, apparent corrections or velocity
/// are computed, nor does the source label prove physical accuracy.
#[derive(Clone, Debug, PartialEq)]
pub struct EopWgs84GcrsObserver {
    pub eop: EopUt1EraOrientation,
    pub cip_cio_source_label: String,
    pub observer_gcrs: ObserverGcrsPosition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EopWgs84GcrsError {
    Eop(EopUt1EraError),
    MissingCipCioSource,
    CipCioEpochRequiresTt(TimeScale),
    /// Exact split-JD match is required: no silent resampling.
    CipCioEpochMismatch,
    Observer(ObserverGcrsError),
}

impl fmt::Display for EopWgs84GcrsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eop(error) => write!(f, "EOP-derived UT1/TT orientation: {error}"),
            Self::MissingCipCioSource => write!(f, "CIP/CIO source label cannot be empty"),
            Self::CipCioEpochRequiresTt(scale) => {
                write!(f, "CIP/CIO epoch requires TT, got {scale:?}")
            }
            Self::CipCioEpochMismatch => {
                write!(
                    f,
                    "CIP/CIO split TT epoch differs from the EOP-derived TT epoch"
                )
            }
            Self::Observer(error) => write!(f, "WGS84 observer transformation: {error}"),
        }
    }
}

impl Error for EopWgs84GcrsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Eop(error) => Some(error),
            Self::Observer(error) => Some(error),
            _ => None,
        }
    }
}

/// Produce a geocentric **GCRS position in kilometers** for a WGS84 site.
///
/// 1. Uses `eop_ut1_era_from_utc_si_seconds` to obtain quality-tagged
///    polar EOP xp,yp, normal/positive/negative UTC leap SI seconds,
///    continuous UT1−TAI interpolation, tagged UT1/TT epochs, IAU ERA,
///    and opted-in *approximate* TIO s′(TT).
/// 2. Requires caller-provided CIP X/Y and CIO s at the **exact same
///    split TT epoch**; different scales or split fractions fail closed.
///    This is a contract check and does NOT authenticate a CIP/CIO model.
/// 3. Composes the original `wgs84_observer_to_gcrs` position pipeline,
///    passing the original UT1, xp,yp, s′ and explicit CIP/CIO fields.
///    No frame/astronomy equations are reimplemented here.
///
/// The site is expected to use a compatible ITRS realization; the
/// EOP snapshot, leap schedule, CIP/CIO series and their source frames
/// require independent validation. A successful call does NOT imply
/// full IERS physical accuracy, real DE441 parity, or apparent sky
/// positions. The EOP input may contain forecasts *only if* allowed
/// explicitly by the prediction policy.
pub fn wgs84_gcrs_from_eop_utc_and_cip_cio(
    catalog: &CelestrakEopPolarCatalog,
    utc_mjd: i64,
    elapsed_utc_si_seconds: f64,
    prediction_policy: EopPredictionPolicy,
    site: Wgs84ObserverSite,
    cip_cio: ExternalCipCioAtTt<'_>,
) -> Result<EopWgs84GcrsObserver, EopWgs84GcrsError> {
    if cip_cio.source_label.trim().is_empty() {
        return Err(EopWgs84GcrsError::MissingCipCioSource);
    }
    if cip_cio.tt_epoch.scale() != TimeScale::Tt {
        return Err(EopWgs84GcrsError::CipCioEpochRequiresTt(
            cip_cio.tt_epoch.scale(),
        ));
    }
    let eop = eop_ut1_era_from_utc_si_seconds(
        catalog,
        utc_mjd,
        elapsed_utc_si_seconds,
        prediction_policy,
    )
    .map_err(EopWgs84GcrsError::Eop)?;
    if cip_cio.tt_epoch != eop.tt_epoch {
        return Err(EopWgs84GcrsError::CipCioEpochMismatch);
    }
    let params = ObserverOrientationInputs {
        ut1_epoch: eop.ut1_epoch,
        polar_xp_rad: eop.xp_rad,
        polar_yp_rad: eop.yp_rad,
        tio_locator_sp_rad: eop.approximate_polar.locator.sp_rad,
        cip_x: cip_cio.cip_x,
        cip_y: cip_cio.cip_y,
        cio_locator_s_rad: cip_cio.cio_locator_s_rad,
    };
    let observer_gcrs =
        wgs84_observer_to_gcrs(site, params).map_err(EopWgs84GcrsError::Observer)?;
    Ok(EopWgs84GcrsObserver {
        eop,
        cip_cio_source_label: cip_cio.source_label.to_owned(),
        observer_gcrs,
    })
}
