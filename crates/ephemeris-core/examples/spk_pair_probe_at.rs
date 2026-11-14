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

//! Offline geometric SPK pair diagnostics with explicit per-row failures.
//!
//! Modification (2026-11-14): read-only, user-supplied-kernel CSV adapter.
//! The Rust core still owns all SPK priority, interpolation and center chains.
//! This CLI is not an independent NAIF/JPL numerical accuracy check.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Seek, Write};

use virgostars_ephemeris_core::spk_direct_state::SpkStateError;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;
use virgostars_ephemeris_core::spk_pair_diagnostics::{
    probe_relative_states_at, SpkRelativeProbe,
};
use virgostars_ephemeris_core::spk_relative_state::SpkRelativeError;

const HEADER: &str = "et_tdb_seconds,request_index,target_id,center_id,status,common_ancestor_id,\
x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s,error_code,error_message";

fn usage() -> &'static str {
    "usage: cargo run --offline --locked --example spk_pair_probe_at -- \
     --et ET_TDB_SECONDS --pair TARGET:CENTER [--pair TARGET:CENTER ...] \
     <kernel1.bsp> [kernel2.bsp ...]\n\
     ET is finite TDB seconds past J2000. SPKs load in given priority order.\n\
     Individual pair errors remain in CSV; process exits 2 if any pair fails.\n\
     Use '--' before filenames starting with a dash."
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
        .ok_or_else(|| format!("invalid --pair {raw:?}; expected TARGET:CENTER"))?;
    let target = target
        .parse::<i32>()
        .map_err(|_| format!("invalid target NAIF i32 ID: {target:?}"))?;
    let center = center
        .parse::<i32>()
        .map_err(|_| format!("invalid center NAIF i32 ID: {center:?}"))?;
    if target == center {
        return Err("same-body pair cannot establish SPK data coverage".into());
    }
    Ok((target, center))
}

fn parse_arguments(values: &[String]) -> Result<Arguments<'_>, String> {
    let mut et = None;
    let mut pairs = Vec::new();
    let mut paths = Vec::new();
    let mut i = 0;
    while i < values.len() {
        match values[i].as_str() {
            "--et" => {
                let raw = values
                    .get(i + 1)
                    .ok_or("--et requires explicit TDB seconds past J2000")?;
                let value = raw.parse::<f64>().map_err(|_| "invalid --et TDB epoch")?;
                if !value.is_finite() {
                    return Err("--et must be finite TDB seconds past J2000".into());
                }
                if et.replace(value).is_some() {
                    return Err("duplicate --et option".into());
                }
                i += 2;
            }
            "--pair" => {
                let value = values.get(i + 1).ok_or("--pair requires TARGET:CENTER")?;
                pairs.push(parse_pair(value)?);
                i += 2;
            }
            "--" => {
                paths.extend(values[i + 1..].iter().map(String::as_str));
                break;
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unexpected option {flag}; {}", usage()));
            }
            path => {
                paths.push(path);
                i += 1;
            }
        }
    }
    let et = et.ok_or_else(|| format!("missing --et; {}", usage()))?;
    if pairs.is_empty() {
        return Err(format!("at least one --pair is required; {}", usage()));
    }
    if paths.is_empty() || paths.iter().any(|path| path.is_empty()) {
        return Err(format!("at least one local SPK file is required; {}", usage()));
    }
    Ok(Arguments { et, pairs, paths })
}

// Error codes are stable identifiers; messages are context for human review.
fn error_code(error: &SpkRelativeError) -> &'static str {
    match error {
        SpkRelativeError::InvalidEpoch => "invalid_epoch",
        SpkRelativeError::Cycle { .. } => "cycle",
        SpkRelativeError::Disconnected { .. } => "disconnected",
        SpkRelativeError::NumericalOverflow => "numerical_overflow",
        SpkRelativeError::Segment(reason) => match reason {
            SpkStateError::Io(_) => "segment_io",
            SpkStateError::InvalidEpoch => "segment_invalid_epoch",
            SpkStateError::MissingSegment { .. } => "segment_missing",
            SpkStateError::UnsupportedFrame(_) => "unsupported_frame",
            SpkStateError::UnsupportedType(_) => "unsupported_type",
            SpkStateError::InvalidSegment(_) => "invalid_segment",
            SpkStateError::Interpolation(_) => "interpolation",
        },
    }
}

fn quoted_csv(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

fn render<W: Write>(
    output: &mut W,
    et: f64,
    probes: &[SpkRelativeProbe],
) -> io::Result<(usize, usize)> {
    writeln!(output, "{HEADER}")?;
    let mut errors = 0;
    for probe in probes {
        write!(
            output,
            "{et:.17e},{},{},{},",
            probe.request_index, probe.target_id, probe.center_id
        )?;
        match &probe.result {
            Ok(state) => {
                write!(output, "ok,{},", state.common_ancestor_id)?;
                for (index, value) in state
                    .position_km
                    .iter()
                    .chain(state.velocity_km_per_second.iter())
                    .enumerate()
                {
                    if index != 0 {
                        write!(output, ",")?;
                    }
                    write!(output, "{value:.17e}")?;
                }
                writeln!(output, ",,")?;
            }
            Err(error) => {
                errors += 1;
                // Empty ancestor and six components, never fabricated zero.
                write!(output, "error")?;
                for _ in 0..7 {
                    write!(output, ",")?;
                }
                writeln!(output, ",{},{}", error_code(error), quoted_csv(&error.to_string()))?;
            }
        }
    }
    Ok((probes.len(), errors))
}

fn evaluate_and_render<R: Read + Seek, W: Write>(
    sources: impl IntoIterator<Item = R>,
    pairs: &[(i32, i32)],
    et: f64,
    output: &mut W,
) -> Result<(usize, usize), Box<dyn Error>> {
    if !et.is_finite() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "nonfinite TDB epoch").into());
    }
    if pairs.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no diagnostic pairs").into());
    }
    let mut kernels = SpkKernelSet::new();
    let mut loaded = 0usize;
    for source in sources {
        kernels.load(source)?;
        loaded += 1;
    }
    if loaded == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no SPK kernels").into());
    }
    // All kernels are validated and all row outcomes computed before stdout.
    // Per-pair failures become error rows, not errors from this function.
    let probes = probe_relative_states_at(&mut kernels, pairs, et)?;
    Ok(render(output, et, &probes)?)
}

fn run<W: Write>(values: &[String], output: &mut W) -> Result<(usize, usize), Box<dyn Error>> {
    let args = parse_arguments(values)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let files = args
        .paths
        .iter()
        .map(File::open)
        .collect::<io::Result<Vec<_>>>()?;
    evaluate_and_render(files, &args.pairs, args.et, output)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args, &mut io::stdout().lock()) {
        Ok((rows, failures)) => {
            eprintln!(
                "Geometric SPK J2000/ICRF relative diagnostics: {rows} pairs, \
                 {failures} failures (km, km/s); NOT independent NAIF/JPL validation."
            );
            if failures != 0 {
                std::process::exit(2);
            }
        }
        Err(error) => {
            eprintln!("SPK pair diagnostic preflight failed: {error}");
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

    fn i32_at(bytes: &mut [u8], offset: usize, value: i32, order: Order) {
        let raw = match order {
            Order::Little => value.to_le_bytes(),
            Order::Big => value.to_be_bytes(),
        };
        bytes[offset..offset + 4].copy_from_slice(&raw);
    }

    fn f64_at(bytes: &mut [u8], offset: usize, value: f64, order: Order) {
        let raw = match order {
            Order::Little => value.to_le_bytes(),
            Order::Big => value.to_be_bytes(),
        };
        bytes[offset..offset + 8].copy_from_slice(&raw);
    }

    fn kernel(order: Order, target: i32, center: i32, frame: i32, position: f64) -> Cursor<Vec<u8>> {
        // Synthetic one-record Type-2 Chebyshev SPK with constant XYZ.
        let mut bytes = vec![0u8; 5 * 1024];
        bytes[..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(match order {
            Order::Little => b"LTL-IEEE",
            Order::Big => b"BIG-IEEE",
        });
        for (offset, val) in [(8, 2), (12, 6), (76, 3), (80, 3), (84, 525)] {
            i32_at(&mut bytes, offset, val, order);
        }
        let summary = 2 * 1024;
        f64_at(&mut bytes, summary + 16, 1.0, order);
        f64_at(&mut bytes, summary + 24, 0.0, order);
        f64_at(&mut bytes, summary + 32, 10.0, order);
        for (index, val) in [target, center, frame, 2, 513, 524].into_iter().enumerate() {
            i32_at(&mut bytes, summary + 40 + 4 * index, val, order);
        }
        let record = 4 * 1024;
        f64_at(&mut bytes, record, 5.0, order);
        f64_at(&mut bytes, record + 8, 5.0, order);
        for (i, val) in [position, position * 2.0, -position].into_iter().enumerate() {
            f64_at(&mut bytes, record + 16 + i * 16, val, order);
            f64_at(&mut bytes, record + 24 + i * 16, 0.0, order);
        }
        for (i, val) in [0.0, 10.0, 8.0, 1.0].into_iter().enumerate() {
            f64_at(&mut bytes, record + 64 + i * 8, val, order);
        }
        Cursor::new(bytes)
    }

    fn evaluate(
        sources: Vec<Cursor<Vec<u8>>>,
        pairs: &[(i32, i32)],
        et: f64,
    ) -> Result<(String, (usize, usize)), Box<dyn Error>> {
        let mut out = Vec::new();
        let result = evaluate_and_render(sources, pairs, et, &mut out)?;
        Ok((String::from_utf8(out)?, result))
    }

    #[test]
    fn good_bad_good_rows_are_ordered_and_have_stable_codes() {
        let sources = vec![
            kernel(Order::Little, 399, 0, 1, 100.0),
            kernel(Order::Big, 301, 399, 1, 2.0),
        ];
        let pairs = [(301, 0), (999, 0), (399, 0), (301, 0)];
        let (csv, counts) = evaluate(sources, &pairs, 5.0).unwrap();
        assert_eq!(counts, (4, 1));
        let rows = csv.lines().collect::<Vec<_>>();
        assert_eq!(rows[0], HEADER);
        assert_eq!(rows.len(), 5);
        assert!(rows.iter().skip(1).all(|row| row.split(',').count() == 14));
        assert!(rows[1].starts_with("5.00000000000000000e0,0,301,0,ok,0,"));
        assert!(rows[1].contains(",1.02000000000000000e2,"));
        assert!(rows[1].ends_with(",,"));
        assert!(rows[2].starts_with("5.00000000000000000e0,1,999,0,error,"));
        assert!(rows[2].contains(",disconnected,\"SPK target 999"));
        assert_eq!(rows[2].split(',').nth(5), Some(""));
        assert!(rows[3].contains(",2,399,0,ok,0,"));
        assert_eq!(rows[1].split(',').skip(6).take(6).collect::<Vec<_>>(),
                   rows[4].split(',').skip(6).take(6).collect::<Vec<_>>());
    }

    #[test]
    fn unsupported_newer_overlay_only_fails_affected_pairs() {
        let sources = vec![
            kernel(Order::Little, 399, 0, 1, 100.0),
            kernel(Order::Big, 301, 399, 1, 2.0),
            kernel(Order::Little, 301, 399, 17, 99.0),
        ];
        let pairs = [(301, 0), (399, 0), (301, 399)];
        let (csv, counts) = evaluate(sources, &pairs, 5.0).unwrap();
        assert_eq!(counts, (3, 2));
        let rows = csv.lines().collect::<Vec<_>>();
        assert!(rows[1].contains(",unsupported_frame,"));
        assert!(rows[2].contains(",ok,"));
        assert!(rows[3].contains(",unsupported_frame,"));
        assert!(rows[1].split(',').skip(5).take(7).all(|field| field.is_empty()));
    }

    #[test]
    fn corrupt_higher_priority_segment_is_not_replaced() {
        let mut corrupted = kernel(Order::Big, 301, 399, 1, 99.0);
        // Footer record-size word is invalid but descriptor remains valid.
        f64_at(corrupted.get_mut(), 4 * 1024 + 80, 0.0, Order::Big);
        let sources = vec![
            kernel(Order::Little, 301, 399, 1, 2.0),
            corrupted,
        ];
        let (csv, counts) = evaluate(sources, &[(301, 399)], 5.0).unwrap();
        assert_eq!(counts, (1, 1));
        assert!(csv.contains(",invalid_segment,\""));
        assert!(!csv.contains(",ok,"));
    }

    #[test]
    fn strict_cli_arguments_do_not_infer_scientific_defaults() {
        for args in [
            vec!["--et", "0", "--pair", "301:399"],
            vec!["--pair", "301:399", "a.bsp"],
            vec!["--et", "0", "a.bsp"],
            vec!["--et", "0", "--pair", "301:301", "a.bsp"],
            vec!["--et", "NaN", "--pair", "301:399", "a.bsp"],
            vec!["--et", "inf", "--pair", "301:399", "a.bsp"],
            vec!["--et", "0", "--pair", "301:399", "--et", "1", "a.bsp"],
            vec!["--et", "0", "--pair", "301", "a.bsp"],
            vec!["--et", "0", "--pair", "2147483648:0", "a.bsp"],
            vec!["--et", "0", "--pair", "301:399", "--unknown", "a.bsp"],
        ] {
            let values = args.into_iter().map(str::to_owned).collect::<Vec<_>>();
            assert!(parse_arguments(&values).is_err(), "accepted {values:?}");
        }
        let values = ["--et", "-1.25", "--pair", "-10:399", "--", "-local.bsp"]
            .map(str::to_owned);
        assert_eq!(
            parse_arguments(&values).unwrap(),
            Arguments {
                et: -1.25,
                pairs: vec![(-10, 399)],
                paths: vec!["-local.bsp"]
            }
        );
    }

    #[test]
    fn preflight_failures_emit_no_csv_and_quote_errors_safely() {
        let mut output = Vec::new();
        assert!(evaluate_and_render(
            [kernel(Order::Little, 301, 399, 1, 2.0)],
            &[(301, 399)],
            f64::INFINITY,
            &mut output
        ).is_err());
        assert!(output.is_empty());
        assert!(evaluate_and_render(
            [Cursor::new(vec![0u8; 1024])],
            &[(301, 399)],
            5.0,
            &mut output
        ).is_err());
        assert!(output.is_empty());
        assert!(evaluate_and_render(
            Vec::<Cursor<Vec<u8>>>::new(),
            &[(301, 399)],
            5.0,
            &mut output
        ).is_err());
        assert!(output.is_empty());
        assert_eq!(quoted_csv("one,\"two\""), "\"one,\"\"two\"\"\"");
        assert_eq!(error_code(&SpkRelativeError::NumericalOverflow), "numerical_overflow");
    }
}
