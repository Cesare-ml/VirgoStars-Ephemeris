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

//! Offline same-TDB-epoch SPK batch query using the canonical Rust core.
//!
//! Modification (2026-11-14): CLI-only adapter for caller-supplied SPK files.
//! There are no new orbital algorithms, kernel downloads or accuracy claims.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Seek, Write};

use virgostars_ephemeris_core::spk_batch::relative_states_at;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;
use virgostars_ephemeris_core::spk_relative_state::SpkRelativeState;

const HEADER: &str = "et_tdb_seconds,target_id,center_id,common_ancestor_id,\
x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s";

fn usage() -> &'static str {
    "usage: cargo run --offline --locked --example spk_batch_at -- \
     --et ET_TDB_SECONDS --pair TARGET_ID:CENTER_ID [--pair TARGET_ID:CENTER_ID ...] \
     <kernel1.bsp> [kernel2.bsp ...]\n\
     ET is TDB seconds past J2000. Kernels are loaded in the supplied order.\n\
     Repeat --pair to request multiple ordered geometric J2000 states.\n\
     Place '--' before kernel paths that start with '-'."
}

#[derive(Debug, PartialEq)]
struct Arguments<'a> {
    et: f64,
    pairs: Vec<(i32, i32)>,
    paths: Vec<&'a str>,
}

fn parse_pair(raw: &str) -> Result<(i32, i32), String> {
    let (target, center) = raw
        .split_once(':')
        .ok_or_else(|| format!("invalid --pair {raw:?}; expected TARGET_ID:CENTER_ID"))?;
    let target_id = target
        .parse::<i32>()
        .map_err(|_| format!("invalid --pair target NAIF ID: {target:?}"))?;
    let center_id = center
        .parse::<i32>()
        .map_err(|_| format!("invalid --pair center NAIF ID: {center:?}"))?;
    if target_id == center_id {
        return Err(
            "--pair target and center must differ (same-body zeros are not evidence)".into(),
        );
    }
    Ok((target_id, center_id))
}

fn parse_arguments(values: &[String]) -> Result<Arguments<'_>, String> {
    let mut et = None;
    let mut pairs = Vec::new();
    let mut paths = Vec::new();
    let mut index = 0;
    while index < values.len() {
        match values[index].as_str() {
            "--et" => {
                let raw = values
                    .get(index + 1)
                    .ok_or("--et requires TDB seconds past J2000")?;
                let parsed = raw
                    .parse::<f64>()
                    .map_err(|_| "invalid --et numeric TDB epoch")?;
                if !parsed.is_finite() {
                    return Err("--et must be finite TDB seconds past J2000".into());
                }
                if et.replace(parsed).is_some() {
                    return Err("duplicate --et option".into());
                }
                index += 2;
            }
            "--pair" => {
                let raw = values
                    .get(index + 1)
                    .ok_or("--pair requires TARGET_ID:CENTER_ID")?;
                pairs.push(parse_pair(raw)?);
                index += 2;
            }
            "--" => {
                paths.extend(values[index + 1..].iter().map(String::as_str));
                break;
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown option {flag}; {}", usage()));
            }
            path => {
                paths.push(path);
                index += 1;
            }
        }
    }
    let et = et.ok_or_else(|| format!("missing --et; {}", usage()))?;
    if pairs.is_empty() {
        return Err(format!("at least one --pair is required; {}", usage()));
    }
    if paths.is_empty() || paths.iter().any(|path| path.is_empty()) {
        return Err("at least one explicit local SPK file is required".into());
    }
    Ok(Arguments { et, pairs, paths })
}

fn evaluate_sources<R: Read + Seek>(
    sources: impl IntoIterator<Item = R>,
    pairs: &[(i32, i32)],
    et: f64,
) -> Result<Vec<SpkRelativeState>, Box<dyn Error>> {
    let mut kernels = SpkKernelSet::new();
    let mut count = 0;
    for source in sources {
        kernels.load(source)?;
        count += 1;
    }
    if count == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no SPK files supplied").into());
    }
    Ok(relative_states_at(&mut kernels, pairs, et)?)
}

fn render<W: Write>(output: &mut W, states: &[SpkRelativeState]) -> io::Result<()> {
    writeln!(output, "{HEADER}")?;
    for state in states {
        writeln!(
            output,
            "{:.17e},{},{},{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",
            state.tdb_seconds_past_j2000,
            state.target_id,
            state.center_id,
            state.common_ancestor_id,
            state.position_km[0],
            state.position_km[1],
            state.position_km[2],
            state.velocity_km_per_second[0],
            state.velocity_km_per_second[1],
            state.velocity_km_per_second[2],
        )?;
    }
    Ok(())
}

fn evaluate_and_render<R: Read + Seek, W: Write>(
    sources: impl IntoIterator<Item = R>,
    pairs: &[(i32, i32)],
    et: f64,
    output: &mut W,
) -> Result<(), Box<dyn Error>> {
    // All states must succeed before emitting even the CSV header.
    let states = evaluate_sources(sources, pairs, et)?;
    render(output, &states)?;
    Ok(())
}

fn run<W: Write>(values: &[String], output: &mut W) -> Result<(), Box<dyn Error>> {
    let args = parse_arguments(values)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let sources = args
        .paths
        .iter()
        .map(File::open)
        .collect::<io::Result<Vec<_>>>()?;
    evaluate_and_render(sources, &args.pairs, args.et, output)
}

fn main() {
    let values: Vec<String> = env::args().skip(1).collect();
    match run(&values, &mut io::stdout().lock()) {
        Ok(()) => eprintln!(
            "Experimental geometric J2000/ICRF SPK batch (km, km/s); \
             NOT independently validated against NASA/JPL/NAIF."
        ),
        Err(error) => {
            eprintln!("SPK batch query failed: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use virgostars_ephemeris_core::spk_index::SpkByteOrder;

    fn put_i32(bytes: &mut [u8], offset: usize, value: i32, order: SpkByteOrder) {
        let encoded = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[offset..offset + 4].copy_from_slice(&encoded);
    }

    fn put_f64(bytes: &mut [u8], offset: usize, value: f64, order: SpkByteOrder) {
        let encoded = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[offset..offset + 8].copy_from_slice(&encoded);
    }

    fn synthetic_kernel(
        target: i32,
        center: i32,
        position: [f64; 2],
        velocity: [f64; 2],
        order: SpkByteOrder,
    ) -> Cursor<Vec<u8>> {
        // One synthetic Type-2 segment with interval [0, 10] ET, J2000 frame.
        // Coefficients encode linear positions and exact rates in km and km/s.
        let mut bytes = vec![0_u8; 5 * 1024];
        bytes[..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(match order {
            SpkByteOrder::LittleEndian => b"LTL-IEEE",
            SpkByteOrder::BigEndian => b"BIG-IEEE",
        });
        for (offset, value) in [(8, 2), (12, 6), (76, 3), (80, 3), (84, 525)] {
            put_i32(&mut bytes, offset, value, order);
        }
        let summary = 2 * 1024;
        put_f64(&mut bytes, summary + 16, 1.0, order);
        put_f64(&mut bytes, summary + 24, 0.0, order);
        put_f64(&mut bytes, summary + 32, 10.0, order);
        for (index, value) in [target, center, 1, 2, 513, 524].into_iter().enumerate() {
            put_i32(&mut bytes, summary + 40 + index * 4, value, order);
        }
        let record = 4 * 1024;
        put_f64(&mut bytes, record, 5.0, order);
        put_f64(&mut bytes, record + 8, 5.0, order);
        for (index, value) in [
            position[0],
            velocity[0] * 5.0,
            position[1],
            velocity[1] * 5.0,
            0.0,
            0.0,
        ]
        .into_iter()
        .enumerate()
        {
            put_f64(&mut bytes, record + 16 + index * 8, value, order);
        }
        for (index, value) in [0.0, 10.0, 8.0, 1.0].into_iter().enumerate() {
            put_f64(&mut bytes, record + 64 + index * 8, value, order);
        }
        Cursor::new(bytes)
    }

    fn two_sources() -> Vec<Cursor<Vec<u8>>> {
        vec![
            synthetic_kernel(
                399,
                0,
                [100.0, 5.0],
                [3.0, -1.0],
                SpkByteOrder::LittleEndian,
            ),
            synthetic_kernel(301, 399, [2.0, 4.0], [0.1, 0.2], SpkByteOrder::BigEndian),
        ]
    }

    #[test]
    fn parses_explicit_epoch_ordered_pairs_and_dash_prefixed_path() {
        let values = [
            "--et",
            "-2.5",
            "--pair",
            "301:399",
            "--pair",
            "-301:0",
            "--pair",
            "301:399",
            "--",
            "-planet.bsp",
        ]
        .map(str::to_string);
        assert_eq!(
            parse_arguments(&values).unwrap(),
            Arguments {
                et: -2.5,
                pairs: vec![(301, 399), (-301, 0), (301, 399)],
                paths: vec!["-planet.bsp"],
            }
        );
    }

    #[test]
    fn rejects_implicit_science_defaults_and_malformed_requests() {
        let cases: &[&[&str]] = &[
            &["--pair", "301:399", "a.bsp"],
            &["--et", "0", "a.bsp"],
            &["--et", "0", "--pair", "301:399"],
            &["--et", "0", "--et", "1", "--pair", "301:399", "a.bsp"],
            &["--et", "NaN", "--pair", "301:399", "a.bsp"],
            &["--et", "inf", "--pair", "301:399", "a.bsp"],
            &["--et", "0", "--pair", "301:301", "a.bsp"],
            &["--et", "0", "--pair", "0:0", "a.bsp"],
            &["--et", "0", "--pair", "abc:399", "a.bsp"],
            &["--et", "0", "--pair", "301:2147483648", "a.bsp"],
            &["--et", "0", "--pair", "301:399:0", "a.bsp"],
            &["--et", "0", "--pair", "301", "a.bsp"],
            &["--et", "0", "--pair", "301:", "a.bsp"],
            &["--et", "0", "--pair", ":399", "a.bsp"],
            &["--et", "0", "--pair", "301:399", "--typo", "a.bsp"],
        ];
        for case in cases {
            let values: Vec<String> = case.iter().map(|word| (*word).to_string()).collect();
            assert!(parse_arguments(&values).is_err(), "accepted {values:?}");
        }
        let values = ["--et", "0", "--pair", "301:399", "a.bsp", ""].map(str::to_string);
        assert!(parse_arguments(&values).is_err());
    }

    #[test]
    fn mixed_endian_cross_kernel_csv_preserves_pair_order_and_rate_units() {
        let mut csv = Vec::new();
        let pairs = [(301, 0), (399, 0), (301, 399), (301, 0)];
        evaluate_and_render(two_sources(), &pairs, 5.0, &mut csv).unwrap();
        let text = String::from_utf8(csv).unwrap();
        let lines = text.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0], HEADER);
        let rows = lines[1..]
            .iter()
            .map(|line| line.split(',').collect::<Vec<_>>())
            .collect::<Vec<_>>();
        for (index, &(target, center)) in pairs.iter().enumerate() {
            assert_eq!(rows[index].len(), 10);
            assert_eq!(rows[index][1], target.to_string());
            assert_eq!(rows[index][2], center.to_string());
            assert_eq!(rows[index][0].parse::<f64>().unwrap(), 5.0);
        }
        assert_eq!(rows[0], rows[3]);
        assert_eq!(rows[0][3], "0");
        assert_eq!(rows[0][4].parse::<f64>().unwrap(), 102.0);
        assert_eq!(rows[0][5].parse::<f64>().unwrap(), 9.0);
        assert!((rows[0][7].parse::<f64>().unwrap() - 3.1).abs() < 1.0e-12);
        assert!((rows[0][8].parse::<f64>().unwrap() + 0.8).abs() < 1.0e-12);
    }

    #[test]
    fn failed_batch_or_corrupt_kernel_never_writes_partial_csv() {
        let mut csv = Vec::new();
        let error =
            evaluate_and_render(two_sources(), &[(301, 0), (999, 0)], 5.0, &mut csv).unwrap_err();
        assert!(error.to_string().contains("request 1"));
        assert!(csv.is_empty());
        let error =
            evaluate_and_render([Cursor::new(vec![0_u8; 1024])], &[(301, 0)], 5.0, &mut csv)
                .unwrap_err();
        assert!(!error.to_string().is_empty());
        assert!(csv.is_empty());
    }

    #[test]
    fn empty_kernel_sources_are_rejected_without_output() {
        let mut csv = Vec::new();
        let empty: Vec<Cursor<Vec<u8>>> = Vec::new();
        let error = evaluate_and_render(empty, &[(301, 0)], 5.0, &mut csv).unwrap_err();
        assert!(error.to_string().contains("no SPK files supplied"));
        assert!(csv.is_empty());
    }
}
