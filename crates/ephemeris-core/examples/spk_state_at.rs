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

//! Offline, explicit geometric SPK state query through the canonical Rust core.
//!
//! Modification (2026-11-14): a read-only one-epoch CLI for locally supplied
//! J2000 Type-2/3 SPKs. No automatic kernel acquisition, invented times or
//! scientific accuracy certification. Not an independent NAIF comparison.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Seek, Write};

use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;
use virgostars_ephemeris_core::spk_relative_state::SpkRelativeState;

const HEADER: &str = "et_tdb_seconds,target_id,center_id,common_ancestor_id,\
x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s";

fn usage() -> &'static str {
    "usage: cargo run --offline --locked --example spk_state_at -- \
     --target NAIF_ID --center NAIF_ID --et ET_TDB_SECONDS \
     <kernel1.bsp> [kernel2.bsp ...]\n\
     ET is TDB seconds past J2000; SPKs are loaded in the supplied order.\n\
     For kernel filenames beginning with '-', place '--' before the paths."
}

#[derive(Debug, PartialEq)]
struct Arguments<'a> {
    target_id: i32,
    center_id: i32,
    et: f64,
    paths: Vec<&'a str>,
}

fn parse_arguments(values: &[String]) -> Result<Arguments<'_>, String> {
    let mut target = None;
    let mut center = None;
    let mut et = None;
    let mut paths = Vec::new();
    let mut i = 0;
    while i < values.len() {
        match values[i].as_str() {
            "--target" | "--center" => {
                let kind = values[i].as_str();
                let raw = values
                    .get(i + 1)
                    .ok_or_else(|| format!("{kind} requires a signed i32 NAIF ID"))?;
                let parsed = raw
                    .parse::<i32>()
                    .map_err(|_| format!("invalid {kind} NAIF i32 ID"))?;
                let slot = if kind == "--target" {
                    &mut target
                } else {
                    &mut center
                };
                if slot.replace(parsed).is_some() {
                    return Err(format!("duplicate {kind} option"));
                }
                i += 2;
            }
            "--et" => {
                let raw = values
                    .get(i + 1)
                    .ok_or("--et requires explicit TDB seconds past J2000")?;
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
            "--" => {
                paths.extend(values[i + 1..].iter().map(String::as_str));
                break;
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown option {flag}; {}", usage()))
            }
            path => {
                paths.push(path);
                i += 1;
            }
        }
    }
    let target_id = target.ok_or_else(|| format!("missing --target; {}", usage()))?;
    let center_id = center.ok_or_else(|| format!("missing --center; {}", usage()))?;
    let et = et.ok_or_else(|| format!("missing --et; {}", usage()))?;
    if target_id == center_id {
        return Err(
            "target and center must differ; same-body zeros are not a scientific check".into(),
        );
    }
    if paths.is_empty() || paths.iter().any(|path| path.is_empty()) {
        return Err("at least one explicit local SPK file is required".into());
    }
    Ok(Arguments {
        target_id,
        center_id,
        et,
        paths,
    })
}

fn evaluate_sources<R: Read + Seek>(
    sources: impl IntoIterator<Item = R>,
    target: i32,
    center: i32,
    et: f64,
) -> Result<SpkRelativeState, Box<dyn Error>> {
    let mut kernels = SpkKernelSet::new();
    let mut count = 0;
    for source in sources {
        kernels.load(source)?;
        count += 1;
    }
    if count == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no SPK files supplied").into());
    }
    // Selected highest-priority record and complete requested center chain
    // are validated inside the single canonical Rust SPK implementation.
    Ok(kernels.relative_state(target, center, et)?)
}

fn render<W: Write>(output: &mut W, state: SpkRelativeState) -> io::Result<()> {
    writeln!(output, "{HEADER}")?;
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
    )
}

fn run<W: Write>(values: &[String], output: &mut W) -> Result<(), Box<dyn Error>> {
    let arguments = parse_arguments(values)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    // Do not emit a plausible-looking partial CSV when any kernel fails.
    let files = arguments
        .paths
        .iter()
        .map(File::open)
        .collect::<io::Result<Vec<_>>>()?;
    let state = evaluate_sources(
        files,
        arguments.target_id,
        arguments.center_id,
        arguments.et,
    )?;
    render(output, state)?;
    Ok(())
}

fn main() {
    let values: Vec<String> = env::args().skip(1).collect();
    match run(&values, &mut io::stdout().lock()) {
        Ok(()) => {
            eprintln!(
                "Experimental geometric J2000/ICRF target-minus-center state (km, km/s). \
                 NOT independently JPL/NAIF validated."
            );
        }
        Err(error) => {
            eprintln!("SPK state query failed: {error}");
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

    fn put_i32(bytes: &mut [u8], offset: usize, value: i32, order: SpkByteOrder) {
        let data = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[offset..offset + 4].copy_from_slice(&data);
    }

    fn put_f64(bytes: &mut [u8], offset: usize, value: f64, order: SpkByteOrder) {
        let data = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[offset..offset + 8].copy_from_slice(&data);
    }

    fn synthetic_spk(
        order: SpkByteOrder,
        target: i32,
        center: i32,
        offset: f64,
        slope: f64,
        kind: i32,
    ) -> Vec<u8> {
        let components = if kind == 2 { 3 } else { 6 };
        let record_words = 2 + 2 * components;
        let final_address = FIRST_WORD + record_words + 3;
        let mut bytes = vec![0u8; 5 * RECORD_BYTES];
        bytes[..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(match order {
            SpkByteOrder::LittleEndian => b"LTL-IEEE",
            SpkByteOrder::BigEndian => b"BIG-IEEE",
        });
        put_i32(&mut bytes, 8, 2, order);
        put_i32(&mut bytes, 12, 6, order);
        put_i32(&mut bytes, 76, 3, order);
        put_i32(&mut bytes, 80, 3, order);
        put_i32(&mut bytes, 84, (final_address + 1) as i32, order);
        let summary = 2 * RECORD_BYTES;
        put_f64(&mut bytes, summary + 16, 1.0, order);
        put_f64(&mut bytes, summary + 24, -5.0, order);
        put_f64(&mut bytes, summary + 32, 5.0, order);
        for (i, number) in [
            target,
            center,
            1,
            kind,
            FIRST_WORD as i32,
            final_address as i32,
        ]
        .into_iter()
        .enumerate()
        {
            put_i32(&mut bytes, summary + 40 + i * 4, number, order);
        }
        let base = (FIRST_WORD - 1) * 8;
        put_f64(&mut bytes, base, 0.0, order);
        put_f64(&mut bytes, base + 8, 5.0, order);
        let mut coeffs = vec![[offset, slope], [2.0 * offset, 0.0], [-offset, 0.0]];
        if kind == 3 {
            coeffs.extend_from_slice(&[[slope / 5.0, 0.0], [0.0, 0.0], [0.0, 0.0]]);
        }
        for (component, pair) in coeffs.iter().enumerate() {
            for (i, &value) in pair.iter().enumerate() {
                put_f64(&mut bytes, base + (2 + component * 2 + i) * 8, value, order);
            }
        }
        let footer = (final_address - 4) * 8;
        for (i, value) in [-5.0, 10.0, record_words as f64, 1.0]
            .into_iter()
            .enumerate()
        {
            put_f64(&mut bytes, footer + i * 8, value, order);
        }
        bytes
    }

    #[test]
    fn explicit_arguments_reject_implicit_science_defaults() {
        let cases = [
            vec!["--target", "301", "--et", "0", "a.bsp"],
            vec!["--center", "399", "--et", "0", "a.bsp"],
            vec!["--target", "301", "--center", "399", "a.bsp"],
            vec!["--target", "301", "--center", "399", "--et", "0"],
            vec!["--target", "301", "--center", "301", "--et", "0", "a.bsp"],
            vec![
                "--target", "301", "--target", "399", "--center", "0", "--et", "0", "a.bsp",
            ],
            vec!["--target", "301", "--center", "0", "--et", "inf", "a.bsp"],
            vec!["--target", "301", "--center", "0", "--et", "NaN", "a.bsp"],
            vec![
                "--target",
                "2147483648",
                "--center",
                "0",
                "--et",
                "0",
                "a.bsp",
            ],
            vec![
                "--target", "301", "--center", "0", "--et", "0", "--typo", "a.bsp",
            ],
        ];
        for args in cases {
            let values: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            assert!(parse_arguments(&values).is_err(), "accepted {values:?}");
        }
    }

    #[test]
    fn negative_et_and_dash_prefixed_path_are_preserved() {
        let values = [
            "--target",
            "301",
            "--center",
            "399",
            "--et",
            "-2.5",
            "--",
            "-test.bsp",
        ]
        .map(String::from);
        assert_eq!(
            parse_arguments(&values).unwrap(),
            Arguments {
                target_id: 301,
                center_id: 399,
                et: -2.5,
                paths: vec!["-test.bsp"],
            }
        );
    }

    #[test]
    fn mixed_endian_type2_and_type3_files_compose_only_required_links() {
        for (earth_kind, moon_kind) in [(2, 3), (3, 2)] {
            let earth = Cursor::new(synthetic_spk(
                SpkByteOrder::LittleEndian,
                399,
                0,
                100.0,
                10.0,
                earth_kind,
            ));
            let moon = Cursor::new(synthetic_spk(
                SpkByteOrder::BigEndian,
                301,
                399,
                10.0,
                5.0,
                moon_kind,
            ));
            let state = evaluate_sources(vec![earth, moon], 301, 0, 0.0).unwrap();
            assert_eq!((state.target_id, state.center_id), (301, 0));
            assert_eq!(state.common_ancestor_id, 0);
            assert_eq!(state.position_km, [110.0, 220.0, -110.0]);
            assert_eq!(state.velocity_km_per_second, [3.0, 0.0, 0.0]);

            let mut csv = Vec::new();
            render(&mut csv, state).unwrap();
            let text = String::from_utf8(csv).unwrap();
            let rows: Vec<_> = text.lines().collect();
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0], HEADER);
            assert_eq!(rows[1].split(',').count(), 10);
            assert!(rows[1].starts_with("0.00000000000000000e0,301,0,0,"));
            assert!(rows[1].contains("1.10000000000000000e2"));
        }
    }

    #[test]
    fn missing_chain_is_error_and_never_yields_plausible_csv() {
        let earth = Cursor::new(synthetic_spk(
            SpkByteOrder::LittleEndian,
            399,
            0,
            100.0,
            10.0,
            2,
        ));
        assert!(evaluate_sources(vec![earth], 301, 399, 0.0).is_err());
        let empty: Vec<Cursor<Vec<u8>>> = Vec::new();
        assert!(evaluate_sources(empty, 301, 399, 0.0).is_err());
    }
}
