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

//! Offline declared-target SPK coverage and effective source at one ET.
//!
//! Modification (2026-11-14): single-epoch CSV inventory, including missing
//! declared targets and unsupported winning overlays, with no coefficient
//! reads, independent numerical validation, or automatic kernel downloads.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Seek, Write};

use virgostars_ephemeris_core::spk_effective_index::effective_segments_at;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;

const HEADER: &str = "et_tdb_seconds,target_id,status,kernel_load_index,center_id,frame_id,\
spk_type,candidate_j2000_type23,start_et_tdb_seconds,end_et_tdb_seconds,initial_word,final_word";

fn usage() -> &'static str {
    "usage: cargo run --offline --locked --example spk_effective_at -- \
     --et ET_TDB_SECONDS <kernel1.bsp> [kernel2.bsp ...]\n\
     Every declared NAIF target is included, including those with no coverage.\n\
     ET is TDB seconds since J2000, NOT UTC or Julian date.\n\
     Kernels are in declared priority order, last loaded wins.\n\
     Use '--' before filenames starting with a dash."
}

#[derive(Debug, PartialEq)]
struct Arguments<'a> {
    et: f64,
    paths: Vec<&'a str>,
}

fn parse_arguments(values: &[String]) -> Result<Arguments<'_>, String> {
    let mut et = None;
    let mut paths = Vec::new();
    let mut index = 0;
    while index < values.len() {
        match values[index].as_str() {
            "--et" => {
                if et.is_some() {
                    return Err("duplicate --et option".into());
                }
                let raw = values.get(index + 1).ok_or("--et requires TDB seconds past J2000")?;
                let value = raw.parse::<f64>().map_err(|_| "invalid --et TDB epoch")?;
                if !value.is_finite() {
                    return Err("--et must be finite TDB seconds past J2000".into());
                }
                et = Some(value);
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
    if paths.is_empty() || paths.iter().any(|path| path.is_empty()) {
        return Err(format!("at least one explicit local SPK file required; {}", usage()));
    }
    Ok(Arguments { et, paths })
}

/// Construct all metadata results before printing any potentially misleading
/// CSV, preserving canonical priority and exact source load indices.
fn render<W: Write, R: Read + Seek>(
    output: &mut W,
    kernels: &SpkKernelSet<R>,
    et: f64,
) -> io::Result<usize> {
    let winners = effective_segments_at(kernels, et).map_err(io::Error::other)?;
    let mut next_winner = winners.iter().peekable();
    let targets = kernels.declared_target_ids();
    writeln!(output, "{HEADER}")?;

    for target in &targets {
        if next_winner.peek().is_some_and(|row| row.target_id == *target) {
            let row = next_winner.next().expect("selected winner");
            let segment = row.descriptor;
            let candidate = segment.frame_id == 1 && matches!(segment.spk_type, 2 | 3);
            writeln!(
                output,
                "{et:.17e},{target},covered,{},{},{},{},{},{:.17e},{:.17e},{},{}",
                row.kernel_load_index,
                segment.center_id,
                segment.frame_id,
                segment.spk_type,
                candidate,
                segment.start_tdb_seconds,
                segment.end_tdb_seconds,
                segment.initial_address,
                segment.final_address,
            )?;
        } else {
            // Nine empty *metadata* columns: no coverage is never represented
            // with a false zero source, body, frame, type or DAF address.
            writeln!(output, "{et:.17e},{target},missing,,,,,,,,,")?;
        }
    }
    debug_assert!(next_winner.next().is_none());
    Ok(targets.len())
}

fn run<W: Write>(values: &[String], output: &mut W) -> Result<usize, Box<dyn Error>> {
    let args = parse_arguments(values)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let mut kernels = SpkKernelSet::new();
    for path in args.paths {
        kernels.load(File::open(path)?)?;
    }
    Ok(render(output, &kernels, args.et)?)
}

fn main() {
    let values: Vec<String> = env::args().skip(1).collect();
    match run(&values, &mut io::stdout().lock()) {
        Ok(rows) => eprintln!(
            "Emitted {rows} declared-target SPK metadata rows. \
             A covered descriptor is NOT proof of an evaluable geometric state, \
             complete center chain, authentic NASA/JPL kernel, or numerical accuracy."
        ),
        Err(error) => {
            eprintln!("SPK effective segment inventory failed: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[derive(Clone, Copy)]
    enum Order {
        Little,
        Big,
    }

    fn put_i32(bytes: &mut [u8], offset: usize, value: i32, order: Order) {
        let encoded = match order {
            Order::Little => value.to_le_bytes(),
            Order::Big => value.to_be_bytes(),
        };
        bytes[offset..offset + 4].copy_from_slice(&encoded);
    }

    fn put_f64(bytes: &mut [u8], offset: usize, value: f64, order: Order) {
        let encoded = match order {
            Order::Little => value.to_le_bytes(),
            Order::Big => value.to_be_bytes(),
        };
        bytes[offset..offset + 8].copy_from_slice(&encoded);
    }

    fn kernel(
        order: Order,
        target: i32,
        center: i32,
        frame: i32,
        kind: i32,
        start: f64,
        end: f64,
    ) -> Cursor<Vec<u8>> {
        // Minimal, invented DAF summary: no actual JPL coefficients are read.
        let mut bytes = vec![0u8; 5 * 1024];
        bytes[..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(match order {
            Order::Little => b"LTL-IEEE",
            Order::Big => b"BIG-IEEE",
        });
        for (offset, value) in [(8, 2), (12, 6), (76, 3), (80, 3), (84, 517)] {
            put_i32(&mut bytes, offset, value, order);
        }
        let summary = 2 * 1024;
        put_f64(&mut bytes, summary + 16, 1.0, order);
        put_f64(&mut bytes, summary + 24, start, order);
        put_f64(&mut bytes, summary + 32, end, order);
        for (slot, value) in [target, center, frame, kind, 513, 516].into_iter().enumerate() {
            put_i32(&mut bytes, summary + 40 + 4 * slot, value, order);
        }
        Cursor::new(bytes)
    }

    fn fixtures() -> SpkKernelSet<Cursor<Vec<u8>>> {
        let mut set = SpkKernelSet::new();
        set.load(kernel(Order::Little, 399, 0, 1, 2, -10.0, -2.0)).unwrap();
        set.load(kernel(Order::Big, 301, 399, 1, 2, 0.0, 10.0)).unwrap();
        // Newer unsupported frame and SPK type must be kept, not hidden.
        set.load(kernel(Order::Little, 301, 4, 17, 9, 4.0, 8.0)).unwrap();
        set
    }

    fn rows(output: &[u8]) -> Vec<Vec<&str>> {
        std::str::from_utf8(output).unwrap().lines()
            .skip(1).map(|line| line.split(',').collect()).collect()
    }

    #[test]
    fn every_declared_target_is_listed_in_sorted_order_even_when_uncovered() {
        let set = fixtures();
        let mut out = Vec::new();
        assert_eq!(render(&mut out, &set, 5.0).unwrap(), 2);
        let csv = std::str::from_utf8(&out).unwrap();
        assert!(csv.starts_with(HEADER));
        let rows = rows(&out);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.len() == 12));
        assert_eq!((rows[0][1], rows[0][2]), ("301", "covered"));
        assert_eq!((rows[0][3], rows[0][4], rows[0][5], rows[0][6], rows[0][7]), ("2", "4", "17", "9", "false"));
        assert_eq!((rows[0][10], rows[0][11]), ("513", "516"));
        assert_eq!((rows[1][1], rows[1][2]), ("399", "missing"));
        assert!(rows[1][3..].iter().all(|field| field.is_empty()));
        assert_eq!(rows[0][0], format!("{:.17e}", 5.0));
        assert_eq!(set.kernel_count(), 3);
    }

    #[test]
    fn boundary_selection_and_unload_restore_oldest_winners() {
        let mut set = fixtures();
        let mut out = Vec::new();
        render(&mut out, &set, 8.0).unwrap();
        assert_eq!(rows(&out)[0][3], "2");
        out.clear();
        render(&mut out, &set, f64::from_bits(8.0_f64.to_bits() + 1)).unwrap();
        assert_eq!((rows(&out)[0][2], rows(&out)[0][3], rows(&out)[0][7]), ("covered", "1", "true"));
        out.clear();
        render(&mut out, &set, -10.0).unwrap();
        assert_eq!((rows(&out)[0][2], rows(&out)[1][2]), ("missing", "covered"));
        assert_eq!(rows(&out)[1][3], "0");
        out.clear();
        set.unload_last().unwrap();
        render(&mut out, &set, 5.0).unwrap();
        assert_eq!((rows(&out)[0][3], rows(&out)[0][7]), ("1", "true"));
    }

    #[test]
    fn no_coverage_has_explicit_missing_rows_not_empty_or_zero_sources() {
        let set = fixtures();
        let mut out = Vec::new();
        assert_eq!(render(&mut out, &set, -1.0).unwrap(), 2);
        let rows = rows(&out);
        assert!(rows.iter().all(|row| row[2] == "missing" && row[3..].iter().all(|v| v.is_empty())));
        assert!(rows.iter().all(|row| row.len() == 12));
    }

    #[test]
    fn parse_requires_explicit_et_files_and_finite_numbers() {
        for invalid in [
            vec!["kernel.bsp"],
            vec!["--et", "0"],
            vec!["--et", "nan", "a.bsp"],
            vec!["--et", "inf", "a.bsp"],
            vec!["--et", "0", "--et", "1", "a.bsp"],
            vec!["--et", "0", "--unknown", "a.bsp"],
            vec!["--et", "bad", "a.bsp"],
            vec!["--et", "0", "--"],
        ] {
            let args: Vec<String> = invalid.iter().map(|value| value.to_string()).collect();
            assert!(parse_arguments(&args).is_err(), "accepted {args:?}");
        }
        let values = vec![
            "--et".to_string(),
            "-0.25".to_string(),
            "--".to_string(),
            "-kernel.bsp".to_string(),
        ];
        assert_eq!(
            parse_arguments(&values).unwrap(),
            Arguments { et: -0.25, paths: vec!["-kernel.bsp"] }
        );
    }

    #[test]
    fn validation_failure_precedes_csv_header_and_kernel_load_errors() {
        let set = fixtures();
        let mut out = Vec::new();
        assert!(render(&mut out, &set, f64::NAN).is_err());
        assert!(out.is_empty());
        let invalid = vec!["--et".to_string(), "5".to_string(), "no-such-spk-file.bsp".to_string()];
        assert!(run(&invalid, &mut out).is_err());
        assert!(out.is_empty());
        let empty: SpkKernelSet<Cursor<Vec<u8>>> = SpkKernelSet::new();
        assert_eq!(render(&mut out, &empty, 0.0).unwrap(), 0);
        assert_eq!(std::str::from_utf8(&out).unwrap(), format!("{HEADER}\n"));
    }
}
