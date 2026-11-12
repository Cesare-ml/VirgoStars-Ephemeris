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

//! Explicit and *experimental* GCRS observer to DE441-ICRF sightline bridge.
//!
//! The caller must provide a frame rotation, a physically sourced
//! TDB-minus-UT1 offset, and an audit identifier describing its origin.
//! This API verifies numerical coherence, NOT the astronomical accuracy
//! or physical adequacy of the claimed GCRS↔DE441 ICRF transform.
//!
//! A proper 3x3 rotation is only a positional approximation to a
//! generally relativistic reference-system mapping. No translational
//! correction, coordinate time transformation, velocity conversion,
//! light-time, deflection or aberration is implemented.
//!
//! Original VirgoStars implementation, 2026-11-12. See IERS 2010 ch. 5.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::de441::{Body, De441, JulianDateTdb};
use crate::de441_observer_geometry::{
    de441_geometric_observer_sightline_icrf, De441IcrfObserverError, De441IcrfObserverSightline,
};
use crate::observer_gcrs::{
    wgs84_observer_to_gcrs, ObserverGcrsError, ObserverGcrsPosition,
    ObserverOrientationInputs, Wgs84ObserverSite,
};

const SECONDS_PER_DAY: f64 = 86_400.0;
const EPOCH_MATCH_TOLERANCE_SECONDS: f64 = 0.000_1;
const ROTATION_TOLERANCE: f64 = 1.0e-10;

/// Caller-supplied, documented alignment at a single physical epoch.
///
/// All three fields are MANDATORY. Neither identity rotation nor
/// TDB=UT1 is supplied as a default; an identity rotation, if chosen
/// explicitly by the caller, must be separately justified and cannot
/// be considered validated merely because this API accepts it.
///
/// `source_label` must identify the independent model, datum or
/// reference adopted by the application (not a proof of its accuracy).
/// The matrix transforms GCRS **geocentric position components**
/// to the adopted geocentric DE441 ICRF axes in kilometers. It must
/// be a proper orthogonal right-handed 3x3 rotation, row-major and
/// multiplying column vectors.
///
/// `tdb_minus_ut1_seconds` is the externally computed coordinate
/// time difference at this physical epoch, not a built-in ΔT or
/// a conversion algorithm. The caller must supply UTC/EOP/TT/TDB
/// inputs from compatible scientific sources.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExplicitGcrsIcrfAlignment<'a> {
    pub rotation_gcrs_to_icrf: [[f64; 3]; 3],
    pub tdb_minus_ut1_seconds: f64,
    pub source_label: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wgs84De441GeometricSightline {
    pub observer_gcrs: ObserverGcrsPosition,
    pub observer_icrf_km: [f64; 3],
    pub sightline_icrf: De441IcrfObserverSightline,
}

/// This error indicates only a numerical/contract error; a successful
/// result does not establish astrophysical fidelity.
#[derive(Debug)]
pub enum ObserverFrameBridgeError {
    MissingAlignmentSource,
    InvalidAlignmentRotation,
    InvalidTdbMinusUt1Offset,
    InvalidTdbEpoch,
    EpochMismatch,
    UnrepresentableFramePosition,
    Observer(ObserverGcrsError),
    De441(De441IcrfObserverError),
}

impl fmt::Display for ObserverFrameBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAlignmentSource => write!(f, "frame alignment requires a source label"),
            Self::InvalidAlignmentRotation => {
                write!(f, "GCRS-to-ICRF matrix must be a proper orthogonal rotation")
            }
            Self::InvalidTdbMinusUt1Offset => write!(f, "TDB-minus-UT1 seconds must be finite"),
            Self::InvalidTdbEpoch => write!(f, "TDB Julian date must be finite"),
            Self::EpochMismatch => write!(f, "TDB epoch does not match explicit UT1 offset"),
            Self::UnrepresentableFramePosition => write!(f, "non-finite rotated ICRF observer"),
            Self::Observer(error) => write!(f, "WGS84/GCRS observer: {error}"),
            Self::De441(error) => write!(f, "DE441 observer sightline: {error}"),
        }
    }
}

impl Error for ObserverFrameBridgeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Observer(error) => Some(error),
            Self::De441(error) => Some(error),
            _ => None,
        }
    }
}

fn is_rotation(matrix: &[[f64; 3]; 3]) -> bool {
    if matrix.iter().flatten().any(|entry| !entry.is_finite()) {
        return false;
    }
    for (i, row) in matrix.iter().enumerate() {
        for (j, other) in matrix.iter().enumerate() {
            let dot = row.iter().zip(other).map(|(x, y)| x * y).sum::<f64>();
            let expected = if i == j { 1.0 } else { 0.0 };
            if (dot - expected).abs() > ROTATION_TOLERANCE {
                return false;
            }
        }
    }
    let m = matrix;
    let determinant = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    (determinant - 1.0).abs() <= ROTATION_TOLERANCE
}

/// Combine WGS84→GCRS observer geometry with a DE441 ICRF target,
/// using **only explicitly supplied** frame and time alignment.
///
/// The UT1 value is read from `orientation.ut1_epoch` (a split JD),
/// and checked against the caller's TDB JD and externally supplied
/// TDB−UT1 offset within 100 microseconds of single-f64-JD
/// quantization. The check CANNOT validate either time standard.
/// It deliberately makes no UTC, TT or TDB conversions.
///
/// The caller must first independently validate that a proper
/// rotation is a sufficient approximation for its GCRS/DE441
/// reference system and desired accuracy. The accepted source
/// label is for auditability only, not evidence of validation.
///
/// The result is an **instantaneous geometric**, fixed-axis
/// position-only topocentric line of sight: no observer velocity,
/// target emission epoch, aberration, frame derivative, general
/// relativistic terms or JPL/Horizons parity.
pub fn wgs84_de441_geometric_sightline_with_alignment<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    site: Wgs84ObserverSite,
    orientation: ObserverOrientationInputs,
    epoch_tdb: JulianDateTdb,
    alignment: ExplicitGcrsIcrfAlignment<'_>,
) -> Result<Wgs84De441GeometricSightline, ObserverFrameBridgeError> {
    if alignment.source_label.trim().is_empty() {
        return Err(ObserverFrameBridgeError::MissingAlignmentSource);
    }
    if !is_rotation(&alignment.rotation_gcrs_to_icrf) {
        return Err(ObserverFrameBridgeError::InvalidAlignmentRotation);
    }
    if !alignment.tdb_minus_ut1_seconds.is_finite() {
        return Err(ObserverFrameBridgeError::InvalidTdbMinusUt1Offset);
    }
    if !epoch_tdb.0.is_finite() {
        return Err(ObserverFrameBridgeError::InvalidTdbEpoch);
    }

    // Compute the JD difference with the split-UT1 day first, retaining
    // the UT1 fraction until after removing the large integral day.
    // A single-f64 TDB JD still has ~tens-of-microseconds precision
    // near modern epochs, motivating the explicit 100-us guard.
    let ut1 = orientation.ut1_epoch;
    let (ut1_day, ut1_fraction) = ut1
        .parts()
        .map_err(|_| ObserverFrameBridgeError::EpochMismatch)?;
    let epoch_difference_seconds =
        ((epoch_tdb.0 - ut1_day) - ut1_fraction) * SECONDS_PER_DAY;
    if !epoch_difference_seconds.is_finite()
        || (epoch_difference_seconds - alignment.tdb_minus_ut1_seconds).abs()
            > EPOCH_MATCH_TOLERANCE_SECONDS
    {
        return Err(ObserverFrameBridgeError::EpochMismatch);
    }

    let observer_gcrs =
        wgs84_observer_to_gcrs(site, orientation).map_err(ObserverFrameBridgeError::Observer)?;
    let observer_icrf_km: [f64; 3] = alignment.rotation_gcrs_to_icrf.map(|row| {
        row.iter()
            .zip(observer_gcrs.position_gcrs_km)
            .map(|(component, position)| component * position)
            .sum()
    });
    if observer_icrf_km.iter().any(|value| !value.is_finite()) {
        return Err(ObserverFrameBridgeError::UnrepresentableFramePosition);
    }

    let sightline_icrf =
        de441_geometric_observer_sightline_icrf(ephemeris, target, epoch_tdb, observer_icrf_km)
            .map_err(ObserverFrameBridgeError::De441)?;
    Ok(Wgs84De441GeometricSightline {
        observer_gcrs,
        observer_icrf_km,
        sightline_icrf,
    })
}
