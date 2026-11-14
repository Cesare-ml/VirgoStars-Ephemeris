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

//! Offline SPK target-by-target direct-state diagnostics at a caller-supplied ET.
//!
//! Modification (2026-11-14): render the existing canonical Rust
//! SpkKernelSet::probe_direct_states_at results as a non-atomic CSV report.
//! No invented apparent positions, hidden kernel downloads or JPL claims.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Seek, Write};

use virgostars_ephemeris_core::spk_kernel_set::{SpkDirectStateProbe, SpkKernelSet};

const HEADER: &str = "et_tdb_seconds,target_id,kernel_load_index,segment_center_id,frame_id,\
spk_type,coverage_start_et_tdb_seconds,coverage_end_et_tdb_seconds,status,\
x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s,error";

fn usage() -> &'static str {
    "usage: cargo run --offline --locked --example spk_probe_at -- \
     --et ET_TDB_SECONDS [--target NAIF_ID] <kernel1.bsp> [kernel2.bsp ...]\n\
     ET is finite TDB seconds past J2000, not UTC or a Julian date.\n\
     Later kernels have higher priority; -- separates dash-prefixed paths.\n\
     Stdout is diagnostic CSV: a failed target retains its own error row."
}

#[derive(Debug, PartialEq)]
struct Arguments<'a> {
    et: f64,
    target: Option<i32>,
    paths: Vec<&'a str>,
}

fn parse_arguments(values: &[String]) -> Result<Arguments<'_>, String> {
    let mut et = None;
    let mut target = None;
    let mut paths = Vec::new();
    let mut i = 0;
    while i < values.len() {
        match values[i].as_str() {
            "--et" => {
                let raw = values
                    .get(i + 1)
                    .ok_or("--et requires numeric TDB seconds past J2000")?;
                let parsed = raw
                    .parse::<f64>()
                    .map_err(|_| "invalid --et numeric TDB epoch")?;
                if !parsed.is_finite() {
                    return Err("--et must be finite TDB seconds past J2000".into());
                }
                if et.replace(parsed).is_some() {
                    return Err("duplicate --et option".into());
                }
                i += 2;
            }
            "--target" => {
                let raw = values
                    .get(i + 1)
                    .ok_or("--target requires a signed NAIF i32 ID")?;
                let parsed = raw
                    .parse::<i32>()
                    .map_err(|_| "invalid --target NAIF i32 ID")?;
                if target.replace(parsed).is_some() {
                    return Err("duplicate --target option".into());
                }
                i += 2;
            }
            "--" => {
                paths.extend(values[i + 1..].iter().map(String::as_str));
                break;
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown option {flag}; {}", usage()));
            }
            path => {
                paths.push(path);
                i += 1;
            }
        }
    }
    let et = et.ok_or_else(|| format!("missing --et; {}", usage()))?;
    if paths.is_empty() || paths.iter().any(|p| p.is_empty()) {
        return Err(format!(
            "at least one explicit local SPK file is required; {}",
            usage()
        ));
    }
    Ok(Arguments { et, target, paths })
}

fn quoted_csv(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn render<W: Write>(
    output: &mut W,
    et: f64,
    probes: &[(i32, SpkDirectStateProbe)],
) -> io::Result<(usize, usize)> {
    writeln!(output, "{HEADER}")?;
    let mut failures = 0;
    for (target_id, probe) in probes {
        let descriptor = probe.descriptor;
        write!(
            output,
            "{et:.17e},{target_id},{},{},{},{},{:.17e},{:.17e}",
            probe.kernel_index,
            descriptor.center_id,
            descriptor.frame_id,
            descriptor.spk_type,
            descriptor.start_tdb_seconds,
            descriptor.end_tdb_seconds
        )?;
        match &probe.evaluation {
            Ok(state) => {
                write!(output, ",ok")?;
                for value in state
                    .position_km
                    .iter()
                    .chain(state.velocity_km_per_second.iter())
                {
                    write!(output, ",{value:.17e}")?;
                }
                writeln!(output, ",")?;
            }
            Err(error) => {
                failures += 1;
                // Six empty vector components followed by an always-quoted
                // error field. Do not forge numerical zeros on failed reads.
                writeln!(output, ",error,,,,,,,{}", quoted_csv(&error.to_string()))?;
            }
        }
    }
    Ok((probes.len(), failures))
}

fn evaluate_and_render<R: Read + Seek, W: Write>(
    sources: impl IntoIterator<Item = R>,
    et: f64,
    target: Option<i32>,
    output: &mut W,
) -> Result<(usize, usize), Box<dyn Error>> {
    if !et.is_finite() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "nonfinite TDB ET").into());
    }
    let mut kernels = SpkKernelSet::new();
    let mut count = 0;
    for source in sources {
        kernels.load(source)?;
        count += 1;
    }
    if count == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no SPK files supplied").into());
    }
    // All header/index and explicit target checks precede any stdout bytes.
    let probes = if let Some(id) = target {
        if kernels.declared_target_ids().binary_search(&id).is_err() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("NAIF target {id} is not declared in the supplied kernels"),
            )
            .into());
        }
        kernels
            .probe_direct_state(id, et)?
            .map(|probe| vec![(id, probe)])
            .unwrap_or_default()
    } else {
        kernels.probe_direct_states_at(et)?
    };
    Ok(render(output, et, &probes)?)
}

fn run<W: Write>(values: &[String], output: &mut W) -> Result<(usize, usize), Box<dyn Error>> {
    let args = parse_arguments(values)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    // Open all files before writing the CSV header. Read failures during
    // per-target evaluation remain in their selected error rows.
    let sources = args
        .paths
        .iter()
        .map(File::open)
        .collect::<io::Result<Vec<_>>>()?;
    evaluate_and_render(sources, args.et, args.target, output)
}

fn main() {
    let values: Vec<String> = env::args().skip(1).collect();
    match run(&values, &mut io::stdout().lock()) {
        Ok((rows, failures)) => {
            eprintln!(
                "SPK direct-state diagnostics: {rows} covered targets, \
                 {failures} evaluation errors. Geometry is J2000/ICRF in km, \
                 km/s. Not independent NASA/JPL/NAIF validation."
            );
            if failures > 0 {
                std::process::exit(2);
            }
        }
        Err(error) => {
            eprintln!("SPK diagnostic preflight failed: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use virgostars_ephemeris_core::spk_index::SpkByteOrder;

    const RECORD_BYTES: usize = 1024;
    const FIRST_WORD: usize = 513;

    fn put_i32(bytes: &mut [u8], at: usize, value: i32, order: SpkByteOrder) {
        let word = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[at..at + 4].copy_from_slice(&word);
    }

    fn put_f64(bytes: &mut [u8], at: usize, value: f64, order: SpkByteOrder) {
        let word = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[at..at + 8].copy_from_slice(&word);
    }

    // Entirely synthetic one-segment Type-2 SPK with a constant finite XYZ
    // geometric position, or an intentionally unsupported frame.
    fn kernel(order: SpkByteOrder, target: i32, center: i32, frame: i32, x: f64) -> Vec<u8> {
        let record_words = 8usize;
        let last = FIRST_WORD + record_words + 3;
        let mut bytes = vec![0u8; 5 * RECORD_BYTES];
        bytes[..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(match order {
            SpkByteOrder::LittleEndian => b"LTL-IEEE",
            SpkByteOrder::BigEndian => b"BIG-IEEE",
        });
        for (at, n) in [(8, 2), (12, 6), (76, 3), (80, 3), (84, (last + 1) as i32)] {
            put_i32(&mut bytes, at, n, order);
        }
        let summary = 2 * RECORD_BYTES;
        for (at, value) in [(16, 1.0), (24, 0.0), (32, 10.0)] {
            put_f64(&mut bytes, summary + at, value, order);
        }
        for (i, number) in [target, center, frame, 2, FIRST_WORD as i32, last as i32]
            .into_iter()
            .enumerate()
        {
            put_i32(&mut bytes, summary + 40 + i * 4, number, order);
        }
        let record = (FIRST_WORD - 1) * 8;
        put_f64(&mut bytes, record, 5.0, order);
        put_f64(&mut bytes, record + 8, 5.0, order);
        for (axis, value) in [x, x * 2.0, -x].into_iter().enumerate() {
            put_f64(&mut bytes, record + (2 + axis * 2) * 8, value, order);
            put_f64(&mut bytes, record + (3 + axis * 2) * 8, 0.0, order);
        }
        let footer = (last - 4) * 8;
        for (i, value) in [0.0, 10.0, record_words as f64, 1.0]
            .into_iter()
            .enumerate()
        {
            put_f64(&mut bytes, footer + i * 8, value, order);
        }
        bytes
    }

    fn evaluate(
        files: Vec<Vec<u8>>,
        et: f64,
        target: Option<i32>,
    ) -> Result<(String, (usize, usize)), Box<dyn Error>> {
        let mut output = Vec::new();
        let counts =
            evaluate_and_render(files.into_iter().map(Cursor::new), et, target, &mut output)?;
        Ok((String::from_utf8(output)?, counts))
    }

    #[test]
    fn options_require_explicit_finite_tdb_and_local_kernels() {
        let invalid = [
            vec![],
            vec!["--et", "0"],
            vec!["file.bsp"],
            vec!["--et", "nan", "file.bsp"],
            vec!["--et", "inf", "file.bsp"],
            vec!["--et", "0", "--et", "2", "file.bsp"],
            vec!["--et", "0", "--target", "4", "--target", "5", "file.bsp"],
            vec!["--et", "0", "--target", "2147483648", "file.bsp"],
            vec!["--et", "0", "--unknown", "file.bsp"],
        ];
        for args in invalid {
            let args: Vec<String> = args.into_iter().map(str::to_owned).collect();
            assert!(parse_arguments(&args).is_err(), "accepted {args:?}");
        }
        let args = ["--et", "-2.5", "--target", "-82", "--", "-local.bsp"].map(str::to_owned);
        assert_eq!(
            parse_arguments(&args).unwrap(),
            Arguments {
                et: -2.5,
                target: Some(-82),
                paths: vec!["-local.bsp"]
            }
        );
    }

    #[test]
    fn report_retains_valid_states_and_unsupported_overlay_errors_in_sorted_rows() {
        let older = vec![
            kernel(SpkByteOrder::LittleEndian, 301, 399, 1, 2.0),
            kernel(SpkByteOrder::BigEndian, -82, 0, 1, 8.0),
            kernel(SpkByteOrder::LittleEndian, 399, 0, 1, 3.0),
        ];
        let newer = kernel(SpkByteOrder::BigEndian, 301, 4, 17, 99.0);
        let files = older.into_iter().chain([newer]).collect();
        let (csv, count) = evaluate(files, 5.0, None).unwrap();
        assert_eq!(count, (3, 1));
        let rows: Vec<_> = csv.lines().collect();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0], HEADER);
        assert_eq!(rows[1].split(',').nth(1), Some("-82"));
        assert_eq!(rows[2].split(',').nth(1), Some("301"));
        assert_eq!(rows[3].split(',').nth(1), Some("399"));
        assert!(rows[1].contains(",ok,"));
        assert!(rows[2].contains(",error,,,,,,,\"unsupported SPK frame ID 17\""));
        assert!(rows[2].starts_with("5.00000000000000000e0,301,3,4,17,2,"));
        assert!(rows[3].contains(",ok,"));
        assert!(rows[1].contains("8.00000000000000000e0"));
        assert_eq!(rows[2].split(',').count(), 16);
        assert_eq!(rows[3].split(',').count(), 16);
    }

    #[test]
    fn corrupt_record_is_not_replaced_with_valid_older_data() {
        let good = kernel(SpkByteOrder::LittleEndian, 301, 399, 1, 2.0);
        let mut bad = kernel(SpkByteOrder::BigEndian, 301, 4, 1, 99.0);
        let last = FIRST_WORD + 8 + 3;
        // Corrupt the RSIZE word in the segment's footer, not DAF metadata.
        put_f64(&mut bad, (last - 4) * 8 + 16, 0.0, SpkByteOrder::BigEndian);
        let (csv, counts) = evaluate(vec![good, bad], 5.0, Some(301)).unwrap();
        assert_eq!(counts, (1, 1));
        assert!(csv.contains(",301,1,4,1,2,"));
        assert!(csv.contains("invalid SPK segment: invalid SPK record size"));
        assert!(!csv.contains(",ok,"));
    }

    #[test]
    fn no_epoch_coverage_is_header_only_and_target_filter_is_strict() {
        let data = kernel(SpkByteOrder::LittleEndian, 301, 399, 1, 2.0);
        let (csv, counts) = evaluate(vec![data.clone()], 11.0, None).unwrap();
        assert_eq!(counts, (0, 0));
        assert_eq!(csv, format!("{HEADER}\n"));

        let (csv, counts) = evaluate(vec![data.clone()], 11.0, Some(301)).unwrap();
        assert_eq!(counts, (0, 0));
        assert_eq!(csv, format!("{HEADER}\n"));

        let mut buffer = Vec::new();
        assert!(evaluate_and_render([Cursor::new(data)], 5.0, Some(777), &mut buffer).is_err());
        assert!(buffer.is_empty());
    }

    #[test]
    fn invalid_file_and_nonfinite_epoch_do_not_emit_csv() {
        let mut buffer = Vec::new();
        assert!(
            evaluate_and_render([Cursor::new(vec![0u8; 1024])], 5.0, None, &mut buffer).is_err()
        );
        assert!(buffer.is_empty());
        assert!(evaluate_and_render(
            [Cursor::new(kernel(
                SpkByteOrder::LittleEndian,
                301,
                0,
                1,
                2.0
            ))],
            f64::INFINITY,
            None,
            &mut buffer
        )
        .is_err());
        assert!(buffer.is_empty());
        let no_files: Vec<Cursor<Vec<u8>>> = Vec::new();
        assert!(evaluate_and_render(no_files, 5.0, None, &mut buffer).is_err());
        assert!(buffer.is_empty());
    }

    #[test]
    fn csv_error_field_quotes_commas_and_quotes() {
        assert_eq!(
            quoted_csv("unsupported, \"bad\""),
            "\"unsupported, \"\"bad\"\"\""
        );
    }
}
