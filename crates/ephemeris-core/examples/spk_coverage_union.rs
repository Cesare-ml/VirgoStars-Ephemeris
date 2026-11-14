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

//! Offline declared coverage union across explicitly loaded local NAIF SPKs.
//!
//! Modification (2026-11-14): read-only target/coverage inventory CLI. It
//! delegates all SPK decoding and union math to the canonical Rust kernel set.
//! A declared span is NOT proof that the coefficients, frames or center chains
//! can produce a geometric state; no NASA/JPL accuracy claim is made.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Seek, Write};

use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;

const HEADER: &str =
    "target_id,coverage_start_et_tdb_seconds,coverage_end_et_tdb_seconds";

fn usage() -> &'static str {
    "usage: cargo run --offline --locked --example spk_coverage_union -- \
     [--target NAIF_ID] <kernel1.bsp> [kernel2.bsp ...]\n\
     Kernels are in load order; metadata is declared, not validated geometry.\n\
     Epochs are TDB seconds past J2000; use '--' before dash-prefixed paths."
}

#[derive(Debug, PartialEq)]
struct Arguments<'a> {
    target: Option<i32>,
    paths: Vec<&'a str>,
}

fn parse_arguments(values: &[String]) -> Result<Arguments<'_>, String> {
    let mut target = None;
    let mut paths = Vec::new();
    let mut index = 0;
    while index < values.len() {
        match values[index].as_str() {
            "--target" => {
                if target.is_some() {
                    return Err("duplicate --target option".into());
                }
                let value = values
                    .get(index + 1)
                    .ok_or("--target requires a signed NAIF i32 ID")?;
                target = Some(
                    value
                        .parse::<i32>()
                        .map_err(|_| "invalid --target NAIF i32 ID")?,
                );
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
    if paths.is_empty() || paths.iter().any(|path| path.is_empty()) {
        return Err(format!("at least one local SPK path required; {}", usage()));
    }
    Ok(Arguments { target, paths })
}

fn render<W: Write, R: Read + Seek>(
    output: &mut W,
    kernels: &SpkKernelSet<R>,
    target: Option<i32>,
) -> io::Result<usize> {
    let ids = kernels.declared_target_ids();
    if let Some(selected) = target {
        if ids.binary_search(&selected).is_err() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("NAIF target {selected} has no declared SPK segments"),
            ));
        }
    }
    writeln!(output, "{HEADER}")?;
    let mut rows = 0;
    for id in ids {
        if target.is_some_and(|selected| selected != id) {
            continue;
        }
        for (start, end) in kernels.declared_coverage_intervals_for_target(id) {
            writeln!(output, "{id},{start:.17e},{end:.17e}")?;
            rows += 1;
        }
    }
    Ok(rows)
}

fn run<W: Write>(values: &[String], output: &mut W) -> Result<usize, Box<dyn Error>> {
    let args = parse_arguments(values).map_err(|error| {
        io::Error::new(io::ErrorKind::InvalidInput, error)
    })?;
    let mut kernels = SpkKernelSet::new();
    for path in args.paths {
        kernels.load(File::open(path)?)?;
    }
    Ok(render(output, &kernels, args.target)?)
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let mut output = io::stdout().lock();
    let rows = run(&arguments, &mut output)?;
    eprintln!(
        "SPK declared-only coverage: {rows} rows. Not proof of supported \
         Type-2/3/J2000 states, complete center chains, NAIF provenance, \
         or JPL/Horizons numerical accuracy."
    );
    Ok(())
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

    fn i32_word(file: &mut [u8], offset: usize, value: i32, order: Order) {
        let bytes = match order {
            Order::Little => value.to_le_bytes(),
            Order::Big => value.to_be_bytes(),
        };
        file[offset..offset + 4].copy_from_slice(&bytes);
    }

    fn f64_word(file: &mut [u8], offset: usize, value: f64, order: Order) {
        let bytes = match order {
            Order::Little => value.to_le_bytes(),
            Order::Big => value.to_be_bytes(),
        };
        file[offset..offset + 8].copy_from_slice(&bytes);
    }

    // A synthetic five-record DAF header/index only. No JPL/NAIF coefficients.
    fn kernel(order: Order, id: i32, start: f64, end: f64) -> Cursor<Vec<u8>> {
        let mut bytes = vec![0u8; 5 * 1024];
        bytes[..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(match order {
            Order::Little => b"LTL-IEEE",
            Order::Big => b"BIG-IEEE",
        });
        for (offset, value) in [(8, 2), (12, 6), (76, 3), (80, 3), (84, 517)] {
            i32_word(&mut bytes, offset, value, order);
        }
        let summary = 2 * 1024;
        f64_word(&mut bytes, summary + 16, 1.0, order);
        f64_word(&mut bytes, summary + 24, start, order);
        f64_word(&mut bytes, summary + 32, end, order);
        for (i, value) in [id, 0, 1, 2, 513, 516].into_iter().enumerate() {
            i32_word(&mut bytes, summary + 40 + 4 * i, value, order);
        }
        Cursor::new(bytes)
    }

    #[test]
    fn mixed_endian_overlaps_coalesce_but_disjoint_windows_remain_separate() {
        let mut set = SpkKernelSet::new();
        set.load(kernel(Order::Little, 301, 0.0, 10.0)).unwrap();
        set.load(kernel(Order::Big, 399, -8.0, 2.0)).unwrap();
        set.load(kernel(Order::Big, 301, 5.0, 20.0)).unwrap();
        set.load(kernel(Order::Little, 301, 30.0, 40.0)).unwrap();

        let mut bytes = Vec::new();
        assert_eq!(render(&mut bytes, &set, None).unwrap(), 3);
        let text = String::from_utf8(bytes).unwrap();
        assert_eq!(
            text,
            format!(
                "{HEADER}\n\
                 301,{:.17e},{:.17e}\n\
                 301,{:.17e},{:.17e}\n\
                 399,{:.17e},{:.17e}\n",
                0.0, 20.0, 30.0, 40.0, -8.0, 2.0
            )
        );

        let mut filtered = Vec::new();
        assert_eq!(render(&mut filtered, &set, Some(301)).unwrap(), 2);
        let csv = String::from_utf8(filtered).unwrap();
        assert!(csv.starts_with(HEADER));
        assert!(!csv.contains("\n399,"));
        assert_eq!(csv.lines().count(), 3);
    }

    #[test]
    fn one_ulp_gap_is_not_closed_and_signed_zero_is_normalized() {
        let next = f64::from_bits(10.0_f64.to_bits() + 1);
        let mut set = SpkKernelSet::new();
        set.load(kernel(Order::Big, -10, -0.0, 10.0)).unwrap();
        set.load(kernel(Order::Little, -10, next, 20.0)).unwrap();
        let mut bytes = Vec::new();
        assert_eq!(render(&mut bytes, &set, Some(-10)).unwrap(), 2);
        let content = String::from_utf8(bytes).unwrap();
        assert_eq!(
            content,
            format!(
                "{HEADER}\n-10,{:.17e},{:.17e}\n-10,{:.17e},{:.17e}\n",
                0.0, 10.0, next, 20.0
            )
        );
        assert!(!content.contains("-0.00000000000000000e0"));
    }

    #[test]
    fn refuses_missing_target_without_printing_misleading_csv() {
        let mut set = SpkKernelSet::new();
        set.load(kernel(Order::Little, 301, 0.0, 10.0)).unwrap();
        let mut bytes = Vec::new();
        let error = render(&mut bytes, &set, Some(399)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("399"));
        assert!(bytes.is_empty());
    }

    #[test]
    fn rejects_implicit_kernels_duplicate_filters_and_non_i32_ids() {
        let case = |args: &[&str]| {
            let args: Vec<_> = args.iter().map(|s| s.to_string()).collect();
            parse_arguments(&args).unwrap_err()
        };
        assert!(case(&[]).contains("at least one"));
        assert!(case(&["--target", "301"]).contains("at least one"));
        assert!(case(&["--target", "301", "--target", "302", "a.bsp"])
            .contains("duplicate"));
        assert!(case(&["--target", "999999999999999", "a.bsp"]).contains("i32"));
        assert!(case(&["--unknown", "a.bsp"]).contains("unknown"));
        let args = ["--target", "-10", "--", "-kernel.bsp"];
        let values: Vec<_> = args.iter().map(|s| s.to_string()).collect();
        let parsed = parse_arguments(&values).unwrap();
        assert_eq!(parsed.target, Some(-10));
        assert_eq!(parsed.paths, vec!["-kernel.bsp"]);
    }

    #[test]
    fn failed_later_source_load_keeps_stdout_empty() {
        let missing = vec!["this-nonexistent-kernel-file.bsp".to_owned()];
        let mut buffer = Vec::new();
        assert!(run(&missing, &mut buffer).is_err());
        assert!(buffer.is_empty());
    }
}
