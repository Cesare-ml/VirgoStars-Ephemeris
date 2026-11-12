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

//! Strict, fully offline comparison of TWO six-field CSV exports in
//! **identical** scientific conventions. Pure Rust text/math validation;
//! this utility implements NO ephemeris or coordinate model.
//!
//! IMPORTANT: Two VirgoStars files agreeing proves repeatability ONLY.
//! Independent astronomy accuracy needs separately sourced, verified
//! reference data transformed to precisely the same correction/frame set.

use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, Write};

const HEADER: &str = "target,center,reception_tdb_jd,emission_tdb_jd,fixed_tilt_rad,longitude_deg,latitude_deg,distance_au,longitude_speed_deg_per_tdb_day,latitude_speed_deg_per_tdb_day,distance_speed_au_per_tdb_day,light_time_seconds,light_time_rate,observer_acceleration_source,frame,corrections";
const OUTPUT_HEADER: &str = "target,longitude_error_arcsec,latitude_error_arcsec,distance_error_au,longitude_speed_error_arcsec_per_tdb_day,latitude_speed_error_arcsec_per_tdb_day,distance_speed_error_au_per_tdb_day,light_time_error_seconds,light_time_rate_error,within_tolerances";

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

/// All numerical columns 2..=12, in the exact export order.
///
/// [reception TDB JD, emission TDB JD, fixed tilt radians,
///  longitude degrees, latitude degrees, range AU,
///  longitude/latitude speeds degrees/TDB day, range speed AU/TDB day,
///  light-time seconds, light-time rate s/s].
#[derive(Debug, Clone)]
struct Row {
    body: String,
    values: [f64; 11],
}

fn supported_body(body: &str) -> bool {
    matches!(
        body,
        "Sun"
            | "Moon"
            | "Mercury"
            | "Venus"
            | "MarsBarycenter"
            | "JupiterBarycenter"
            | "SaturnBarycenter"
            | "UranusBarycenter"
            | "NeptuneBarycenter"
            | "PlutoBarycenter"
    )
}

/// Strict CSV subset: no quoting is needed because all contract columns
/// use fixed ASCII identifiers or finite numbers, never commas/newlines.
fn parse_csv(input: &str) -> Result<Vec<Row>, String> {
    let mut lines = input.lines();
    if lines.next().map(str::trim_end) != Some(HEADER) {
        return Err("CSV must begin with the exact 16-column aberrated_csv header".into());
    }
    let mut rows: Vec<Row> = Vec::new();
    for (index, raw) in lines.enumerate() {
        let line_number = index + 2;
        let parts: Vec<&str> = raw.trim_end().split(',').collect();
        if parts.len() != 16 {
            return Err(format!("line {line_number}: expected exactly 16 columns"));
        }
        let body = parts[0];
        if !supported_body(body) || parts[1] != "Earth" {
            return Err(format!("line {line_number}: unsupported body/center pair"));
        }
        if parts[13] != "AnalyticDe441Chebyshev"
            || parts[14] != "fixed_ICRF_equatorial"
            || parts[15] != "LT_plus_annual_SR_aberration"
        {
            return Err(format!(
                "line {line_number}: correction, frame or acceleration provenance differs"
            ));
        }

        let mut values = [0.0; 11];
        for (i, value) in values.iter_mut().enumerate() {
            *value = parts[i + 2].parse::<f64>().map_err(|_| {
                format!("line {line_number}: invalid number in column {}", i + 3)
            })?;
            if !value.is_finite() {
                return Err(format!(
                    "line {line_number}: nonfinite number in column {}",
                    i + 3
                ));
            }
        }
        // Fixed ICRF axes require exactly zero tilt, not a silently
        // relabelled rotated fixed frame. The exporter promises this.
        if values[2] != 0.0
            || !(0.0..360.0).contains(&values[3])
            || !(-90.0..=90.0).contains(&values[4])
            || values[5] <= 0.0
            || values[9] <= 0.0
            || values[1] >= values[0]
        {
            return Err(format!("line {line_number}: physically invalid fixed-frame fields"));
        }
        if let Some(first) = rows.first() {
            if values[0] != first.values[0] {
                return Err(format!(
                    "line {line_number}: reception TDB Julian date differs within one batch"
                ));
            }
        }
        rows.push(Row {
            body: body.to_string(),
            values,
        });
    }
    if rows.is_empty() {
        return Err("at least one target row is required".into());
    }
    Ok(rows)
}

#[derive(Clone, Copy, Debug)]
struct Thresholds {
    angle_arcsec: f64,
    distance_au: f64,
    angular_rate_arcsec_per_tdb_day: f64,
    radial_rate_au_per_tdb_day: f64,
    light_time_seconds: f64,
    light_time_rate: f64,
}

impl Thresholds {
    fn parse(args: &[String]) -> Result<Self, io::Error> {
        if args.len() != 6 {
            return Err(invalid("six nonnegative error tolerances are required"));
        }
        let mut values = [0.0; 6];
        for (i, item) in args.iter().enumerate() {
            let n = item
                .parse::<f64>()
                .map_err(|_| invalid(format!("invalid tolerance {}", i + 1)))?;
            if !n.is_finite() || n < 0.0 {
                return Err(invalid(format!(
                    "tolerance {} must be finite and nonnegative",
                    i + 1
                )));
            }
            values[i] = n;
        }
        Ok(Self {
            angle_arcsec: values[0],
            distance_au: values[1],
            angular_rate_arcsec_per_tdb_day: values[2],
            radial_rate_au_per_tdb_day: values[3],
            light_time_seconds: values[4],
            light_time_rate: values[5],
        })
    }

    fn limits(self) -> [f64; 8] {
        [
            self.angle_arcsec,
            self.angle_arcsec,
            self.distance_au,
            self.angular_rate_arcsec_per_tdb_day,
            self.angular_rate_arcsec_per_tdb_day,
            self.radial_rate_au_per_tdb_day,
            self.light_time_seconds,
            self.light_time_rate,
        ]
    }
}

#[derive(Debug)]
struct Comparison {
    body: String,
    /// Unsigned errors, in units and column order given by OUTPUT_HEADER.
    residuals: [f64; 8],
    passes: bool,
}

fn compare(actual: &[Row], reference: &[Row], tolerances: Thresholds) -> Result<Vec<Comparison>, String> {
    if actual.len() != reference.len() {
        return Err("actual and reference target counts differ; no partial matching".into());
    }
    let mut result = Vec::with_capacity(actual.len());
    let limits = tolerances.limits();
    for (index, (a, b)) in actual.iter().zip(reference).enumerate() {
        if a.body != b.body {
            return Err(format!("row {}: target/order mismatch", index + 2));
        }
        if a.values[0] != b.values[0] || a.values[2] != b.values[2] {
            return Err(format!(
                "row {}: reception TDB epoch or fixed-frame tilt mismatch",
                index + 2
            ));
        }
        // Wrap the difference across 0/360, but do not normalize an
        // invalid source longitude: that already failed parse_csv.
        let longitude_error = (a.values[3] - b.values[3] + 180.0).rem_euclid(360.0) - 180.0;
        let residuals = [
            longitude_error.abs() * 3600.0,
            (a.values[4] - b.values[4]).abs() * 3600.0,
            (a.values[5] - b.values[5]).abs(),
            (a.values[6] - b.values[6]).abs() * 3600.0,
            (a.values[7] - b.values[7]).abs() * 3600.0,
            (a.values[8] - b.values[8]).abs(),
            (a.values[9] - b.values[9]).abs(),
            (a.values[10] - b.values[10]).abs(),
        ];
        if residuals.iter().any(|value| !value.is_finite()) {
            return Err(format!("row {}: residual overflow", index + 2));
        }
        let passes = residuals.iter().zip(limits).all(|(error, limit)| *error <= limit);
        result.push(Comparison {
            body: a.body.clone(),
            residuals,
            passes,
        });
    }
    Ok(result)
}

fn write_report<W: Write>(out: &mut W, comparison: &[Comparison]) -> io::Result<()> {
    writeln!(out, "{OUTPUT_HEADER}")?;
    for row in comparison {
        let x = row.residuals;
        writeln!(
            out,
            "{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{}",
            row.body, x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7], row.passes
        )?;
    }
    Ok(())
}

fn execute() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 8 {
        return Err(invalid(
            "usage: compare_aberrated_csv <actual.csv> <like-for-like-reference.csv> \
             <angle-arcsec> <distance-AU> <angular-rate-arcsec/TDB-day> \
             <radial-rate-AU/TDB-day> <lighttime-seconds> <lighttime-rate>",
        )
        .into());
    }
    let thresholds = Thresholds::parse(&args[2..])?;
    let actual = parse_csv(&fs::read_to_string(&args[0])?).map_err(invalid)?;
    let reference = parse_csv(&fs::read_to_string(&args[1])?).map_err(invalid)?;
    let comparisons = compare(&actual, &reference, thresholds).map_err(invalid)?;

    // All structural gates succeed before emitting stdout. An error
    // threshold failure is a complete CSV report, followed by exit 1.
    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());
    write_report(&mut output, &comparisons)?;
    output.flush()?;
    if comparisons.iter().any(|r| !r.passes) {
        return Err(invalid("one or more six-field residuals exceeded tolerances").into());
    }
    Ok(())
}

fn main() {
    if let Err(error) = execute() {
        eprintln!("compare_aberrated_csv: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = "Moon,Earth,2461354.500000000000,2461354.49998,0.00000000000000000e0,359.99000000000000000e0,1.00000000000000000e0,2.00000000000000000e0,-1.00000000000000000e0,2.00000000000000000e0,-3.00000000000000000e0,1.00000000000000000e0,-1.00000000000000000e-5,AnalyticDe441Chebyshev,fixed_ICRF_equatorial,LT_plus_annual_SR_aberration";

    fn csv(line: &str) -> String {
        format!("{HEADER}\n{line}\n")
    }
    fn zero() -> Thresholds {
        Thresholds::parse(&["0".to_string(); 6]).unwrap()
    }

    #[test]
    fn identical_data_passes_zero_tolerances_including_duplicate_bodies() {
        let input = csv(&format!("{ONE}\n{ONE}"));
        let parsed = parse_csv(&input).unwrap();
        let results = compare(&parsed, &parsed, zero()).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|x| x.passes && x.residuals == [0.0; 8]));
        let mut out = Vec::new();
        write_report(&mut out, &results).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert_eq!(output.lines().count(), 3);
        assert!(output.contains("longitude_error_arcsec"));
    }

    #[test]
    fn circular_longitude_and_signed_velocity_residuals_are_not_lost() {
        let reference = parse_csv(&csv(ONE)).unwrap();
        let adjusted = ONE
            .replace("359.99000000000000000e0", "1.00000000000000000e-2")
            .replace("-1.00000000000000000e0,2.00000000000000000e0,-3.00000000000000000e0",
                     "-2.00000000000000000e0,2.00000000000000000e0,-3.00000000000000000e0");
        let actual = parse_csv(&csv(&adjusted)).unwrap();
        let loose = Thresholds::parse(&[
            "73".to_string(), "0".to_string(), "3600".to_string(),
            "0".to_string(), "0".to_string(), "0".to_string(),
        ]).unwrap();
        let result = compare(&actual, &reference, loose).unwrap();
        assert!(result[0].passes);
        assert!((result[0].residuals[0] - 72.0).abs() < 1.0e-8);
        assert_eq!(result[0].residuals[3], 3600.0);
        let strict = compare(&actual, &reference, zero()).unwrap();
        assert!(!strict[0].passes);
    }

    #[test]
    fn rejects_frame_corrections_body_and_tdb_epoch_mismatches() {
        let original = csv(ONE);
        for mutated in [
            original.replace("fixed_ICRF_equatorial", "true_ecliptic_of_date"),
            original.replace("LT_plus_annual_SR_aberration", "LT_only"),
            original.replace("AnalyticDe441Chebyshev", "CallerProvided"),
            original.replace("Moon,Earth", "Moon,Sun"),
            original.replace("Moon,Earth", "Earth,Earth"),
            original.replace("0.00000000000000000e0", "1.00000000000000000e-1"),
        ] {
            assert!(parse_csv(&mutated).is_err());
        }
        let reference = parse_csv(&original).unwrap();
        let shifted_epoch = parse_csv(&original.replace(
            "2461354.500000000000", "2461354.600000000000",
        )).unwrap();
        assert!(compare(&shifted_epoch, &reference, zero()).is_err());
        let venus = parse_csv(&original.replace("Moon,Earth", "Venus,Earth")).unwrap();
        assert!(compare(&venus, &reference, zero()).is_err());
    }

    #[test]
    fn rejects_duplicate_header_invalid_numbers_rows_and_timescales() {
        let original = csv(ONE);
        assert!(parse_csv(&original.replace("target,center", "target,observer")).is_err());
        assert!(parse_csv(&format!("{HEADER}\n{HEADER}\n{ONE}")).is_err());
        assert!(parse_csv(&original.replace("2.00000000000000000e0", "NaN")).is_err());
        assert!(parse_csv(&original.replace("2.00000000000000000e0", "Infinity")).is_err());
        assert!(parse_csv(&original.replace("2461354.49998", "2461354.8")).is_err());
        assert!(parse_csv(&format!("{HEADER}\n")).is_err());
        let base = parse_csv(&original).unwrap();
        let other = parse_csv(&csv(&format!("{ONE}\n{ONE}"))).unwrap();
        assert!(compare(&base, &other, zero()).is_err());
        assert!(parse_csv(&csv(&format!(
            "{ONE}\n{}",
            ONE.replace("2461354.500000000000", "2461354.700000000000")
        ))).is_err());
    }

    #[test]
    fn tolerance_values_must_be_finite_and_nonnegative() {
        assert!(Thresholds::parse(&["0".to_string(); 5]).is_err());
        for value in ["NaN", "inf", "-0.001", "not-a-number"] {
            let mut args = ["0".to_string(); 6];
            args[3] = value.to_string();
            assert!(Thresholds::parse(&args).is_err());
        }
    }
}
