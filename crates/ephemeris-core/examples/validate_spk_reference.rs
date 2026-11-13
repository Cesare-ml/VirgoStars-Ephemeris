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

//! Offline comparator for independently generated NAIF/SPICE geometric states.
//!
//! The user supplies both ordered SPK kernels and a curated reference CSV;
//! nothing is downloaded or bundled. This validator is only a scientific
//! *tool*: its tests exercise syntax and synthetic residual arithmetic, not
//! agreement with an actual independently computed JPL state.
//!
//! NAIF spkezr contract: <https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/cspice/spkezr_c.html>

use std::collections::{BTreeMap, HashSet};
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;

const CSV_COLUMNS: &str =
    "et_tdb_seconds,target_id,center_id,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s";

#[derive(Debug, PartialEq)]
struct Case {
    et: f64,
    target: i32,
    center: i32,
    expected: [f64; 6],
}

#[derive(Debug)]
struct Reference {
    ordered_kernel_sha256: Vec<String>,
    cases: Vec<Case>,
}

fn read_reference(content: &str) -> Result<Reference, String> {
    let mut metadata = BTreeMap::<String, String>::new();
    let mut cases = Vec::new();
    let mut seen_rows = HashSet::new();
    let mut seen_header = false;
    for (line_index, raw) in content.lines().enumerate() {
        let line_no = line_index + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(raw_metadata) = line.strip_prefix('#') {
            if seen_header {
                return Err(format!("line {line_no}: metadata follows CSV header"));
            }
            let (key, value) = raw_metadata
                .trim()
                .split_once('=')
                .ok_or_else(|| format!("line {line_no}: expected # key=value"))?;
            let (key, value) = (key.trim(), value.trim());
            if key.is_empty() || value.is_empty() {
                return Err(format!("line {line_no}: empty reference metadata"));
            }
            if metadata.insert(key.to_owned(), value.to_owned()).is_some() {
                return Err(format!("line {line_no}: duplicate reference key '{key}'"));
            }
            continue;
        }
        if !seen_header {
            if line != CSV_COLUMNS {
                return Err(format!("line {line_no}: unexpected reference column layout"));
            }
            seen_header = true;
            continue;
        }
        let fields: Vec<_> = line.split(',').map(str::trim).collect();
        if fields.len() != 9 {
            return Err(format!("line {line_no}: exactly nine CSV fields required"));
        }
        let et = fields[0]
            .parse::<f64>()
            .map_err(|_| format!("line {line_no}: invalid TDB/ET value"))?;
        if !et.is_finite() {
            return Err(format!("line {line_no}: non-finite TDB/ET"));
        }
        let target = fields[1]
            .parse::<i32>()
            .map_err(|_| format!("line {line_no}: invalid target NAIF integer ID"))?;
        let center = fields[2]
            .parse::<i32>()
            .map_err(|_| format!("line {line_no}: invalid observer NAIF integer ID"))?;
        if target == center {
            return Err(format!(
                "line {line_no}: identical target/observer is not independent ephemeris evidence"
            ));
        }
        // Treat +0.0 and -0.0 as the same TDB instant for duplicate detection.
        let epoch_bits = if et == 0.0 { 0 } else { et.to_bits() };
        if !seen_rows.insert((target, center, epoch_bits)) {
            return Err(format!(
                "line {line_no}: duplicated target/observer/ET reference case"
            ));
        }
        let mut expected = [0.0; 6];
        for axis in 0..6 {
            let value = fields[axis + 3]
                .parse::<f64>()
                .map_err(|_| format!("line {line_no}: invalid vector component {}", axis + 1))?;
            if !value.is_finite() {
                return Err(format!(
                    "line {line_no}: non-finite vector component {}",
                    axis + 1
                ));
            }
            expected[axis] = value;
        }
        cases.push(Case {
            et,
            target,
            center,
            expected,
        });
    }
    if !seen_header || cases.is_empty() {
        return Err("CSV header and at least one nontrivial reference case required".into());
    }
    for (key, required) in [
        ("provider", "NAIF_SPICE"),
        ("method", "spkezr"),
        ("time_scale", "TDB"),
        ("frame", "J2000"),
        ("corrections", "NONE"),
        ("units", "KM-S"),
    ] {
        if metadata.get(key).map(String::as_str) != Some(required) {
            return Err(format!("reference metadata {key} must equal {required}"));
        }
    }
    for key in ["toolkit_version", "generated_utc", "reference_source"] {
        if !metadata.contains_key(key) {
            return Err(format!("reference metadata {key} is required"));
        }
    }
    let hashes = metadata
        .get("ordered_kernel_sha256")
        .ok_or_else(|| "reference metadata ordered_kernel_sha256 is required".to_string())?;
    let ordered_kernel_sha256: Vec<String> = hashes
        .split(';')
        .map(str::trim)
        .map(str::to_owned)
        .collect();
    if ordered_kernel_sha256.is_empty()
        || ordered_kernel_sha256.iter().any(|digest| {
            digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    {
        return Err("ordered_kernel_sha256 must contain semicolon-separated 64-hex SHA256 digests".into());
    }
    // These are user-declared hashes, NOT cryptographically verified by this
    // dependency-free example. Verify them independently at execution time.
    Ok(Reference {
        ordered_kernel_sha256,
        cases,
    })
}

#[derive(Debug, PartialEq)]
struct Residual {
    signed_position_km: [f64; 3],
    signed_velocity_km_per_second: [f64; 3],
    max_position_component_km: f64,
    max_velocity_component_km_per_second: f64,
    position_norm_km: f64,
    velocity_norm_km_per_second: f64,
}

fn residual(actual_position: [f64; 3], actual_velocity: [f64; 3], expected: [f64; 6]) -> Result<Residual, String> {
    let mut result = Residual {
        signed_position_km: [0.0; 3],
        signed_velocity_km_per_second: [0.0; 3],
        max_position_component_km: 0.0,
        max_velocity_component_km_per_second: 0.0,
        position_norm_km: 0.0,
        velocity_norm_km_per_second: 0.0,
    };
    for axis in 0..3 {
        let dp = actual_position[axis] - expected[axis];
        let dv = actual_velocity[axis] - expected[axis + 3];
        if !dp.is_finite() || !dv.is_finite() {
            return Err("non-finite signed SPK-vs-NAIF component residual".into());
        }
        result.signed_position_km[axis] = dp;
        result.signed_velocity_km_per_second[axis] = dv;
        result.max_position_component_km = result.max_position_component_km.max(dp.abs());
        result.max_velocity_component_km_per_second =
            result.max_velocity_component_km_per_second.max(dv.abs());
    }
    let p = result.signed_position_km;
    let v = result.signed_velocity_km_per_second;
    result.position_norm_km = p[0].hypot(p[1]).hypot(p[2]);
    result.velocity_norm_km_per_second = v[0].hypot(v[1]).hypot(v[2]);
    if !result.position_norm_km.is_finite() || !result.velocity_norm_km_per_second.is_finite() {
        return Err("non-finite SPK-vs-NAIF vector norm".into());
    }
    Ok(result)
}

fn input_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidInput, message.into()))
}

fn tolerance(value: &str, name: &str) -> Result<f64, Box<dyn Error>> {
    let result = value
        .parse::<f64>()
        .map_err(|_| input_error(format!("{name} must be numeric")))?;
    if !result.is_finite() || result <= 0.0 {
        return Err(input_error(format!("{name} must be finite and strictly positive")));
    }
    Ok(result)
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.len() < 4 {
        return Err(input_error(
            "usage: validate_spk_reference <reference.csv> <max-position-component-km> \
             <max-velocity-component-km-per-second> <kernel1.bsp> [kernel2.bsp ...]",
        ));
    }
    let position_limit = tolerance(&arguments[1], "position limit")?;
    let velocity_limit = tolerance(&arguments[2], "velocity limit")?;
    let reference = read_reference(&fs::read_to_string(&arguments[0])?).map_err(input_error)?;
    let paths = &arguments[3..];
    if paths.len() != reference.ordered_kernel_sha256.len() {
        return Err(input_error(format!(
            "reference declares {} ordered kernels but {} paths were supplied",
            reference.ordered_kernel_sha256.len(),
            paths.len()
        )));
    }

    let mut kernels = SpkKernelSet::new();
    for path in paths {
        // Preserve exact declared load order, which changes SPK precedence.
        kernels.load(fs::File::open(path)?)?;
    }

    let mut max_position: f64 = 0.0;
    let mut max_velocity: f64 = 0.0;
    let mut max_position_norm: f64 = 0.0;
    let mut max_velocity_norm: f64 = 0.0;
    let mut failures = 0usize;
    for (index, case) in reference.cases.iter().enumerate() {
        let actual = kernels
            .relative_state(case.target, case.center, case.et)
            .map_err(|error| input_error(format!("case {}: {error}", index + 1)))?;
        let diff = residual(actual.position_km, actual.velocity_km_per_second, case.expected)
            .map_err(input_error)?;
        let exceeded = diff.max_position_component_km > position_limit
            || diff.max_velocity_component_km_per_second > velocity_limit;
        if exceeded {
            failures += 1;
        }
        max_position = max_position.max(diff.max_position_component_km);
        max_velocity = max_velocity.max(diff.max_velocity_component_km_per_second);
        max_position_norm = max_position_norm.max(diff.position_norm_km);
        max_velocity_norm = max_velocity_norm.max(diff.velocity_norm_km_per_second);
        println!(
            "case {}: ET={:.9} target={} center={} dr={:?} km dv={:?} km/s \
             max|dr|={:.9e} km max|dv|={:.9e} km/s {}",
            index + 1,
            case.et,
            case.target,
            case.center,
            diff.signed_position_km,
            diff.signed_velocity_km_per_second,
            diff.max_position_component_km,
            diff.max_velocity_component_km_per_second,
            if exceeded { "FAIL" } else { "PASS" }
        );
    }
    println!(
        "{} cases: worst component |dr|={:.9e} km, |dv|={:.9e} km/s; \
         worst 3D norms |dr|={:.9e} km, |dv|={:.9e} km/s; failures={failures}",
        reference.cases.len(),
        max_position,
        max_velocity,
        max_position_norm,
        max_velocity_norm
    );
    println!(
        "WARNING: reference source and ordered_kernel_sha256 are metadata declarations only; \
         verify original NAIF output, ordered file SHA-256 values and matching kernel versions independently."
    );
    if failures != 0 {
        return Err(input_error("SPK/NAIF reference residual exceeds explicit limits"));
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("SPK reference comparison failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASELINE: &str = concat!(
        "# provider=NAIF_SPICE\n",
        "# method=spkezr\n",
        "# time_scale=TDB\n",
        "# frame=J2000\n",
        "# corrections=NONE\n",
        "# units=KM-S\n",
        "# toolkit_version=N0067\n",
        "# generated_utc=2026-11-13T00:00:00Z\n",
        "# reference_source=synthetic-fixture-not-a-real-spice-output\n",
        "# ordered_kernel_sha256=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
        "et_tdb_seconds,target_id,center_id,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s\n",
        "0,301,399,1,2,3,4,5,6\n",
    );

    fn change(original: &str, replacement: &str) -> String {
        assert!(BASELINE.contains(original), "fixture edit did not match");
        BASELINE.replacen(original, replacement, 1)
    }

    #[test]
    fn parses_strict_reference_and_preserves_signed_values() {
        let parsed = read_reference(BASELINE).unwrap();
        assert_eq!(parsed.cases.len(), 1);
        assert_eq!(parsed.ordered_kernel_sha256.len(), 1);
        assert_eq!(parsed.cases[0].target, 301);
        assert_eq!(parsed.cases[0].center, 399);
        assert_eq!(parsed.cases[0].expected, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn rejects_wrong_frame_time_scale_units_or_corrections() {
        for (from, to) in [
            ("frame=J2000", "frame=ECLIPDATE"),
            ("time_scale=TDB", "time_scale=TT"),
            ("units=KM-S", "units=KM-D"),
            ("corrections=NONE", "corrections=LT"),
            ("method=spkezr", "method=custom"),
            ("provider=NAIF_SPICE", "provider=UNVERIFIED"),
        ] {
            assert!(read_reference(&change(from, to)).is_err(), "{from}");
        }
    }

    #[test]
    fn requires_provenance_and_valid_ordered_hashes() {
        assert!(read_reference(&change(
            "# generated_utc=",
            "# missing_utc=",
        ))
        .is_err());
        assert!(read_reference(&change(
            "ordered_kernel_sha256=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "ordered_kernel_sha256=unknown",
        ))
        .is_err());
        let two = change(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa;bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        assert_eq!(read_reference(&two).unwrap().ordered_kernel_sha256.len(), 2);
    }

    #[test]
    fn refuses_duplicate_or_tautological_rows_and_invalid_numbers() {
        assert!(read_reference(&format!("{BASELINE}0,301,399,1,2,3,4,5,6\n")).is_err());
        assert!(read_reference(&format!("{BASELINE}-0,301,399,1,2,3,4,5,6\n")).is_err());
        for (from, to) in [
            ("0,301,399,1,2,3,4,5,6", "0,301,301,1,2,3,4,5,6"),
            ("0,301,399,1,2,3,4,5,6", "NaN,301,399,1,2,3,4,5,6"),
            ("0,301,399,1,2,3,4,5,6", "0,301.0,399,1,2,3,4,5,6"),
            ("0,301,399,1,2,3,4,5,6", "0,301,399,1,2,3,4,5,inf"),
            ("0,301,399,1,2,3,4,5,6", "0,301,399,1,2,3,4,5"),
        ] {
            assert!(read_reference(&change(from, to)).is_err(), "{to}");
        }
    }

    #[test]
    fn duplicate_metadata_or_late_metadata_never_override_contract() {
        assert!(read_reference(&format!(
            "# frame=J2000\n{BASELINE}"
        ))
        .is_err());
        assert!(read_reference(&format!("{BASELINE}# frame=J2000\n")).is_err());
    }

    #[test]
    fn residuals_keep_six_signed_components_and_numerical_norms() {
        let actual_p = [10.0, -20.0, 30.0];
        let actual_v = [1.0, 2.0, 3.0];
        let expected = [7.0, -18.0, 31.0, 0.0, 3.0, 1.0];
        let r = residual(actual_p, actual_v, expected).unwrap();
        assert_eq!(r.signed_position_km, [3.0, -2.0, -1.0]);
        assert_eq!(r.signed_velocity_km_per_second, [1.0, -1.0, 2.0]);
        assert_eq!(r.max_position_component_km, 3.0);
        assert_eq!(r.max_velocity_component_km_per_second, 2.0);
        assert!((r.position_norm_km - 14.0_f64.sqrt()).abs() < 1e-12);
        assert!((r.velocity_norm_km_per_second - 6.0_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn nonfinite_or_overflowed_residuals_fail_closed() {
        assert!(residual([f64::MAX, 0.0, 0.0], [0.0; 3], [-f64::MAX, 0.0, 0.0, 0.0, 0.0, 0.0]).is_err());
        assert!(residual([f64::MAX, f64::MAX, 0.0], [0.0; 3], [0.0; 6]).is_err());
        assert!(residual([0.0; 3], [f64::INFINITY, 0.0, 0.0], [0.0; 6]).is_err());
    }

    #[test]
    fn tolerances_require_positive_finite_explicit_numbers() {
        for bad in ["0", "-1", "NaN", "inf", "abc"] {
            assert!(tolerance(bad, "position").is_err(), "{bad}");
        }
        assert_eq!(tolerance("0.001", "position").unwrap(), 0.001);
    }

    #[test]
    fn rejects_empty_reference_and_unexpected_columns() {
        assert!(read_reference("").is_err());
        assert!(read_reference(&change(
            CSV_COLUMNS,
            "jd_tdb,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s",
        ))
        .is_err());
    }
}
