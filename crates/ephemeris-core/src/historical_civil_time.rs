// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Explicit historical local-clock to UT1 conversion. No timezone database,
//! calendar reform, legal offset, or historical clock reading is inferred.

use crate::calendar::{Calendar, CalendarDateTime, CalendarError, JulianDate, TimeScale};
use std::{error::Error, fmt};

/// The supplied clock reading, expressed in the chosen proleptic calendar.
/// Astronomical year 0 is 1 BCE. The scale field is required to be UT1: it
/// labels the arithmetic, not an assertion that the local clock was UT1.
#[derive(Clone, Copy, Debug)]
pub struct HistoricalCivilInput<'a> {
    pub local: CalendarDateTime,
    pub calendar: Calendar,
    pub offset: HistoricalOffset<'a>,
    /// Caller-estimated uncertainty of the local reading and offset together.
    pub uncertainty_seconds: f64,
}

/// Local clock minus UT1. Local mean solar time is four minutes per degree
/// east; a historical legal or recorded offset must be supplied separately.
#[derive(Clone, Copy, Debug)]
pub enum HistoricalOffset<'a> {
    LocalMeanSolar { longitude_deg_east: f64 },
    Supplied { seconds_east: f64, source: &'a str },
}

#[derive(Clone, Debug)]
pub struct HistoricalCivilEpoch {
    pub jd_ut1: f64,
    pub offset_seconds_east: f64,
    pub offset_source: String,
    pub uncertainty_seconds: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CivilTimeError {
    InvalidOffset,
    InvalidSource,
    InvalidUncertainty,
    InvalidScale,
    Calendar(CalendarError),
}
impl fmt::Display for CivilTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid historical civil time: {self:?}")
    }
}
impl Error for CivilTimeError {}

/// Resolve a local clock only under the explicitly selected convention.
/// UTC, DST, time zones and calendar transitions are deliberately not guessed.
pub fn local_to_ut1(
    input: HistoricalCivilInput<'_>,
) -> Result<HistoricalCivilEpoch, CivilTimeError> {
    if input.local.scale != TimeScale::Ut1 {
        return Err(CivilTimeError::InvalidScale);
    }
    if !input.uncertainty_seconds.is_finite() || input.uncertainty_seconds < 0.0 {
        return Err(CivilTimeError::InvalidUncertainty);
    }
    let (offset, source) = match input.offset {
        HistoricalOffset::LocalMeanSolar { longitude_deg_east } => {
            if !longitude_deg_east.is_finite() || longitude_deg_east.abs() > 180.0 {
                return Err(CivilTimeError::InvalidOffset);
            }
            (
                longitude_deg_east * 240.0,
                "LOCAL_MEAN_SOLAR_TIME".to_owned(),
            )
        }
        HistoricalOffset::Supplied {
            seconds_east,
            source,
        } => {
            if !seconds_east.is_finite() || seconds_east.abs() > 86_400.0 {
                return Err(CivilTimeError::InvalidOffset);
            }
            if source.trim().is_empty()
                || source.len() > 200
                || source.chars().any(char::is_control)
            {
                return Err(CivilTimeError::InvalidSource);
            }
            (seconds_east, source.trim().to_owned())
        }
    };
    let local = input
        .local
        .to_julian_date(input.calendar)
        .map_err(CivilTimeError::Calendar)?;
    let fractional = local.midnight_fraction() - offset / 86_400.0;
    let carry = fractional.floor();
    let epoch = JulianDate::new(
        local.day_number() + carry as i64,
        fractional - carry,
        TimeScale::Ut1,
    )
    .map_err(CivilTimeError::Calendar)?;
    // Reject rollover outside the calendar's supported range as well.
    epoch
        .to_calendar(input.calendar)
        .map_err(CivilTimeError::Calendar)?;
    let (day, fraction) = epoch.parts().map_err(CivilTimeError::Calendar)?;
    Ok(HistoricalCivilEpoch {
        jd_ut1: day + fraction,
        offset_seconds_east: offset,
        offset_source: source,
        uncertainty_seconds: input.uncertainty_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn local(year: i32, month: u8, day: u8, hour: u8) -> CalendarDateTime {
        CalendarDateTime {
            year,
            month,
            day,
            hour,
            minute: 0,
            second: 0.0,
            scale: TimeScale::Ut1,
        }
    }
    fn resolve(
        date: CalendarDateTime,
        calendar: Calendar,
        offset: HistoricalOffset<'_>,
    ) -> HistoricalCivilEpoch {
        local_to_ut1(HistoricalCivilInput {
            local: date,
            calendar,
            offset,
            uncertainty_seconds: 4.0,
        })
        .unwrap()
    }
    #[test]
    fn longitude_offset_and_day_rollover() {
        let calendar = Calendar::ProlepticGregorian;
        let east = resolve(
            local(2000, 1, 1, 0),
            calendar,
            HistoricalOffset::LocalMeanSolar {
                longitude_deg_east: 15.0,
            },
        );
        let west = resolve(
            local(1999, 12, 31, 22),
            calendar,
            HistoricalOffset::LocalMeanSolar {
                longitude_deg_east: -15.0,
            },
        );
        assert_eq!(east.jd_ut1, west.jd_ut1);
        assert_eq!(east.offset_seconds_east, 3600.0);
        assert_eq!(east.uncertainty_seconds, 4.0);
        assert_eq!(east.offset_source, "LOCAL_MEAN_SOLAR_TIME");
    }
    #[test]
    fn julian_and_gregorian_dates_identify_the_same_epoch() {
        let a = resolve(
            local(1582, 10, 4, 12),
            Calendar::ProlepticJulian,
            HistoricalOffset::Supplied {
                seconds_east: 0.0,
                source: "record",
            },
        );
        let b = resolve(
            local(1582, 10, 14, 12),
            Calendar::ProlepticGregorian,
            HistoricalOffset::Supplied {
                seconds_east: 0.0,
                source: "record",
            },
        );
        assert_eq!(a.jd_ut1, b.jd_ut1);
        assert_eq!(a.offset_source, "record");
        assert!(resolve(
            local(0, 2, 29, 12),
            Calendar::ProlepticJulian,
            HistoricalOffset::Supplied {
                seconds_east: 0.0,
                source: "record"
            }
        )
        .jd_ut1
        .is_finite());
    }
    #[test]
    fn invalid_inputs_fail_closed() {
        let mut input = HistoricalCivilInput {
            local: local(2000, 1, 1, 0),
            calendar: Calendar::ProlepticGregorian,
            offset: HistoricalOffset::LocalMeanSolar {
                longitude_deg_east: 181.0,
            },
            uncertainty_seconds: 0.0,
        };
        assert_eq!(
            local_to_ut1(input).unwrap_err(),
            CivilTimeError::InvalidOffset
        );
        input.offset = HistoricalOffset::Supplied {
            seconds_east: 0.0,
            source: " ",
        };
        assert_eq!(
            local_to_ut1(input).unwrap_err(),
            CivilTimeError::InvalidSource
        );
        input.offset = HistoricalOffset::Supplied {
            seconds_east: 0.0,
            source: "record",
        };
        input.uncertainty_seconds = f64::NAN;
        assert_eq!(
            local_to_ut1(input).unwrap_err(),
            CivilTimeError::InvalidUncertainty
        );
        input.uncertainty_seconds = 0.0;
        input.local.scale = TimeScale::Tt;
        assert_eq!(
            local_to_ut1(input).unwrap_err(),
            CivilTimeError::InvalidScale
        );
        input.local = local(1900, 2, 29, 0);
        assert_eq!(
            local_to_ut1(input).unwrap_err(),
            CivilTimeError::Calendar(CalendarError::InvalidDate)
        );
    }
}
