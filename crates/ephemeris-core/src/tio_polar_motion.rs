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

//! Explicit composition of the conventional approximate TIO s′(TT)
//! and independently supplied IERS polar motion xp,yp (radians).
//!
//! This is a position-only ITRS→TIRS matrix, not a full Earth
//! orientation model. It requires a TT epoch for s′, and the caller
//! is responsible for xp/yp EOP data from the same physical epoch.
//! The source of xp/yp and the equivalence of TT and EOP epochs
//! cannot be validated by this mathematical function.
//!
//! Derived strictly from separate original VirgoStars modules:
//! `tio_locator` (secular IERS convention) and `polar_motion`
//! (IERS 2010 W(t)); no SOFA/ERFA code is copied.
//!
//! Numerical reference: independent ERFA t_sp00 and t_pom00 fixtures.
//! <https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c>

use std::error::Error;
use std::fmt;

use crate::calendar::JulianDate;
use crate::polar_motion::{itrs_to_tirs_polar_motion_matrix, PolarMotionError, PolarMotionMatrix};
use crate::tio_locator::{approximate_tio_locator_sp00, ApproximateTioLocator, TioLocatorError};

/// Matrix and provenance-aware approximation result for one TT epoch.
///
/// `locator` preserves the approximation's value and TT-day
/// derivative, making the deliberate choice visible to the caller.
/// `itrs_to_tirs` is row-major, maps ITRS Cartesian columns to
/// TIRS Cartesian columns, and preserves position units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApproximateTioPolarMotion {
    pub locator: ApproximateTioLocator,
    pub itrs_to_tirs: PolarMotionMatrix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApproximateTioPolarMotionError {
    Locator(TioLocatorError),
    PolarMotion(PolarMotionError),
}

impl fmt::Display for ApproximateTioPolarMotionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Locator(error) => write!(f, "TIO s-prime approximation: {error}"),
            Self::PolarMotion(error) => write!(f, "ITRS-to-TIRS polar motion: {error}"),
        }
    }
}

impl Error for ApproximateTioPolarMotionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Locator(error) => Some(error),
            Self::PolarMotion(error) => Some(error),
        }
    }
}

/// Calculate a **caller-opted-in approximate** ITRS→TIRS polar matrix.
///
/// Requires TT-tagged `tt_epoch`. `xp_rad` and `yp_rad` must be
/// independent EOP polar-motion measurements for that same epoch,
/// in **radians**, not arcseconds or degrees. The caller is responsible
/// for their physical provenance and epoch matching.
///
/// Conventional `s′≈−47µas × centuries_since_J2000(TT)` is evaluated
/// with the bounded split-JD implementation and passed explicitly to
/// `polar_motion::itrs_to_tirs_polar_motion_matrix`. The result includes
/// both the matrix and the approximate s′/TT-day rate. No zero-pole
/// assumption, UTC=UT1 or TT=UT1 shortcut, EOP acquisition/interpolation,
/// CIO locator s (unprimed), or observer velocity computation occurs.
///
/// If a higher-accuracy measured/modeled s′ is available, use
/// `itrs_to_tirs_polar_motion_matrix(xp,yp,sp)` directly instead.
pub fn itrs_to_tirs_polar_motion_with_approximate_sp00(
    tt_epoch: JulianDate,
    xp_rad: f64,
    yp_rad: f64,
) -> Result<ApproximateTioPolarMotion, ApproximateTioPolarMotionError> {
    let locator =
        approximate_tio_locator_sp00(tt_epoch).map_err(ApproximateTioPolarMotionError::Locator)?;
    let itrs_to_tirs = itrs_to_tirs_polar_motion_matrix(xp_rad, yp_rad, locator.sp_rad)
        .map_err(ApproximateTioPolarMotionError::PolarMotion)?;
    Ok(ApproximateTioPolarMotion {
        locator,
        itrs_to_tirs,
    })
}
