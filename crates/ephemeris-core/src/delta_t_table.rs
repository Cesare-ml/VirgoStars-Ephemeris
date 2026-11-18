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

// Modification 2026-11-18: preserve unknown source errors and reject malformed errors;
// recovered from the independently evaluated historical-time branch c2eccd5.
//! ΔT = TT − UT1 from caller-supplied US Naval Observatory tables.
//!
//! **No data is bundled and nothing is downloaded.** The caller passes the text of
//! three public files published by the USNO Nautical Almanac Office
//! (`https://maia.usno.navy.mil/ser7/`):
//!
//! | File | Content | Used for |
//! |---|---|---|
//! | `historic_deltat.data` | ΔT every half year, 1657 – 1984.5, with source-reported errors | before 1973 |
//! | `deltat.data` | ΔT on the first of each month, 1973-02-01 – the latest observation | measured period |
//! | `deltat.preds` | quarterly predictions of TT−UT with errors | after the last observation, flagged `Predicted` |
//!
//! # Rules
//!
//! - Points come from `historic_deltat.data` for decimal years `< 1973.0`, from
//!   `deltat.data` from 1973-02-01, and from `deltat.preds` only for MJD later than
//!   the last observation. Overlaps are resolved in that order, never averaged.
//! - Between two points ΔT is interpolated **linearly in UT days**; nothing is
//!   extrapolated: before the first and after the last point the query fails.
//! - Every value carries its legacy kind (`Measured` or `Predicted`) and the
//!   interpolated source-reported error when available. `Measured` identifies
//!   the non-prediction table; it does not imply direct atomic-clock
//!   observations in centuries reconstructed from historical evidence.
//!   Between points an error is returned only if both endpoints report one;
//!   it remains a source statistic, not a bound on the interpolation error.
//! - The decimal year of the historic file is converted with the Julian-year
//!   convention `JD = 2451545.0 + (year − 2000)·365.25`. The source file does
//!   not specify this mapping. Relative to Gregorian calendar-year fractions,
//!   the mismatch reaches 3.75 days over the supplied 1657–1984.5 snapshot;
//!   the resulting ΔT shift depends on the local slope and is not assumed.
//!
//! The values are observational/model data of the USNO, not a product of this
//! project, and are not covered by this repository's CPAL-1.0 license.

use std::error::Error;
use std::fmt;

use crate::calendar::{Calendar, TimeScale};
use crate::calendar_decimal_hours::{julday_f64, DecimalHourDateTime};

const J2000_JD: f64 = 2_451_545.0;
const MJD_TO_JD: f64 = 2_400_000.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeltaTKind {
    /// Legacy label for a non-prediction table point, including reconstructions.
    Measured,
    /// At least one neighboring point is from the prediction table.
    Predicted,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    jd_ut: f64,
    delta_t_seconds: f64,
    uncertainty_seconds: Option<f64>,
    kind: DeltaTKind,
}

/// ΔT at one instant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeltaTValue {
    pub delta_t_seconds: f64,
    /// Source-reported error interpolated from neighbouring points, when known;
    /// this does not establish its statistical meaning or interpolation error.
    pub uncertainty_seconds: Option<f64>,
    /// `Predicted` if either neighbour is a prediction.
    pub kind: DeltaTKind,
}

#[derive(Debug, PartialEq)]
pub enum DeltaTTableError {
    /// A line of the named file could not be parsed.
    Parse {
        file: &'static str,
        line: usize,
    },
    /// Points are not strictly increasing in time.
    NotIncreasing {
        file: &'static str,
    },
    /// The files do not form one continuous series.
    Gap,
    TooFewPoints,
    /// The queried instant is outside the table (no extrapolation).
    OutOfRange {
        jd_ut: f64,
    },
    NonFinite,
}

impl fmt::Display for DeltaTTableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { file, line } => write!(f, "{file}: cannot parse line {line}"),
            Self::NotIncreasing { file } => write!(f, "{file}: dates are not strictly increasing"),
            Self::Gap => write!(f, "the delta T files do not join into one series"),
            Self::TooFewPoints => write!(f, "delta T table has too few points"),
            Self::OutOfRange { jd_ut } => {
                write!(f, "UT Julian date {jd_ut} is outside the delta T table")
            }
            Self::NonFinite => write!(f, "non-finite value"),
        }
    }
}
impl Error for DeltaTTableError {}

/// A continuous, validated ΔT series.
#[derive(Clone, Debug, PartialEq)]
pub struct DeltaTTable {
    points: Vec<Point>,
}

fn parse_monthly(text: &str) -> Result<Vec<Point>, DeltaTTableError> {
    let mut points = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let fields: Vec<&str> = t.split_whitespace().collect();
        let bad = || DeltaTTableError::Parse {
            file: "deltat.data",
            line: i + 1,
        };
        if fields.len() != 4 {
            return Err(bad());
        }
        let year: i32 = fields[0].parse().map_err(|_| bad())?;
        let month: u8 = fields[1].parse().map_err(|_| bad())?;
        let day: u8 = fields[2].parse().map_err(|_| bad())?;
        let value: f64 = fields[3].parse().map_err(|_| bad())?;
        if !value.is_finite() {
            return Err(bad());
        }
        let jd = julday_f64(
            DecimalHourDateTime {
                year,
                month,
                day,
                decimal_hour: 0.0,
                scale: TimeScale::Ut1,
            },
            Calendar::ProlepticGregorian,
        )
        .map_err(|_| bad())?;
        points.push(Point {
            jd_ut: jd,
            delta_t_seconds: value,
            uncertainty_seconds: None,
            kind: DeltaTKind::Measured,
        });
    }
    Ok(points)
}

fn parse_historic(text: &str) -> Result<Vec<Point>, DeltaTTableError> {
    let mut points = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // Header lines do not start with a number.
        let Some(first) = fields.first().and_then(|f| f.parse::<f64>().ok()) else {
            continue;
        };
        let bad = || DeltaTTableError::Parse {
            file: "historic_deltat.data",
            line: i + 1,
        };
        if fields.len() < 3 {
            return Err(bad());
        }
        let delta: f64 = fields[1].parse().map_err(|_| bad())?;
        let error: f64 = fields[2].parse().map_err(|_| bad())?;
        if !first.is_finite()
            || !delta.is_finite()
            || !error.is_finite()
            || error < 0.0
            || !(1000.0..3000.0).contains(&first)
        {
            return Err(bad());
        }
        points.push(Point {
            jd_ut: J2000_JD + (first - 2000.0) * 365.25,
            delta_t_seconds: delta,
            uncertainty_seconds: Some(error),
            kind: DeltaTKind::Measured,
        });
    }
    Ok(points)
}

fn parse_preds(text: &str) -> Result<Vec<Point>, DeltaTTableError> {
    let mut points = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let fields: Vec<&str> = line.split('\t').map(str::trim).collect();
        let Some(mjd) = fields.first().and_then(|f| f.parse::<f64>().ok()) else {
            continue; // header
        };
        let bad = || DeltaTTableError::Parse {
            file: "deltat.preds",
            line: i + 1,
        };
        if fields.len() < 3 {
            return Err(bad());
        }
        let delta: f64 = fields[2].parse().map_err(|_| bad())?;
        let error = match fields.get(4).copied() {
            Some("") | None => None,
            Some(value) => {
                let parsed: f64 = value.parse().map_err(|_| bad())?;
                if !parsed.is_finite() || parsed < 0.0 {
                    return Err(bad());
                }
                Some(parsed)
            }
        };
        if !mjd.is_finite() || !delta.is_finite() {
            return Err(bad());
        }
        points.push(Point {
            jd_ut: mjd + MJD_TO_JD,
            delta_t_seconds: delta,
            uncertainty_seconds: error,
            kind: DeltaTKind::Predicted,
        });
    }
    Ok(points)
}

impl DeltaTTable {
    /// Build the series from the text of the three USNO files. `preds` is optional;
    /// without it the table ends at the last observation.
    pub fn from_usno(
        deltat_data: &str,
        historic_deltat_data: &str,
        deltat_preds: Option<&str>,
    ) -> Result<Self, DeltaTTableError> {
        let monthly = parse_monthly(deltat_data)?;
        let historic = parse_historic(historic_deltat_data)?;
        if monthly.windows(2).any(|w| w[1].jd_ut <= w[0].jd_ut) {
            return Err(DeltaTTableError::NotIncreasing {
                file: "deltat.data",
            });
        }
        if historic.windows(2).any(|w| w[1].jd_ut <= w[0].jd_ut) {
            return Err(DeltaTTableError::NotIncreasing {
                file: "historic_deltat.data",
            });
        }
        let first_monthly = monthly.first().ok_or(DeltaTTableError::TooFewPoints)?.jd_ut;
        let last_monthly = monthly.last().ok_or(DeltaTTableError::TooFewPoints)?.jd_ut;
        let cutoff = J2000_JD + (1973.0 - 2000.0) * 365.25;
        let mut points: Vec<Point> = historic.into_iter().filter(|p| p.jd_ut < cutoff).collect();
        if points.is_empty() || points.last().map(|p| p.jd_ut).unwrap_or(0.0) >= first_monthly {
            return Err(DeltaTTableError::Gap);
        }
        // The historic file must reach within one year of the monthly series.
        if first_monthly - points.last().map(|p| p.jd_ut).unwrap_or(0.0) > 370.0 {
            return Err(DeltaTTableError::Gap);
        }
        points.extend(monthly);
        if let Some(text) = deltat_preds {
            let predicted = parse_preds(text)?;
            points.extend(predicted.into_iter().filter(|p| p.jd_ut > last_monthly));
        }
        if points.len() < 10 || points.windows(2).any(|w| w[1].jd_ut <= w[0].jd_ut) {
            return Err(DeltaTTableError::NotIncreasing {
                file: "combined series",
            });
        }
        Ok(Self { points })
    }

    pub fn first_jd_ut(&self) -> f64 {
        self.points[0].jd_ut
    }
    pub fn last_jd_ut(&self) -> f64 {
        self.points[self.points.len() - 1].jd_ut
    }
    /// UT Julian date of the last **measured** point.
    pub fn last_measured_jd_ut(&self) -> f64 {
        self.points
            .iter()
            .rev()
            .find(|p| p.kind == DeltaTKind::Measured)
            .map_or(self.first_jd_ut(), |p| p.jd_ut)
    }

    /// ΔT at a UT Julian date, linear between points; error outside the table.
    pub fn delta_t(&self, jd_ut: f64) -> Result<DeltaTValue, DeltaTTableError> {
        if !jd_ut.is_finite() {
            return Err(DeltaTTableError::NonFinite);
        }
        if jd_ut < self.first_jd_ut() || jd_ut > self.last_jd_ut() {
            return Err(DeltaTTableError::OutOfRange { jd_ut });
        }
        let upper = self
            .points
            .partition_point(|p| p.jd_ut <= jd_ut)
            .min(self.points.len() - 1);
        let (a, b) = if self.points[upper].jd_ut == jd_ut {
            (self.points[upper], self.points[upper])
        } else {
            (self.points[upper - 1], self.points[upper])
        };
        let weight = if b.jd_ut > a.jd_ut {
            (jd_ut - a.jd_ut) / (b.jd_ut - a.jd_ut)
        } else {
            0.0
        };
        // At an exact point only that point counts (weight 0 ignores the next one).
        let (uncertainty, kind) = if weight == 0.0 {
            (a.uncertainty_seconds, a.kind)
        } else {
            let uncertainty = match (a.uncertainty_seconds, b.uncertainty_seconds) {
                (Some(x), Some(y)) => Some(x + (y - x) * weight),
                _ => None,
            };
            let kind = if a.kind == DeltaTKind::Predicted || b.kind == DeltaTKind::Predicted {
                DeltaTKind::Predicted
            } else {
                DeltaTKind::Measured
            };
            (uncertainty, kind)
        };
        Ok(DeltaTValue {
            delta_t_seconds: a.delta_t_seconds + (b.delta_t_seconds - a.delta_t_seconds) * weight,
            uncertainty_seconds: uncertainty,
            kind,
        })
    }
}
