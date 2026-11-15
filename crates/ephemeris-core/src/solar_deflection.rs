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

//! Experimental finite-source solar point-mass deflection, added 2026-11-15.
//! Position only: no light-time, aberration, rate, time conversion or frames.
//! Equation (1), Kaplan (1998), https://aa.usno.navy.mil/downloads/reports/Kaplan1998.pdf.

use crate::six_field_units::IAU_AU_KM;
use std::{error::Error, fmt};

const NOMINAL_SOLAR_GM_KM3_S2: f64 = 132_712_440_000.0;
const SOLAR_RADIUS_KM: f64 = 695_700.0;
const SPEED_OF_LIGHT_KM_S: f64 = 299_792.458;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FiniteSolarDeflection {
    /// Observer-to-source direction in the same fixed axes as the inputs.
    pub direction: [f64; 3],
    /// Small angular difference from the supplied uncorrected direction.
    pub deflection_radians: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolarDeflectionError {
    InvalidGeometry,
    OccultedBySun,
}
impl fmt::Display for SolarDeflectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGeometry => write!(f, "invalid finite-source solar deflection geometry"),
            Self::OccultedBySun => write!(f, "source ray intersects the adopted solar disk"),
        }
    }
}
impl Error for SolarDeflectionError {}

/// First-order finite-source solar point-mass correction. Supply the source
/// and Sun positions relative to the same observer in AU and fixed ICRF axes,
/// sampled consistently with the caller's chosen light-time model. The Sun
/// itself, singular vectors, observers/sources inside the adopted solar disk,
/// and rays that pass through the disk before reaching the source are errors.
/// This function does not compute an apparent direction by itself, does not
/// calculate derivatives, and does not handle planetary multipoles/deflection.
pub fn deflect_finite_source(
    source_from_observer_au: [f64; 3],
    sun_from_observer_au: [f64; 3],
) -> Result<FiniteSolarDeflection, SolarDeflectionError> {
    if source_from_observer_au
        .iter()
        .chain(sun_from_observer_au.iter())
        .any(|x| !x.is_finite())
    {
        return Err(SolarDeflectionError::InvalidGeometry);
    }
    let norm = |v: [f64; 3]| v[0].hypot(v[1]).hypot(v[2]);
    let source_distance = norm(source_from_observer_au);
    let sun_distance = norm(sun_from_observer_au);
    let source_from_sun: [f64; 3] =
        std::array::from_fn(|i| source_from_observer_au[i] - sun_from_observer_au[i]);
    let sun_source_distance = norm(source_from_sun);
    let radius = SOLAR_RADIUS_KM / IAU_AU_KM;
    if !source_distance.is_finite()
        || source_distance <= 0.0
        || !sun_distance.is_finite()
        || sun_distance <= radius
        || !sun_source_distance.is_finite()
        || sun_source_distance <= radius
    {
        return Err(SolarDeflectionError::InvalidGeometry);
    }
    let p = source_from_observer_au.map(|v| v / source_distance);
    let q = source_from_sun.map(|v| v / sun_source_distance);
    let e = sun_from_observer_au.map(|v| -v / sun_distance);
    let dot = |a: [f64; 3], b: [f64; 3]| (0..3).map(|i| a[i] * b[i]).sum::<f64>();
    let pe = dot(p, e).clamp(-1.0, 1.0);
    let qe = dot(q, e).clamp(-1.0, 1.0);
    let pq = dot(p, q).clamp(-1.0, 1.0);
    // Distance along the sightline from observer to closest approach to Sun.
    let along = -pe * sun_distance;
    if along > 0.0 && along < source_distance {
        let impact: [f64; 3] = std::array::from_fn(|i| sun_from_observer_au[i] - along * p[i]);
        if norm(impact) <= radius {
            return Err(SolarDeflectionError::OccultedBySun);
        }
    }
    let denominator = 1.0 + qe;
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err(SolarDeflectionError::InvalidGeometry);
    }
    let scale = 2.0 * NOMINAL_SOLAR_GM_KM3_S2
        / (SPEED_OF_LIGHT_KM_S.powi(2) * sun_distance * IAU_AU_KM * denominator);
    let raw: [f64; 3] = std::array::from_fn(|i| p[i] + scale * (pq * e[i] - pe * q[i]));
    let raw_norm = norm(raw);
    if !raw_norm.is_finite() || raw_norm <= 0.0 {
        return Err(SolarDeflectionError::InvalidGeometry);
    }
    let direction = raw.map(|v| v / raw_norm);
    let cross = [
        p[1] * direction[2] - p[2] * direction[1],
        p[2] * direction[0] - p[0] * direction[2],
        p[0] * direction[1] - p[1] * direction[0],
    ];
    Ok(FiniteSolarDeflection {
        direction,
        deflection_radians: norm(cross).atan2(dot(p, direction)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_geometries_match_independent_erfa_ld_vectors() {
        // Generated using pyerfa 2.0.1.5 ld(bm=1, p, q, e, em=1, dlim=1e-12).
        // Source vectors and ERFA q are consistent Sun/source geometry.
        for (source, expected) in [
            (
                [-0.6, 0.5, 0.2],
                [
                    -0.744_208_400_903_938_1,
                    0.620_173_679_806_020_7,
                    0.248_069_471_922_408_3,
                ],
            ),
            (
                [-1.4, 0.1, 0.05],
                [
                    -0.996_826_384_852_050_7,
                    0.071_202_013_807_544_22,
                    0.035_601_006_903_772_11,
                ],
            ),
            (
                [-0.002, 0.0003, 0.0001],
                [
                    -0.987_729_596_649_101_2,
                    0.148_159_439_500_369_17,
                    0.049_386_479_833_456_4,
                ],
            ),
            ([0.0, 1.0, 0.0], [8.177_096_567_295_8e-9, 1.0, 0.0]),
        ] {
            let actual = deflect_finite_source(source, [-1.0, 0.0, 0.0]).unwrap();
            for (a, e) in actual.direction.into_iter().zip(expected) {
                assert!((a - e).abs() < 1e-12, "{a} vs {e}");
            }
            assert!(actual.deflection_radians.is_finite());
        }
    }
    #[test]
    fn rejects_solar_occultation_but_not_a_source_in_front() {
        assert_eq!(
            deflect_finite_source([-2.0, 0.0, 0.0], [-1.0, 0.0, 0.0]),
            Err(SolarDeflectionError::OccultedBySun)
        );
        let foreground = deflect_finite_source([-0.002, 0.0, 0.0], [-1.0, 0.0, 0.0]).unwrap();
        assert_eq!(foreground.direction, [-1.0, 0.0, 0.0]);
        assert_eq!(
            deflect_finite_source([-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]),
            Err(SolarDeflectionError::InvalidGeometry)
        );
        assert_eq!(
            deflect_finite_source([f64::NAN, 0.0, 0.0], [-1.0, 0.0, 0.0]),
            Err(SolarDeflectionError::InvalidGeometry)
        );
    }
}
