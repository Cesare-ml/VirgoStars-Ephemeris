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

//! Offline descriptor-priority probes immediately around declared SPK boundaries.
//!
//! Metadata only: delegates both declared coverage and winner selection to
//! the canonical Rust SpkKernelSet. This never interpolates a state,
//! downloads data, or claims scientific agreement with NAIF/JPL.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Seek, Write};

use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;

const HEADER: &str = "boundary_et_tdb_seconds,probe,probe_et_tdb_seconds,load_index,center_id,frame_id,spk_type,candidate_j2000_type2_or3";

fn usage() -> &'static str {
    "usage: cargo run --offline --locked --example spk_boundary_probes -- \
     --target NAIF_ID <kernel1.bsp> [kernel2.bsp ...]\n\
     All epochs are TDB/ET seconds past J2000. Kernels are given in load order."
}

/// Parse only explicit target IDs and local files; there is no implicit epoch.
fn arguments(values: &[String]) -> Result<(i32, Vec<&str>), String> {
    let mut target = None;
    let mut files = Vec::new();
    let mut i = 0;
    while i < values.len() {
        match values[i].as_str() {
            "--target" => {
                if target.is_some() {
                    return Err("duplicate --target".to_string());
                }
                let value = values
                    .get(i + 1)
                    .ok_or_else(|| "--target requires an NAIF i32 ID".to_string())?;
                target = Some(
                    value
                        .parse::<i32>()
                        .map_err(|_| "invalid --target NAIF i32 ID".to_string())?,
                );
                i += 2;
            }
            "--" => {
                files.extend(values[i + 1..].iter().map(String::as_str));
                break;
            }
            option if option.starts_with('-') => {
                return Err(format!("unexpected argument {option}; {}", usage()));
            }
            _ => {
                files.push(values[i].as_str());
                i += 1;
            }
        }
    }
    let target = target.ok_or_else(|| format!("--target required; {}", usage()))?;
    if files.is_empty() || files.iter().any(|path| path.is_empty()) {
        return Err(format!("explicit local SPK file required; {}", usage()));
    }
    Ok((target, files))
}

/// Immediately adjacent representable binary64 ET, not an arbitrary offset.
///
/// An adjacent finite number may be too close to a boundary for meaningful
/// Chebyshev/physical comparisons: only descriptor selection is inspected.
fn neighbor(et: f64, after: bool) -> Option<f64> {
    debug_assert!(et.is_finite());
    if et == 0.0 {
        return Some(if after {
            f64::from_bits(1)
        } else {
            -f64::from_bits(1)
        });
    }
    let bits = et.to_bits();
    let next_bits = if (et > 0.0) == after {
        bits + 1
    } else {
        bits - 1
    };
    let adjacent = f64::from_bits(next_bits);
    adjacent.is_finite().then_some(adjacent)
}

fn render<W: Write, R: Read + Seek>(
    output: &mut W,
    kernels: &SpkKernelSet<R>,
    target: i32,
) -> io::Result<usize> {
    let boundaries = kernels.declared_coverage_boundaries_for_target(target);
    if boundaries.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no SPK descriptors declare coverage for this target",
        ));
    }

    writeln!(output, "{HEADER}")?;
    let mut count = 0;
    for boundary in boundaries {
        let probes = [
            ("before", neighbor(boundary, false)),
            ("at", Some(boundary)),
            ("after", neighbor(boundary, true)),
        ];
        for (label, sample) in probes {
            let Some(et) = sample else {
                continue; // At +/-f64::MAX, the outside neighbor is infinite.
            };
            let winner = kernels
                .selected_segment(target, et)
                .map_err(io::Error::other)?;
            if let Some((index, descriptor)) = winner {
                let candidate = descriptor.frame_id == 1 && matches!(descriptor.spk_type, 2 | 3);
                writeln!(
                    output,
                    "{boundary:.17e},{label},{et:.17e},{},{},{},{},{}",
                    index + 1,
                    descriptor.center_id,
                    descriptor.frame_id,
                    descriptor.spk_type,
                    candidate
                )?;
            } else {
                // Exactly five empty metadata fields: unknown is not zero.
                writeln!(output, "{boundary:.17e},{label},{et:.17e},,,,,")?;
            }
            count += 1;
        }
    }
    Ok(count)
}

fn run<W: Write>(args: &[String], output: &mut W) -> Result<usize, Box<dyn Error>> {
    let (target, paths) =
        arguments(args).map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;

    // Validate ALL kernels before writing any CSV row/header.
    let mut kernels = SpkKernelSet::new();
    for path in paths {
        kernels.load(File::open(path)?)?;
    }
    Ok(render(output, &kernels, target)?)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args, &mut io::stdout().lock()) {
        Ok(rows) => eprintln!(
            "Emitted {rows} metadata-only SPK priority probes; no coefficient, chain, or NAIF accuracy validation."
        ),
        Err(error) => {
            eprintln!("SPK boundary probe failed: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn put_i32(bytes: &mut [u8], at: usize, value: i32, big: bool) {
        let encoded = if big {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        };
        bytes[at..at + 4].copy_from_slice(&encoded);
    }

    fn put_f64(bytes: &mut [u8], at: usize, value: f64, big: bool) {
        let encoded = if big {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        };
        bytes[at..at + 8].copy_from_slice(&encoded);
    }

    // Minimal, synthetic five-record DAF. Coefficients are deliberately NOT
    // used, since this example tests metadata precedence, not positions.
    fn synthetic_spk(big: bool, target: i32, center: i32, start: f64, end: f64) -> Vec<u8> {
        let mut bytes = vec![0u8; 5 * 1024];
        bytes[..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(if big { b"BIG-IEEE" } else { b"LTL-IEEE" });
        put_i32(&mut bytes, 8, 2, big);
        put_i32(&mut bytes, 12, 6, big);
        put_i32(&mut bytes, 76, 3, big);
        put_i32(&mut bytes, 80, 3, big);
        put_i32(&mut bytes, 84, 517, big);
        put_f64(&mut bytes, 2 * 1024, 0.0, big);
        put_f64(&mut bytes, 2 * 1024 + 8, 0.0, big);
        put_f64(&mut bytes, 2 * 1024 + 16, 1.0, big);
        let descriptor = 2 * 1024 + 3 * 8;
        put_f64(&mut bytes, descriptor, start, big);
        put_f64(&mut bytes, descriptor + 8, end, big);
        for (slot, value) in [target, center, 1, 2, 513, 516].into_iter().enumerate() {
            put_i32(&mut bytes, descriptor + 16 + 4 * slot, value, big);
        }
        bytes
    }

    #[test]
    fn parse_rejects_implicit_target_duplicate_and_missing_files() {
        for args in [
            vec!["a.bsp"],
            vec!["--target", "bad", "a.bsp"],
            vec!["--target", "399"],
            vec!["--target"],
            vec!["--target", "399", "--target", "301", "a.bsp"],
            vec!["--target", "399", "--at-et", "0", "a.bsp"],
        ] {
            assert!(arguments(&args.into_iter().map(str::to_string).collect::<Vec<_>>()).is_err());
        }
        let supplied = vec![
            "--target".to_string(),
            "-399".to_string(),
            "--".to_string(),
            "-local.bsp".to_string(),
        ];
        assert_eq!(arguments(&supplied).unwrap(), (-399, vec!["-local.bsp"]));
    }

    #[test]
    fn next_binary64_neighbor_handles_zero_negative_and_finite_extremes() {
        assert_eq!(neighbor(0.0, true), Some(f64::from_bits(1)));
        assert_eq!(neighbor(-0.0, false), Some(-f64::from_bits(1)));
        assert_eq!(
            neighbor(8.0, true),
            Some(f64::from_bits(8.0_f64.to_bits() + 1))
        );
        assert_eq!(
            neighbor(-8.0, false),
            Some(f64::from_bits((-8.0_f64).to_bits() + 1))
        );
        assert_eq!(neighbor(f64::MAX, true), None);
        assert_eq!(neighbor(-f64::MAX, false), None);
    }

    #[test]
    fn emits_three_samples_per_boundary_with_correct_kernel_priority() {
        let mut kernels = SpkKernelSet::new();
        kernels
            .load(Cursor::new(synthetic_spk(false, 399, 0, 0.0, 10.0)))
            .unwrap();
        kernels
            .load(Cursor::new(synthetic_spk(true, 399, 301, 4.0, 8.0)))
            .unwrap();

        let mut output = Vec::new();
        assert_eq!(render(&mut output, &kernels, 399).unwrap(), 12);
        let text = String::from_utf8(output).unwrap();
        let rows: Vec<Vec<&str>> = text
            .lines()
            .skip(1)
            .map(|line| line.split(',').collect())
            .collect();
        assert_eq!(rows.len(), 12);
        assert!(rows.iter().all(|row| row.len() == 8));
        // First boundary: below zero has no coverage; at/after zero use older kernel.
        assert_eq!((rows[0][1], rows[0][3]), ("before", ""));
        assert_eq!((rows[1][1], rows[1][3]), ("at", "1"));
        // Second boundary 4: just before old, at and just after new.
        assert_eq!((rows[3][1], rows[3][3]), ("before", "1"));
        assert_eq!((rows[4][1], rows[4][3]), ("at", "2"));
        assert_eq!((rows[5][1], rows[5][4]), ("after", "301"));
        // Third boundary 8: new is still selected at its inclusive end.
        assert_eq!((rows[7][1], rows[7][3]), ("at", "2"));
        assert_eq!((rows[8][1], rows[8][3]), ("after", "1"));
        // Final endpoint 10 is covered, but the next representable ET is not.
        assert_eq!((rows[10][1], rows[10][3]), ("at", "1"));
        assert_eq!((rows[11][1], rows[11][3]), ("after", ""));
        assert!(rows[4][7] == "true");
    }

    #[test]
    fn empty_target_and_invalid_paths_fail_before_any_output() {
        let mut kernels = SpkKernelSet::new();
        kernels
            .load(Cursor::new(synthetic_spk(false, 399, 0, 0.0, 10.0)))
            .unwrap();
        let mut output = Vec::new();
        assert!(render(&mut output, &kernels, 301).is_err());
        assert!(output.is_empty());

        let mut bad_output = Vec::new();
        let bad_args = vec![
            "--target".to_string(),
            "399".to_string(),
            "no-such-spk-file.bsp".to_string(),
        ];
        assert!(run(&bad_args, &mut bad_output).is_err());
        assert!(bad_output.is_empty());
    }
}
