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
    declared_kernel_files: Option<Vec<(String, u64)>>,
    cases: Vec<Case>,
}

/// Accept canonical, calendar-valid UTC provenance timestamps only.
///
/// This timestamp records *when evidence was produced*, not the TDB/ET epoch
/// used for any astronomical vector. Leap-second notation is deliberately
/// excluded because the generator uses standard UTC datetimes, never :60.
/// Modification (2026-11-14): reject ambiguous or impossible audit dates.
fn valid_provenance_utc(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes.last() != Some(&b'Z')
    {
        return false;
    }
    if bytes.len() == 20 {
        if bytes[19] != b'Z' {
            return false;
        }
    } else if bytes[19] != b'.'
        || bytes[20..bytes.len() - 1].is_empty()
        || !bytes[20..bytes.len() - 1]
            .iter()
            .all(|digit| digit.is_ascii_digit())
    {
        return false;
    }
    let fields = [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)];
    if fields
        .iter()
        .any(|&(start, end)| !bytes[start..end].iter().all(|digit| digit.is_ascii_digit()))
    {
        return false;
    }
    let year: u32 = value[0..4].parse().expect("four ASCII digits");
    let month: u32 = value[5..7].parse().expect("two ASCII digits");
    let day: u32 = value[8..10].parse().expect("two ASCII digits");
    let hour: u32 = value[11..13].parse().expect("two ASCII digits");
    let minute: u32 = value[14..16].parse().expect("two ASCII digits");
    let second: u32 = value[17..19].parse().expect("two ASCII digits");
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    year != 0 && day >= 1 && day <= days_in_month && hour <= 23 && minute <= 59 && second <= 59
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
            // For kernel identity fields, silently trimming a filename or
            // byte count could make a malformed manifest look valid. Preserve
            // the general metadata parser's historical whitespace behavior.
            if matches!(key.trim(), "ordered_kernel_names" | "ordered_kernel_sizes_bytes")
                && (value != value.trim() || raw != raw.trim_end())
            {
                return Err(format!("line {line_no}: whitespace in kernel manifest"));
            }
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
                return Err(format!(
                    "line {line_no}: unexpected reference column layout"
                ));
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
    if !valid_provenance_utc(
        metadata
            .get("generated_utc")
            .expect("generated_utc presence checked above"),
    ) {
        return Err(
            "reference generated_utc must be an actual YYYY-MM-DDTHH:MM:SS[.fraction]Z UTC date"
                .into(),
        );
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
        return Err(
            "ordered_kernel_sha256 must contain semicolon-separated 64-hex SHA256 digests".into(),
        );
    }
    // Modification (2026-11-14): honor optional generator-declared names
    // and byte sizes even when this Rust example runs without Python preflight.
    // Older hash-only reference CSVs remain supported.
    let declared_kernel_files = match (
        metadata.get("ordered_kernel_names"),
        metadata.get("ordered_kernel_sizes_bytes"),
    ) {
        (None, None) => None,
        (Some(names), Some(sizes)) => {
            let names: Vec<_> = names.split(';').collect();
            let sizes: Vec<_> = sizes.split(';').collect();
            if names.len() != ordered_kernel_sha256.len()
                || sizes.len() != ordered_kernel_sha256.len()
            {
                return Err("declared kernel name/size/hash counts differ".into());
            }
            let mut files = Vec::with_capacity(names.len());
            for (name, size) in names.into_iter().zip(sizes) {
                if name.is_empty()
                    || name == "."
                    || name == ".."
                    || name.trim() != name
                    || name.contains('/')
                    || name.contains('\\')
                {
                    return Err("declared kernel names must be plain basenames".into());
                }
                if size.is_empty()
                    || size.len() > 20
                    || !size.bytes().all(|byte| byte.is_ascii_digit())
                    || (size.len() > 1 && size.starts_with('0'))
                {
                    return Err("declared kernel sizes must be canonical decimal bytes".into());
                }
                let size = size
                    .parse::<u64>()
                    .map_err(|_| "declared kernel size exceeds u64 byte range")?;
                files.push((name.to_owned(), size));
            }
            Some(files)
        }
        _ => return Err("declared kernel names and sizes must appear together".into()),
    };
    // These are user-declared hashes, NOT cryptographically verified by this
    // dependency-free example. Verify them independently at execution time.
    Ok(Reference {
        ordered_kernel_sha256,
        declared_kernel_files,
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

fn residual(
    actual_position: [f64; 3],
    actual_velocity: [f64; 3],
    expected: [f64; 6],
) -> Result<Residual, String> {
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

/// Check optional ordered kernel basenames and lengths before scientific I/O.
///
/// The CSV's SHA-256 declarations are not authenticated by this example.
/// Use the separate SHA-256 preflight to verify actual kernel bytes.
fn verify_declared_kernel_files(
    reference: &Reference,
    paths: &[String],
) -> Result<(), Box<dyn Error>> {
    let Some(declared) = &reference.declared_kernel_files else {
        return Ok(());
    };
    if paths.len() != declared.len() {
        return Err(input_error("declared kernel filename/size count mismatch"));
    }
    for (index, (path, (expected_name, expected_size))) in
        paths.iter().zip(declared.iter()).enumerate()
    {
        let basename = std::path::Path::new(path)
            .file_name()
            .and_then(|name| name.to_str());
        if basename != Some(expected_name.as_str()) {
            return Err(input_error(format!(
                "kernel {} filename/load-order mismatch (expected {expected_name})",
                index + 1
            )));
        }
        let size = fs::metadata(path)?.len();
        if size != *expected_size {
            return Err(input_error(format!(
                "kernel {} byte size mismatch (expected {expected_size}, found {size})",
                index + 1
            )));
        }
    }
    Ok(())
}

fn tolerance(value: &str, name: &str) -> Result<f64, Box<dyn Error>> {
    let result = value
        .parse::<f64>()
        .map_err(|_| input_error(format!("{name} must be numeric")))?;
    if !result.is_finite() || result <= 0.0 {
        return Err(input_error(format!(
            "{name} must be finite and strictly positive"
        )));
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

    verify_declared_kernel_files(&reference, paths)?;

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
        let diff = residual(
            actual.position_km,
            actual.velocity_km_per_second,
            case.expected,
        )
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
        return Err(input_error(
            "SPK/NAIF reference residual exceeds explicit limits",
        ));
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
        assert!(read_reference(&change("# generated_utc=", "# missing_utc=",)).is_err());
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

    // Modification (2026-11-14): direct Rust invocation must reject
    // wrong ordered filenames/sizes instead of silently ignoring provenance
    // fields emitted by the independent NAIF/SPICE reference generator.
    fn with_manifest(name: &str, size: &str) -> String {
        BASELINE.replace(
            CSV_COLUMNS,
            &format!(
                "# ordered_kernel_names={name}\n# ordered_kernel_sizes_bytes={size}\n{CSV_COLUMNS}"
            ),
        )
    }

    #[test]
    fn optional_kernel_manifest_accepts_legacy_but_rejects_malformed_declarations() {
        assert!(read_reference(BASELINE)
            .unwrap()
            .declared_kernel_files
            .is_none());
        let parsed = read_reference(&with_manifest("one.bsp", "3")).unwrap();
        assert_eq!(
            parsed.declared_kernel_files,
            Some(vec![("one.bsp".to_string(), 3)])
        );

        for (name, size) in [
            ("", "3"),
            ("../one.bsp", "3"),
            ("dir\\one.bsp", "3"),
            ("one.bsp ", "3"),
            ("one.bsp", ""),
            ("one.bsp", "-1"),
            ("one.bsp", "03"),
            ("one.bsp", "1.5"),
            ("one.bsp", "18446744073709551616"),
            ("one.bsp;two.bsp", "3;4"),
        ] {
            assert!(
                read_reference(&with_manifest(name, size)).is_err(),
                "accepted manifest {name:?} size {size:?}"
            );
        }
        let names_only = BASELINE.replace(
            CSV_COLUMNS,
            &format!("# ordered_kernel_names=one.bsp\n{CSV_COLUMNS}"),
        );
        assert!(read_reference(&names_only).is_err());
        let sizes_only = BASELINE.replace(
            CSV_COLUMNS,
            &format!("# ordered_kernel_sizes_bytes=3\n{CSV_COLUMNS}"),
        );
        assert!(read_reference(&sizes_only).is_err());
    }

    #[test]
    fn direct_rust_preflight_rejects_wrong_name_size_and_load_order() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "virgostars-spk-manifest-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        let file = directory.join("first.bsp");
        fs::write(&file, b"abc").unwrap();
        let paths = vec![file.to_string_lossy().into_owned()];

        let correct = read_reference(&with_manifest("first.bsp", "3")).unwrap();
        verify_declared_kernel_files(&correct, &paths).unwrap();

        let wrong_name = read_reference(&with_manifest("second.bsp", "3")).unwrap();
        assert!(verify_declared_kernel_files(&wrong_name, &paths)
            .unwrap_err()
            .to_string()
            .contains("filename/load-order mismatch"));
        let wrong_size = read_reference(&with_manifest("first.bsp", "4")).unwrap();
        assert!(verify_declared_kernel_files(&wrong_size, &paths)
            .unwrap_err()
            .to_string()
            .contains("byte size mismatch"));
        fs::remove_file(&file).unwrap();
        fs::remove_dir(&directory).unwrap();
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
        assert!(read_reference(&format!("# frame=J2000\n{BASELINE}")).is_err());
        assert!(read_reference(&format!("{BASELINE}# frame=J2000\n")).is_err());
    }

    #[test]
    fn utc_provenance_requires_a_real_calendar_date_and_explicit_z() {
        for valid in [
            "2026-11-14T15:30:42Z",
            "2024-02-29T23:59:59.123456Z",
            "2000-02-29T00:00:00Z",
            "1900-03-01T00:00:00Z",
        ] {
            assert!(valid_provenance_utc(valid), "expected valid: {valid}");
            let changed = change(
                "generated_utc=2026-11-13T00:00:00Z",
                &format!("generated_utc={valid}"),
            );
            assert!(read_reference(&changed).is_ok(), "{valid}");
        }
        for invalid in [
            "2026-02-30T12:00:00Z",
            "2025-02-29T00:00:00Z",
            "1900-02-29T00:00:00Z",
            "0000-01-01T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-11-00T00:00:00Z",
            "2026-11-14T24:00:00Z",
            "2026-11-14T00:60:00Z",
            "2026-11-14T00:00:60Z",
            "2026-11-14T00:00:00+00:00",
            "2026-11-14T00:00:00",
            "2026-11-14 00:00:00Z",
            "2026-11-14T00:00:00.Z",
            "2026-11-14T00:00:00.2xZ",
            "2026-11-14T00:00:00ZZ",
        ] {
            assert!(
                !valid_provenance_utc(invalid),
                "expected invalid: {invalid}"
            );
            let changed = change(
                "generated_utc=2026-11-13T00:00:00Z",
                &format!("generated_utc={invalid}"),
            );
            assert!(read_reference(&changed).is_err(), "{invalid}");
        }
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
        assert!(residual(
            [f64::MAX, 0.0, 0.0],
            [0.0; 3],
            [-f64::MAX, 0.0, 0.0, 0.0, 0.0, 0.0]
        )
        .is_err());
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
