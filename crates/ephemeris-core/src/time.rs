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

//! Minimal typed TDB/ET epoch representation.
//!
//! This module deliberately does **not** convert UTC, UT1, TAI, or TT to TDB.
//! Such conversions require explicit time-scale models and, for UTC, leap
//! second handling. Passing a UTC Julian day as TDB would be a silent error.

use crate::error::{EphemerisError, Result};

/// Julian date corresponding to the J2000 reference epoch on the TDB scale.
pub const J2000_JD_TDB: f64 = 2_451_545.0;
pub const SECONDS_PER_DAY: f64 = 86_400.0;

/// SPK-style ephemeris time: TDB seconds past J2000.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct TdbSecondsPastJ2000(f64);

impl TdbSecondsPastJ2000 {
    pub fn new(seconds: f64) -> Result<Self> {
        if !seconds.is_finite() {
            return Err(EphemerisError::NonFinite {
                field: "TDB seconds",
            });
        }
        Ok(Self(seconds))
    }

    pub fn value(self) -> f64 {
        self.0
    }

    /// Convert TDB seconds since J2000 to a single-f64 TDB Julian day.
    ///
    /// This is an epoch representation change, not a time-scale conversion.
    /// Around modern dates the single-f64 Julian day cannot preserve every
    /// sub-millisecond ET offset; retain ET seconds or use a split JD when
    /// finer resolution is required.
    pub fn to_julian_day_tdb(self) -> Result<JulianDayTdb> {
        let julian_day = J2000_JD_TDB + self.0 / SECONDS_PER_DAY;
        if !julian_day.is_finite() {
            return Err(EphemerisError::ArithmeticOverflow);
        }
        JulianDayTdb::new(julian_day)
    }
}

/// Julian day explicitly on the TDB time scale.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct JulianDayTdb(f64);

impl JulianDayTdb {
    pub fn new(julian_day: f64) -> Result<Self> {
        if !julian_day.is_finite() {
            return Err(EphemerisError::NonFinite {
                field: "TDB Julian day",
            });
        }
        Ok(Self(julian_day))
    }

    pub fn value(self) -> f64 {
        self.0
    }

    /// Simple JD(TDB) -> seconds past J2000(TDB) conversion.
    ///
    /// A single f64 Julian day loses sub-millisecond resolution at large
    /// epochs; a two-part JD representation is planned for precision work.
    pub fn to_tdb_seconds_past_j2000(self) -> Result<TdbSecondsPastJ2000> {
        let seconds = (self.0 - J2000_JD_TDB) * SECONDS_PER_DAY;
        if !seconds.is_finite() {
            return Err(EphemerisError::ArithmeticOverflow);
        }
        TdbSecondsPastJ2000::new(seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j2000_is_zero_et() {
        let et = JulianDayTdb::new(J2000_JD_TDB)
            .unwrap()
            .to_tdb_seconds_past_j2000()
            .unwrap();
        assert_eq!(et.value(), 0.0);
    }

    #[test]
    fn half_day_is_43200_seconds() {
        let et = JulianDayTdb::new(J2000_JD_TDB + 0.5)
            .unwrap()
            .to_tdb_seconds_past_j2000()
            .unwrap();
        assert_eq!(et.value(), 43_200.0);
    }

    #[test]
    fn rejects_nonfinite_time() {
        assert!(JulianDayTdb::new(f64::NAN).is_err());
        assert!(TdbSecondsPastJ2000::new(f64::INFINITY).is_err());
    }
}
