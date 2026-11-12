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

//! Extract IAU celestial intermediate pole X,Y from a supplied
//! **complete** GCRS→true-equator/equinox bias-precession-nutation
//! (BPN) matrix at an explicitly tagged TT instant.
//!
//! The third *row*, NOT column, of an SOFA/ERFA-convention BPN matrix
//! is the CIP unit direction in GCRS: [X,Y,Z].
//!
//! The date-dependent celestial CIO series q(TT)=s(TT)+X*Y/2
//! remains independently supplied. Its value cannot be inferred
//! from BPN orientation or a precession-only matrix. This module
//! does NOT calculate the IAU 2006/2000A X/Y/s series.
//!
//! Original VirgoStars Rust implementation, 2026-11-12.
//! Published numerical fixture: <https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c>

use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

use crate::calendar::{CalendarError, JulianDate, TimeScale};
use crate::eop_cip_cio::ModeledCipCioAtTt;

/// Complete, source-identified BPN matrix and independent CIO series.
///
/// `gcrs_to_true_equator` maps GCRS Cartesian **column vectors**
/// into the true equatorial equinox-of-date frame. It is NOT
/// GCRS→CIRS, ITRS→GCRS, mean precession-only, or an inverse matrix.
/// Row 2 contains the north celestial intermediate pole X,Y,Z.
///
/// `q_rad` is the IAU celestial CIO series q=s+X*Y/2 evaluated at
/// exactly the same TT epoch and from the same model conventions.
/// A textual label only identifies the claimed source, not its accuracy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompleteBpnAndCioAtTt<'a> {
    pub tt_epoch: JulianDate,
    pub gcrs_to_true_equator: [[f64; 3]; 3],
    pub cio_series_q_rad: f64,
    pub source_label: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IauCipFromBpnError {
    MissingSource,
    RequiresTt(TimeScale),
    InvalidEpoch(CalendarError),
    NonFiniteMatrix,
    NonOrthogonalMatrix,
    WrongHandedness,
    InvalidNorthCip,
    InvalidCioSeries,
}

impl fmt::Display for IauCipFromBpnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSource => write!(f, "BPN/CIO model source label is missing"),
            Self::RequiresTt(scale) => write!(f, "BPN/CIO input requires TT, got {scale:?}"),
            Self::InvalidEpoch(error) => write!(f, "invalid split TT epoch: {error}"),
            Self::NonFiniteMatrix => write!(f, "BPN matrix contains nonfinite values"),
            Self::NonOrthogonalMatrix => write!(f, "BPN matrix must be orthonormal"),
            Self::WrongHandedness => write!(f, "BPN matrix must be right-handed"),
            Self::InvalidNorthCip => write!(f, "BPN third row is not a valid north CIP"),
            Self::InvalidCioSeries => write!(f, "CIO q=s+XY/2 must be finite in [-pi,pi]"),
        }
    }
}

impl Error for IauCipFromBpnError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidEpoch(error) => Some(error),
            _ => None,
        }
    }
}

/// Build the existing same-TT EOP-correctable `ModeledCipCioAtTt`
/// from an independently obtained **complete** IAU BPN matrix.
///
/// This is a geometric adapter. It rigorously checks finiteness,
/// right-handedness and orthogonality to 5e-11 dimensionless,
/// and uses `[X,Y,Z] = matrix[2]` without transposition.
/// Input provenance and matching model date/conventions cannot be
/// authenticated from a matrix alone.
///
/// In particular a **mean-only** precession matrix can pass these
/// geometric checks, but does NOT satisfy the scientific requirement
/// of complete BPN. A caller must validate its physical model
/// separately before using the result for astrometry.
///
/// The returned `ModeledCipCioAtTt` can be passed unchanged into
/// `eop_cip_cio::wgs84_gcrs_with_eop_corrected_iau_cip`, which then
/// applies CelesTrak/IERS dX,dY at the exact same TT epoch.
pub fn modeled_cip_cio_from_complete_bpn<'a>(
    supplied: CompleteBpnAndCioAtTt<'a>,
) -> Result<ModeledCipCioAtTt<'a>, IauCipFromBpnError> {
    if supplied.source_label.trim().is_empty() {
        return Err(IauCipFromBpnError::MissingSource);
    }
    if supplied.tt_epoch.scale() != TimeScale::Tt {
        return Err(IauCipFromBpnError::RequiresTt(supplied.tt_epoch.scale()));
    }
    supplied
        .tt_epoch
        .parts()
        .map_err(IauCipFromBpnError::InvalidEpoch)?;
    if !supplied.cio_series_q_rad.is_finite() || !(-PI..=PI).contains(&supplied.cio_series_q_rad) {
        return Err(IauCipFromBpnError::InvalidCioSeries);
    }
    let b = supplied.gcrs_to_true_equator;
    if b.iter().flatten().any(|x| !x.is_finite()) {
        return Err(IauCipFromBpnError::NonFiniteMatrix);
    }
    const TOL: f64 = 5.0e-11;
    for (i, row) in b.iter().enumerate() {
        for (j, other) in b.iter().enumerate() {
            let dot: f64 = row.iter().zip(other).map(|(x, y)| x * y).sum();
            let target = if i == j { 1.0 } else { 0.0 };
            if (dot - target).abs() > TOL {
                return Err(IauCipFromBpnError::NonOrthogonalMatrix);
            }
        }
    }
    let cross = [
        b[0][1] * b[1][2] - b[0][2] * b[1][1],
        b[0][2] * b[1][0] - b[0][0] * b[1][2],
        b[0][0] * b[1][1] - b[0][1] * b[1][0],
    ];
    let determinant: f64 = cross.iter().zip(b[2]).map(|(x, y)| x * y).sum();
    if (determinant - 1.0).abs() > TOL {
        return Err(IauCipFromBpnError::WrongHandedness);
    }
    let [x, y, z] = b[2];
    if z <= 0.0 || x.mul_add(x, y * y) >= 1.0 {
        return Err(IauCipFromBpnError::InvalidNorthCip);
    }
    Ok(ModeledCipCioAtTt {
        tt_epoch: supplied.tt_epoch,
        model_x: x,
        model_y: y,
        cio_s_plus_xy_half_rad: supplied.cio_series_q_rad,
        source_label: supplied.source_label,
    })
}
