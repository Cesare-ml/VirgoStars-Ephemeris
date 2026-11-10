/*
EXHIBIT A. Common Public Attribution License Version 1.0.

The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0
*/

//! Offline geometric-state comparison against manually reviewed JPL Horizons vectors.
//! No downloads, reference data or default accuracy claims are bundled.

use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

const COLUMNS: &str = "jd_tdb,x_km,y_km,z_km,vx_km_day,vy_km_day,vz_km_day";

#[derive(Debug)]
struct Case {
    jd_tdb: f64,
    expected: [f64; 6],
}

#[derive(Debug)]
struct Reference {
    target: Body,
    center: Body,
    cases: Vec<Case>,
}

fn body_from_name(name: &str) -> Option<Body> {
    Some(match name {
        "SolarSystemBarycenter" => Body::SolarSystemBarycenter,
        "Mercury" => Body::Mercury,
        "Venus" => Body::Venus,
        "EarthMoonBarycenter" => Body::EarthMoonBarycenter,
        "Earth" => Body::Earth,
        "Moon" => Body::Moon,
        "MarsBarycenter" => Body::MarsBarycenter,
        "JupiterBarycenter" => Body::JupiterBarycenter,
        "SaturnBarycenter" => Body::SaturnBarycenter,
        "UranusBarycenter" => Body::UranusBarycenter,
        "NeptuneBarycenter" => Body::NeptuneBarycenter,
        "PlutoBarycenter" => Body::PlutoBarycenter,
        "Sun" => Body::Sun,
        _ => return None,
    })
}

fn parse_reference(source: &str) -> Result<Reference, String> {
    let mut metadata: BTreeMap<&str, &str> = BTreeMap::new();
    let mut cases = Vec::new();
    let mut seen_columns = false;
    for (index, raw) in source.lines().enumerate() {
        let number = index + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            if seen_columns {
                return Err(format!("line {number}: metadata after CSV header"));
            }
            let (key, value) = comment
                .trim()
                .split_once('=')
                .ok_or_else(|| format!("line {number}: expected # key=value"))?;
            let (key, value) = (key.trim(), value.trim());
            if key.is_empty() || value.is_empty() {
                return Err(format!("line {number}: empty metadata field"));
            }
            if metadata.insert(key, value).is_some() {
                return Err(format!("line {number}: duplicate metadata key {key}"));
            }
            continue;
        }
        if !seen_columns {
            if line != COLUMNS {
                return Err(format!("line {number}: unexpected CSV columns"));
            }
            seen_columns = true;
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 7 {
            return Err(format!("line {number}: expected 7 numeric fields"));
        }
        let mut values = [0.0; 7];
        for (i, field) in fields.iter().enumerate() {
            let value = field
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("line {number}: invalid number at column {}", i + 1))?;
            if !value.is_finite() {
                return Err(format!(
                    "line {number}: non-finite value at column {}",
                    i + 1
                ));
            }
            values[i] = value;
        }
        let mut expected = [0.0; 6];
        expected.copy_from_slice(&values[1..]);
        cases.push(Case {
            jd_tdb: values[0],
            expected,
        });
    }
    if !seen_columns || cases.is_empty() {
        return Err("CSV header and at least one vector row are required".into());
    }

    for (key, expected) in [
        ("provider", "JPL_HORIZONS"),
        ("ephemeris", "DE441"),
        ("time_scale", "TDB"),
        ("frame", "ICRF"),
        ("plane", "FRAME"),
        ("corrections", "NONE"),
        ("units", "KM-D"),
    ] {
        if metadata.get(key).copied() != Some(expected) {
            return Err(format!("metadata {key} must equal {expected}"));
        }
    }
    for key in ["query_url", "retrieved_utc"] {
        if !metadata.contains_key(key) {
            return Err(format!("metadata {key} is required for provenance"));
        }
    }
    let target = body_from_name(
        metadata
            .get("target")
            .ok_or_else(|| "metadata target missing".to_string())?,
    )
    .ok_or_else(|| "metadata target is not a supported body".to_string())?;
    let center = body_from_name(
        metadata
            .get("center")
            .ok_or_else(|| "metadata center missing".to_string())?,
    )
    .ok_or_else(|| "metadata center is not a supported body".to_string())?;
    if target == center {
        return Err("target and center must differ for an independent reference".into());
    }
    Ok(Reference {
        target,
        center,
        cases,
    })
}

/// Signed differences are always computed as Rust DE441 minus Horizons.
#[derive(Debug)]
struct Residuals {
    position_km: [f64; 3],
    velocity_km_per_day: [f64; 3],
    max_position_component_km: f64,
    max_velocity_component_km_per_day: f64,
    position_norm_km: f64,
    velocity_norm_km_per_day: f64,
}

fn residuals(actual_position: [f64; 3], actual_velocity: [f64; 3], expected: [f64; 6]) -> Result<Residuals, String> {
    let mut position_km = [0.0; 3];
    let mut velocity_km_per_day = [0.0; 3];
    let mut max_position_component_km: f64 = 0.0;
    let mut max_velocity_component_km_per_day: f64 = 0.0;
    for axis in 0..3 {
        position_km[axis] = actual_position[axis] - expected[axis];
        velocity_km_per_day[axis] = actual_velocity[axis] - expected[axis + 3];
        if !position_km[axis].is_finite() || !velocity_km_per_day[axis].is_finite() {
            return Err("non-finite signed coordinate or velocity residual".into());
        }
        max_position_component_km = max_position_component_km.max(position_km[axis].abs());
        max_velocity_component_km_per_day =
            max_velocity_component_km_per_day.max(velocity_km_per_day[axis].abs());
    }
    let position_norm_km = position_km[0].hypot(position_km[1]).hypot(position_km[2]);
    let velocity_norm_km_per_day =
        velocity_km_per_day[0].hypot(velocity_km_per_day[1]).hypot(velocity_km_per_day[2]);
    if !position_norm_km.is_finite() || !velocity_norm_km_per_day.is_finite() {
        return Err("non-finite residual vector norm".into());
    }
    Ok(Residuals {
        position_km,
        velocity_km_per_day,
        max_position_component_km,
        max_velocity_component_km_per_day,
        position_norm_km,
        velocity_norm_km_per_day,
    })
}

fn input_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidInput, message.into()))
}

fn positive_tolerance(value: &str, name: &str) -> Result<f64, Box<dyn Error>> {
    let tolerance: f64 = value
        .parse()
        .map_err(|_| input_error(format!("{name} must be a number")))?;
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(input_error(format!("{name} must be finite and positive")));
    }
    Ok(tolerance)
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 4 {
        return Err(input_error(
            "usage: validate_horizons <classic-de441-file> <reference.csv> \
             <max-position-component-error-km> <max-velocity-component-error-km-day>",
        ));
    }
    let position_tolerance = positive_tolerance(&args[2], "position tolerance")?;
    let velocity_tolerance = positive_tolerance(&args[3], "velocity tolerance")?;
    let reference = parse_reference(&fs::read_to_string(&args[1])?).map_err(input_error)?;
    let mut ephemeris = De441::open(&args[0])?;

    let mut largest_position: f64 = 0.0;
    let mut largest_velocity: f64 = 0.0;
    let mut largest_position_norm: f64 = 0.0;
    let mut largest_velocity_norm: f64 = 0.0;
    let mut failing_cases = 0usize;
    for case in &reference.cases {
        let state = ephemeris.state(
            reference.target,
            reference.center,
            JulianDateTdb(case.jd_tdb),
        )?;
        let error = residuals(
            state.position_km,
            state.velocity_km_per_day,
            case.expected,
        )
        .map_err(input_error)?;
        largest_position = largest_position.max(error.max_position_component_km);
        largest_velocity = largest_velocity.max(error.max_velocity_component_km_per_day);
        largest_position_norm = largest_position_norm.max(error.position_norm_km);
        largest_velocity_norm = largest_velocity_norm.max(error.velocity_norm_km_per_day);
        if error.max_position_component_km > position_tolerance
            || error.max_velocity_component_km_per_day > velocity_tolerance
        {
            failing_cases += 1;
        }
        println!(
            "JD TDB {:.9}: signed (DE441 - Horizons) dr = [{:+.9e}, {:+.9e}, {:+.9e}] km; \
             max |dr component| = {:.9e} km; |dr| = {:.9e} km",
            case.jd_tdb,
            error.position_km[0],
            error.position_km[1],
            error.position_km[2],
            error.max_position_component_km,
            error.position_norm_km
        );
        println!(
            "JD TDB {:.9}: signed (DE441 - Horizons) dv = [{:+.9e}, {:+.9e}, {:+.9e}] km/day; \
             max |dv component| = {:.9e} km/day; |dv| = {:.9e} km/day",
            case.jd_tdb,
            error.velocity_km_per_day[0],
            error.velocity_km_per_day[1],
            error.velocity_km_per_day[2],
            error.max_velocity_component_km_per_day,
            error.velocity_norm_km_per_day
        );
    }
    println!(
        "{} cases; worst max-component position {:.9e} km, velocity {:.9e} km/day; \
         worst vector norms {:.9e} km, {:.9e} km/day; failing cases: {}",
        reference.cases.len(),
        largest_position,
        largest_velocity,
        largest_position_norm,
        largest_velocity_norm,
        failing_cases
    );
    if failing_cases != 0 {
        return Err(input_error(
            "reference comparison exceeded explicit tolerances",
        ));
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Reference validation failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = concat!(
        "# provider=JPL_HORIZONS\n",
        "# ephemeris=DE441\n",
        "# time_scale=TDB\n",
        "# frame=ICRF\n",
        "# plane=FRAME\n",
        "# corrections=NONE\n",
        "# units=KM-D\n",
        "# target=Moon\n",
        "# center=Earth\n",
        "# query_url=https://ssd.jpl.nasa.gov/api/horizons.api?example\n",
        "# retrieved_utc=2026-11-10T00:00:00Z\n",
        "jd_tdb,x_km,y_km,z_km,vx_km_day,vy_km_day,vz_km_day\n",
        "2451545.0,1,2,3,4,5,6\n",
    );

    #[test]
    fn parses_strict_geometric_reference_contract() {
        let reference = parse_reference(VALID).unwrap();
        assert_eq!(reference.target, Body::Moon);
        assert_eq!(reference.center, Body::Earth);
        assert_eq!(reference.cases.len(), 1);
        assert_eq!(reference.cases[0].jd_tdb, 2451545.0);
        assert_eq!(reference.cases[0].expected, [1., 2., 3., 4., 5., 6.]);
    }

    #[test]
    fn rejects_mismatched_frame_and_corrected_vectors() {
        assert!(parse_reference(&VALID.replace("# plane=FRAME", "# plane=ECLIPTIC")).is_err());
        assert!(parse_reference(&VALID.replace("# corrections=NONE", "# corrections=LT")).is_err());
        assert!(parse_reference(&VALID.replace("# units=KM-D", "# units=KM-S")).is_err());
    }

    #[test]
    fn rejects_nan_missing_provenance_and_unsupported_body() {
        assert!(parse_reference(&VALID.replace(",1,2,3,4,5,6", ",NaN,2,3,4,5,6")).is_err());
        assert!(parse_reference(&VALID.replace("# retrieved_utc=", "# absent=")).is_err());
        assert!(parse_reference(&VALID.replace("# target=Moon", "# target=PlanetX")).is_err());
        assert!(parse_reference(&VALID.replace("# center=Earth", "# center=Moon")).is_err());
    }

    #[test]
    fn reports_signed_residuals_component_maxima_and_vector_norms() {
        let error = residuals(
            [10.0, -20.0, 30.0],
            [-40.0, 50.0, -60.0],
            [7.0, -18.0, 31.0, -41.0, 48.0, -63.0],
        )
        .unwrap();
        assert_eq!(error.position_km, [3.0, -2.0, -1.0]);
        assert_eq!(error.velocity_km_per_day, [1.0, 2.0, 3.0]);
        assert_eq!(error.max_position_component_km, 3.0);
        assert_eq!(error.max_velocity_component_km_per_day, 3.0);
        assert!((error.position_norm_km - 14.0_f64.sqrt()).abs() < 1e-12);
        assert!((error.velocity_norm_km_per_day - 14.0_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn rejects_non_finite_residuals_and_overflowed_norms() {
        assert!(residuals([f64::MAX, 0.0, 0.0], [0.0; 3], [-f64::MAX, 0.0, 0.0, 0.0, 0.0, 0.0]).is_err());
        assert!(residuals([f64::MAX, f64::MAX, 0.0], [0.0; 3], [0.0; 6]).is_err());
        assert!(residuals([f64::NAN, 0.0, 0.0], [0.0; 3], [0.0; 6]).is_err());
    }

    #[test]
    fn requires_rows_and_positive_finite_tolerances() {
        assert!(parse_reference(&VALID.replace("2451545.0,1,2,3,4,5,6\n", "")).is_err());
        for bad in ["0", "-1", "NaN", "inf", "oops"] {
            assert!(positive_tolerance(bad, "test").is_err());
        }
        assert!(positive_tolerance("1e-6", "test").is_ok());
    }
}
