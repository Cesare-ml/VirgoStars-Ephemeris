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
            return Err(EphemerisError::NonFinite { field: "TDB seconds" });
        }
        Ok(Self(seconds))
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

/// Julian day explicitly on the TDB time scale.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct JulianDayTdb(f64);

impl JulianDayTdb {
    pub fn new(julian_day: f64) -> Result<Self> {
        if !julian_day.is_finite() {
            return Err(EphemerisError::NonFinite { field: "TDB Julian day" });
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
