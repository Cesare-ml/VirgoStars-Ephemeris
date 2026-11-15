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

//! Split-JD TCB→TDB from [IAU 2006 Resolution B3](https://www.iau.org/static/resolutions/IAU2006_Resol3.pdf).
//! This linear relationship does not convert UTC, UT1, TT or time zones.
use std::{error::Error, fmt};

const LB: f64 = 1.550_519_768e-8;
const T0_JD: f64 = 2_443_144.500_372_5;
const TDB0_SECONDS: f64 = -6.55e-5;
const SECONDS_PER_DAY: f64 = 86_400.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TcbTdbError {
    InvalidParts,
    UnsupportedEpoch,
}
impl fmt::Display for TcbTdbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParts => write!(f, "finite bounded TCB Julian date parts required"),
            Self::UnsupportedEpoch => write!(f, "TCB epoch outside supported Julian-day range"),
        }
    }
}
impl Error for TcbTdbError {}

/// Convert the same instant from a two-part TCB JD to a two-part TDB JD.
/// The larger part is preserved and the correction applied to the smaller;
/// input parts may be in either order. Supported TCB JD is [2400000,2600000].
/// No astronomical ephemeris or UTC/leap-second data are required.
pub fn tcb_to_tdb(part1: f64, part2: f64) -> Result<(f64, f64), TcbTdbError> {
    if !part1.is_finite() || !part2.is_finite() || part1.abs() > 1e8 || part2.abs() > 1e8 {
        return Err(TcbTdbError::InvalidParts);
    }
    let epoch = part1 + part2;
    if !(2_400_000.0..=2_600_000.0).contains(&epoch) {
        return Err(TcbTdbError::UnsupportedEpoch);
    }
    let (large, small, first_large) = if part1.abs() >= part2.abs() {
        (part1, part2, true)
    } else {
        (part2, part1, false)
    };
    let elapsed_days = (large - T0_JD) + small;
    let correction_days = -LB * elapsed_days + TDB0_SECONDS / SECONDS_PER_DAY;
    let corrected_small = small + correction_days;
    Ok(if first_large {
        (large, corrected_small)
    } else {
        (corrected_small, large)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_independent_erfa_split_jd_and_preserves_order() {
        // pyerfa 2.0.1.5 tcbtdb; numerical references, not ERFA code.
        for (a, b, expected_small) in [
            (2_457_389.0, 0.0, -0.000_220_864_540_678_925_74),
            (2_451_545.0, 0.0, -0.000_130_252_165_437_005_73),
            (2_453_750.5, 0.893_019_599, 0.892_855_136_274_634_4),
        ] {
            let (x, y) = tcb_to_tdb(a, b).unwrap();
            assert_eq!(x, a);
            assert!((y - expected_small).abs() < 1e-15);
            let (x2, y2) = tcb_to_tdb(b, a).unwrap();
            assert_eq!(y2, a);
            assert!((x2 - y).abs() < 1e-15);
        }
        assert_eq!(tcb_to_tdb(f64::NAN, 0.0), Err(TcbTdbError::InvalidParts));
        assert_eq!(tcb_to_tdb(1e100, 0.0), Err(TcbTdbError::InvalidParts));
        assert_eq!(
            tcb_to_tdb(2_300_000.0, 0.0),
            Err(TcbTdbError::UnsupportedEpoch)
        );
    }
}
