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

//! Query one geometric target-minus-center SPK state using the canonical Rust core.
//!
//! Input ET is TDB seconds past J2000; output frame is NAIF 1 (J2000/ICRF),
//! position km and velocity km/TDB-second. Files are in load order, last wins.
//! No network, kernel downloads, independently validated JPL parity,
//! light-time, aberration, topocentric correction or non-J2000 frames.
//!
//! New (2026-11-14): read-only CLI; no second interpolation implementation.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Seek, Write};

use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;
use virgostars_ephemeris_core::spk_relative_state::{SpkRelativeError, SpkRelativeState};

const HEADER: &str = "et_tdb_seconds,target_id,center_id,common_ancestor_id,frame_id,\
x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s";

fn usage() -> &'static str {
    "usage: cargo run --offline --locked --example spk_at -- \
     --target NAIF_ID --center NAIF_ID --at-et ET_TDB_SECONDS \
     <kernel1.bsp> [kernel2.bsp ...]\n\
     All files are local and loaded in given order (last file wins). \
     Add -- before filenames beginning with a hyphen."
}

#[derive(Debug, PartialEq)]
struct Arguments {
    target_id: i32,
    center_id: i32,
    et: f64,
    files: Vec<String>,
}

fn arguments(values: &[String]) -> Result<Arguments, String> {
    let mut target_id = None;
    let mut center_id = None;
    let mut et = None;
    let mut files = Vec::new();
    let mut i = 0;
    while i < values.len() {
        match values[i].as_str() {
            "--target" | "--center" => {
                let name = values[i].as_str();
                let slot = if name == "--target" {
                    &mut target_id
                } else {
                    &mut center_id
                };
                if slot.is_some() {
                    return Err(format!("duplicate {name}"));
                }
                let raw = values
                    .get(i + 1)
                    .ok_or_else(|| format!("{name} requires a NAIF i32 ID"))?;
                *slot = Some(
                    raw.parse::<i32>()
                        .map_err(|_| format!("invalid {name} NAIF i32 ID"))?,
                );
                i += 2;
            }
            "--at-et" => {
                if et.is_some() {
                    return Err("duplicate --at-et".into());
                }
                let raw = values
                    .get(i + 1)
                    .ok_or("--at-et requires finite TDB seconds past J2000")?;
                let parsed = raw.parse::<f64>().map_err(|_| "invalid --at-et epoch")?;
                if !parsed.is_finite() {
                    return Err("--at-et must be finite".into());
                }
                et = Some(parsed);
                i += 2;
            }
            "--" => {
                files.extend(values[i + 1..].iter().cloned());
                break;
            }
            option if option.starts_with('-') => {
                return Err(format!("unexpected option {option}; {}", usage()));
            }
            _ => {
                files.push(values[i].clone());
                i += 1;
            }
        }
    }
    if files.is_empty() || files.iter().any(String::is_empty) {
        return Err(format!(
            "at least one local SPK kernel path required; {}",
            usage()
        ));
    }
    Ok(Arguments {
        target_id: target_id.ok_or("missing --target")?,
        center_id: center_id.ok_or("missing --center")?,
        et: et.ok_or("missing --at-et")?,
        files,
    })
}

fn evaluate<R: Read + Seek>(
    kernels: &mut SpkKernelSet<R>,
    request: &Arguments,
) -> Result<SpkRelativeState, SpkRelativeError> {
    kernels.relative_state(request.target_id, request.center_id, request.et)
}

fn render<W: Write>(output: &mut W, result: SpkRelativeState) -> io::Result<()> {
    // Both CSV lines are written only AFTER the kernel-set query succeeded.
    writeln!(output, "{HEADER}")?;
    writeln!(
        output,
        "{:.17e},{},{},{},1,{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",
        result.tdb_seconds_past_j2000,
        result.target_id,
        result.center_id,
        result.common_ancestor_id,
        result.position_km[0],
        result.position_km[1],
        result.position_km[2],
        result.velocity_km_per_second[0],
        result.velocity_km_per_second[1],
        result.velocity_km_per_second[2],
    )
}

fn run<W: Write>(values: &[String], output: &mut W) -> Result<(), Box<dyn Error>> {
    let request = arguments(values)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let mut kernels = SpkKernelSet::new();
    for path in &request.files {
        kernels.load(File::open(path)?)?;
    }
    let result = evaluate(&mut kernels, &request)?;
    render(output, result)?;
    Ok(())
}

fn main() {
    let values: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = run(&values, &mut io::stdout().lock()) {
        eprintln!("SPK geometric state query failed: {error}");
        std::process::exit(1);
    }
    eprintln!(
        "Experimental geometric J2000/ICRF state; results are NOT independently \
         validated against JPL/NAIF."
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| (*v).to_owned()).collect()
    }

    #[test]
    fn strict_cli_requires_explicit_target_center_et_and_kernel_paths() {
        let parsed = arguments(&args(&[
            "--target", "301", "--center", "399", "--at-et", "-86400", "old.bsp", "new.bsp",
        ]))
        .unwrap();
        assert_eq!(parsed.target_id, 301);
        assert_eq!(parsed.center_id, 399);
        assert_eq!(parsed.et, -86_400.0);
        assert_eq!(parsed.files, vec!["old.bsp", "new.bsp"]);

        for bad in [
            args(&[]),
            args(&["--target", "301", "--center", "399", "file.bsp"]),
            args(&["--target", "301", "--at-et", "0", "file.bsp"]),
            args(&["--center", "399", "--at-et", "0", "file.bsp"]),
            args(&["--target", "301", "--center", "399", "--at-et", "0"]),
            args(&[
                "--target", "301", "--target", "302", "--center", "399", "--at-et", "0", "file.bsp",
            ]),
            args(&[
                "--target", "301", "--center", "399", "--at-et", "NaN", "file.bsp",
            ]),
            args(&[
                "--target", "301", "--center", "399", "--at-et", "inf", "file.bsp",
            ]),
            args(&[
                "--target",
                "not-an-id",
                "--center",
                "399",
                "--at-et",
                "0",
                "file.bsp",
            ]),
            args(&[
                "--target",
                "301",
                "--center",
                "399",
                "--at-et",
                "0",
                "--unknown",
            ]),
        ] {
            assert!(arguments(&bad).is_err(), "bad arguments accepted: {bad:?}");
        }
        let escaped = arguments(&args(&[
            "--target",
            "-1",
            "--center",
            "0",
            "--at-et",
            "0",
            "--",
            "-local.bsp",
        ]))
        .unwrap();
        assert_eq!(escaped.target_id, -1);
        assert_eq!(escaped.files, vec!["-local.bsp"]);
    }

    fn put_i32(bytes: &mut [u8], offset: usize, value: i32, big: bool) {
        let encoded = if big {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        };
        bytes[offset..offset + 4].copy_from_slice(&encoded);
    }

    fn put_f64(bytes: &mut [u8], offset: usize, value: f64, big: bool) {
        let encoded = if big {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        };
        bytes[offset..offset + 8].copy_from_slice(&encoded);
    }

    fn synthetic_spk(
        target: i32,
        center: i32,
        position: f64,
        speed: f64,
        big: bool,
    ) -> Cursor<Vec<u8>> {
        // Fully synthetic Type-2 constant+linear Chebyshev record (one axis).
        // DAF record 3 summary, record 4 names, record 5 coefficient block.
        let mut bytes = vec![0_u8; 5 * 1024];
        bytes[..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(if big { b"BIG-IEEE" } else { b"LTL-IEEE" });
        for (offset, value) in [(8, 2), (12, 6), (76, 3), (80, 3), (84, 525)] {
            put_i32(&mut bytes, offset, value, big);
        }
        let summary = 2 * 1024;
        put_f64(&mut bytes, summary + 16, 1.0, big); // summary count
        put_f64(&mut bytes, summary + 24, 0.0, big); // start ET
        put_f64(&mut bytes, summary + 32, 10.0, big); // end ET
        for (i, value) in [target, center, 1, 2, 513, 524].into_iter().enumerate() {
            put_i32(&mut bytes, summary + 40 + i * 4, value, big);
        }
        let record = 4 * 1024;
        put_f64(&mut bytes, record, 5.0, big); // midpoint ET
        put_f64(&mut bytes, record + 8, 5.0, big); // radius in seconds
        put_f64(&mut bytes, record + 16, position, big);
        put_f64(&mut bytes, record + 24, 5.0 * speed, big); // T1 coefficient gives km/s
        put_f64(&mut bytes, record + 32, 4.0, big); // Y constant
        for (i, value) in [0.0, 10.0, 8.0, 1.0].into_iter().enumerate() {
            put_f64(&mut bytes, record + 64 + i * 8, value, big); // footer
        }
        Cursor::new(bytes)
    }

    #[test]
    fn computes_cross_kernel_state_and_velocity_with_both_byte_orders() {
        let request = arguments(&args(&[
            "--target",
            "301",
            "--center",
            "0",
            "--at-et",
            "5",
            "earlier.bsp",
            "later.bsp",
        ]))
        .unwrap();
        let mut kernels = SpkKernelSet::new();
        kernels
            .load(synthetic_spk(399, 0, 100.0, 2.0, false))
            .unwrap();
        kernels
            .load(synthetic_spk(301, 399, 3.0, 0.5, true))
            .unwrap();

        let result = evaluate(&mut kernels, &request).unwrap();
        assert_eq!(result.common_ancestor_id, 0);
        assert_eq!(result.position_km, [103.0, 8.0, 0.0]);
        assert!((result.velocity_km_per_second[0] - 2.5).abs() < 1e-14);
        let mut csv = Vec::new();
        render(&mut csv, result).unwrap();
        let text = String::from_utf8(csv).unwrap();
        let rows: Vec<_> = text.lines().collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], HEADER);
        let fields: Vec<_> = rows[1].split(',').collect();
        assert_eq!(fields.len(), 11);
        assert_eq!(
            (fields[1], fields[2], fields[3], fields[4]),
            ("301", "0", "0", "1")
        );
        assert_eq!(fields[5].parse::<f64>().unwrap(), 103.0);
        assert_eq!(fields[8].parse::<f64>().unwrap(), 2.5);
    }

    #[test]
    fn fails_closed_for_unconnected_or_unsupported_selected_links() {
        let request = arguments(&args(&[
            "--target",
            "301",
            "--center",
            "0",
            "--at-et",
            "5",
            "example.bsp",
        ]))
        .unwrap();
        let mut kernels = SpkKernelSet::new();
        kernels
            .load(synthetic_spk(301, 399, 3.0, 0.5, false))
            .unwrap();
        assert!(matches!(
            evaluate(&mut kernels, &request),
            Err(SpkRelativeError::Disconnected { .. })
        ));
        let mut output = Vec::new();
        assert!(run(
            &args(&[
                "--target",
                "301",
                "--center",
                "0",
                "--at-et",
                "5",
                "does-not-exist.bsp",
            ]),
            &mut output
        )
        .is_err());
        assert!(output.is_empty());
    }

    #[test]
    fn reject_nonfinite_query_before_writing_csv() {
        let mut output = Vec::new();
        assert!(run(
            &args(&["--target", "301", "--center", "0", "--at-et", "NaN", "x.bsp"]),
            &mut output
        )
        .is_err());
        assert!(output.is_empty());
    }
}
