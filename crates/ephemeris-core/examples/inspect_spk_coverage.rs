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

//! List actual segment descriptors from local NAIF DAF/SPK files.
//!
//! Reuses the canonical Rust SPK index. Metadata is not a guarantee of
//! complete target/observer chains, interpolation support or JPL accuracy.
//! No kernel downloads, SPICE dependency, state calculations or default ETs.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Write};

use virgostars_ephemeris_core::spk_index::{inspect_spk, SpkIndex, SpkSegmentDescriptor};

const HEADER: &str = "load_index,segment_index,target_id,center_id,frame_id,spk_type,\
coverage_start_et_tdb_seconds,coverage_end_et_tdb_seconds,candidate_j2000_type2_or3";

fn usage() -> &'static str {
    "usage: cargo run --offline --locked --example inspect_spk_coverage -- \
     [--target NAIF_ID] [--at-et ET_TDB_SECONDS] <kernel1.bsp> [kernel2.bsp ...]\n\
     ET is TDB seconds past J2000; kernel paths are in NAIF load order"
}

fn arguments(values: &[String]) -> Result<(Option<i32>, Option<f64>, Vec<&str>), String> {
    let mut target = None;
    let mut selected_et = None;
    let mut files = Vec::new();
    let mut i = 0;
    while i < values.len() {
        match values[i].as_str() {
            "--target" => {
                if target.is_some() {
                    return Err("duplicate --target filter".into());
                }
                let raw = values
                    .get(i + 1)
                    .ok_or_else(|| "--target requires a signed NAIF integer ID".to_string())?;
                target = Some(
                    raw.parse::<i32>()
                        .map_err(|_| "invalid --target NAIF i32 ID".to_string())?,
                );
                i += 2;
            }
            "--at-et" => {
                if selected_et.is_some() {
                    return Err("duplicate --at-et epoch".into());
                }
                let raw = values
                    .get(i + 1)
                    .ok_or_else(|| "--at-et requires TDB seconds past J2000".to_string())?;
                let value = raw
                    .parse::<f64>()
                    .map_err(|_| "invalid --at-et epoch".to_string())?;
                if !value.is_finite() {
                    return Err("--at-et must be finite TDB seconds past J2000".into());
                }
                selected_et = Some(value);
                i += 2;
            }
            "--" => {
                files.extend(values[i + 1..].iter().map(String::as_str));
                break;
            }
            option if option.starts_with('-') => {
                return Err(format!(
                    "unexpected argument {option}; {usage}",
                    usage = usage()
                ));
            }
            _ => {
                files.push(values[i].as_str());
                i += 1;
            }
        }
    }
    if files.is_empty() || files.iter().any(|path| path.is_empty()) {
        return Err(format!(
            "at least one explicit local SPK file required; {}",
            usage()
        ));
    }
    if selected_et.is_some() && target.is_none() {
        return Err("--at-et requires an explicit --target NAIF ID".into());
    }
    Ok((target, selected_et, files))
}

fn is_candidate(segment: &SpkSegmentDescriptor) -> bool {
    segment.frame_id == 1 && matches!(segment.spk_type, 2 | 3)
}

/// Report the final covering descriptor for one target at an explicit ET.
////
//// This is *descriptor precedence*, not evidence that a complete relative
//// path or the selected segment's coefficient records are usable.
fn selected_descriptor(
    indexes: &[SpkIndex],
    target: i32,
    et: f64,
) -> Option<(usize, usize, SpkSegmentDescriptor)> {
    indexes
        .iter()
        .enumerate()
        .rev()
        .find_map(|(file_index, index)| {
            index
                .segments
                .iter()
                .enumerate()
                .rev()
                .find_map(|(segment_index, segment)| {
                    (segment.target_id == target
                        && segment.start_tdb_seconds <= et
                        && et <= segment.end_tdb_seconds)
                        .then_some((file_index, segment_index, *segment))
                })
        })
}

fn render<W: Write>(
    output: &mut W,
    indexes: &[SpkIndex],
    target: Option<i32>,
    selected: Option<(usize, usize)>,
) -> io::Result<usize> {
    writeln!(output, "{HEADER}")?;
    let mut count = 0;
    for (file_index, index) in indexes.iter().enumerate() {
        for (segment_index, segment) in index.segments.iter().enumerate() {
            if target.is_some_and(|id| segment.target_id != id) {
                continue;
            }
            if selected.is_some_and(|location| location != (file_index, segment_index)) {
                continue;
            }
            writeln!(
                output,
                "{},{},{},{},{},{},{:.17e},{:.17e},{}",
                file_index + 1,
                segment_index + 1,
                segment.target_id,
                segment.center_id,
                segment.frame_id,
                segment.spk_type,
                segment.start_tdb_seconds,
                segment.end_tdb_seconds,
                is_candidate(segment)
            )?;
            count += 1;
        }
    }
    Ok(count)
}

fn run<W: Write>(values: &[String], output: &mut W) -> Result<usize, Box<dyn Error>> {
    let (target, selected_et, files) = arguments(values)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;

    // Validate every supplied file before writing any CSV. A corrupt later
    // kernel must not produce an apparently complete partial listing.
    let mut indexes = Vec::with_capacity(files.len());
    for path in &files {
        let mut source = File::open(path)?;
        indexes.push(inspect_spk(&mut source)?);
    }
    let selected = if let Some(et) = selected_et {
        let target_id = target.expect("validated by arguments()");
        let (file_index, segment_index, _) = selected_descriptor(&indexes, target_id, et)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "no SPK segment covers the requested target at this ET",
                )
            })?;
        Some((file_index, segment_index))
    } else {
        None
    };
    let matching = if selected.is_some() {
        1
    } else {
        indexes
            .iter()
            .flat_map(|index| &index.segments)
            .filter(|segment| target.is_none_or(|id| segment.target_id == id))
            .count()
    };
    if matching == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no SPK descriptors match the supplied target filter",
        )
        .into());
    }
    let count = render(output, &indexes, target, selected)?;
    debug_assert_eq!(count, matching);
    Ok(count)
}

fn main() {
    let values: Vec<String> = env::args().skip(1).collect();
    match run(&values, &mut io::stdout().lock()) {
        Ok(count) => {
            eprintln!(
                "Listed {count} declared SPK segment descriptors in input load order. \
                 Metadata is NOT scientific verification or proof of path coverage."
            );
        }
        Err(error) => {
            eprintln!("SPK metadata inspection failed: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn descriptor(target: i32, frame: i32, kind: i32) -> SpkSegmentDescriptor {
        SpkSegmentDescriptor {
            start_tdb_seconds: -86_400.0,
            end_tdb_seconds: 86_400.0,
            target_id: target,
            center_id: 0,
            frame_id: frame,
            spk_type: kind,
            initial_address: 513,
            final_address: 516,
        }
    }

    fn index(segments: Vec<SpkSegmentDescriptor>) -> SpkIndex {
        SpkIndex {
            byte_order: virgostars_ephemeris_core::spk_index::SpkByteOrder::LittleEndian,
            segments,
        }
    }

    fn output_text(indexes: &[SpkIndex], target: Option<i32>) -> (String, usize) {
        let mut out = Vec::new();
        let count = render(&mut out, indexes, target, None).unwrap();
        (String::from_utf8(out).unwrap(), count)
    }

    #[test]
    fn preserves_load_and_segment_order_in_csv() {
        let indexes = vec![
            index(vec![descriptor(399, 1, 2), descriptor(301, 1, 3)]),
            index(vec![descriptor(399, 1, 2)]),
        ];
        let (csv, count) = output_text(&indexes, None);
        let rows: Vec<_> = csv.lines().collect();
        assert_eq!(count, 3);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0], HEADER);
        assert!(rows[1].starts_with("1,1,399,0,1,2,"));
        assert!(rows[2].starts_with("1,2,301,0,1,3,"));
        assert!(rows[3].starts_with("2,1,399,0,1,2,"));
        assert!(rows[1].ends_with(",true"));
    }

    #[test]
    fn filtering_retains_original_segment_ordinals() {
        let indexes = vec![index(vec![descriptor(301, 1, 2), descriptor(399, 1, 2)])];
        let (csv, count) = output_text(&indexes, Some(399));
        assert_eq!(count, 1);
        assert_eq!(csv.lines().count(), 2);
        assert!(csv.lines().nth(1).unwrap().starts_with("1,2,399,"));
    }

    #[test]
    fn selects_last_loaded_file_even_when_its_frame_is_unsupported() {
        let indexes = vec![
            index(vec![descriptor(399, 1, 2)]),
            index(vec![descriptor(399, 17, 9)]),
        ];
        let (file, segment, chosen) = selected_descriptor(&indexes, 399, 0.0).unwrap();
        assert_eq!((file, segment), (1, 0));
        assert_eq!(chosen.frame_id, 17);
        assert_eq!(chosen.spk_type, 9);
        assert!(!is_candidate(&chosen));
        let mut csv = Vec::new();
        assert_eq!(
            render(&mut csv, &indexes, Some(399), Some((file, segment))).unwrap(),
            1
        );
        let rendered = String::from_utf8(csv).unwrap();
        assert!(rendered
            .lines()
            .nth(1)
            .unwrap()
            .starts_with("2,1,399,0,17,9,"));
        assert!(rendered.ends_with(",false\n"));
    }

    #[test]
    fn selects_last_overlapping_segment_inside_one_file() {
        let earlier = descriptor(399, 1, 2);
        let mut later = descriptor(399, 1, 3);
        later.center_id = 301;
        let indexes = vec![index(vec![earlier, later])];
        let (file, segment, selected) = selected_descriptor(&indexes, 399, 12.5).unwrap();
        assert_eq!((file, segment), (0, 1));
        assert_eq!(selected.center_id, 301);
        assert_eq!(selected.spk_type, 3);
    }

    #[test]
    fn coverage_endpoints_are_inclusive_but_outside_and_gaps_fail() {
        let indexes = vec![index(vec![descriptor(399, 1, 2)])];
        for et in [-86_400.0, 0.0, 86_400.0] {
            assert!(selected_descriptor(&indexes, 399, et).is_some());
        }
        for et in [-86_400.5, 86_400.5, f64::NAN] {
            assert!(selected_descriptor(&indexes, 399, et).is_none());
        }
        assert!(selected_descriptor(&indexes, 301, 0.0).is_none());
        let mut split = descriptor(399, 1, 2);
        split.start_tdb_seconds = 100_000.0;
        split.end_tdb_seconds = 200_000.0;
        let indexes = vec![index(vec![descriptor(399, 1, 2), split])];
        assert!(selected_descriptor(&indexes, 399, 90_000.0).is_none());
        assert_eq!(selected_descriptor(&indexes, 399, 150_000.0).unwrap().1, 1);
    }

    #[test]
    fn selected_epoch_requires_one_target_and_finite_nonduplicated_time() {
        let ok = arguments(&[
            "--target".into(),
            "399".into(),
            "--at-et".into(),
            "-86400".into(),
            "first.bsp".into(),
            "second.bsp".into(),
        ])
        .unwrap();
        assert_eq!(
            ok,
            (Some(399), Some(-86_400.0), vec!["first.bsp", "second.bsp"])
        );
        for args in [
            vec!["--at-et", "0", "first.bsp"],
            vec!["--target", "399", "--at-et", "nan", "first.bsp"],
            vec!["--target", "399", "--at-et", "inf", "first.bsp"],
            vec!["--target", "399", "--at-et", "abc", "first.bsp"],
            vec!["--target", "399", "--at-et", "first.bsp"],
            vec![
                "--target",
                "399",
                "--at-et",
                "1",
                "--at-et",
                "2",
                "first.bsp",
            ],
            vec!["--target", "399", "--at-et", "0"],
        ] {
            let args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
            assert!(arguments(&args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn filtered_selection_does_not_reorder_descriptor_ordinals() {
        let indexes = vec![
            index(vec![descriptor(301, 1, 3), descriptor(399, 1, 2)]),
            index(vec![descriptor(301, 1, 2)]),
        ];
        let (file, segment, _) = selected_descriptor(&indexes, 399, 0.0).unwrap();
        let mut output = Vec::new();
        let count = render(&mut output, &indexes, Some(399), Some((file, segment))).unwrap();
        assert_eq!(count, 1);
        let text = String::from_utf8(output).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text.lines().nth(1).unwrap().starts_with("1,2,399,"));
    }

    #[test]
    fn does_not_mislabel_unknown_frames_or_types_as_supported() {
        for segment in [
            descriptor(399, 17, 2),
            descriptor(399, 1, 9),
            descriptor(399, 17, 9),
        ] {
            assert!(!is_candidate(&segment));
        }
        assert!(is_candidate(&descriptor(399, 1, 2)));
        assert!(is_candidate(&descriptor(399, 1, 3)));
        let (csv, _) = output_text(&[index(vec![descriptor(399, 1, 9)])], None);
        assert!(csv.ends_with(",false\n"));
    }

    #[test]
    fn refuses_missing_and_malformed_cli_input() {
        for arguments in [
            vec![],
            vec!["--target".to_string()],
            vec![
                "--target".to_string(),
                "nan".to_string(),
                "x.bsp".to_string(),
            ],
            vec![
                "--target".to_string(),
                "1".to_string(),
                "--target".to_string(),
            ],
            vec!["--unknown".to_string(), "x.bsp".to_string()],
        ] {
            assert!(super::arguments(&arguments).is_err());
        }
        assert_eq!(
            super::arguments(&[
                "--target".to_string(),
                "399".to_string(),
                "x.bsp".to_string()
            ])
            .unwrap(),
            (Some(399), None, vec!["x.bsp"])
        );
    }

    #[test]
    fn validates_every_file_before_any_csv_output() {
        let mut output = Vec::new();
        assert!(run(&["missing-local-spk.bsp".to_string()], &mut output).is_err());
        assert!(output.is_empty());
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

    fn minimal_daf_spk(big: bool) -> Vec<u8> {
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
        let summary = 2 * 1024 + 3 * 8;
        put_f64(&mut bytes, summary, -86_400.0, big);
        put_f64(&mut bytes, summary + 8, 86_400.0, big);
        for (index, value) in [399, 0, 1, 2, 513, 516].into_iter().enumerate() {
            put_i32(&mut bytes, summary + 16 + 4 * index, value, big);
        }
        bytes
    }

    #[test]
    fn consumes_actual_daf_parser_metadata_for_both_endiannesses() {
        for big in [false, true] {
            let parsed = inspect_spk(&mut Cursor::new(minimal_daf_spk(big))).unwrap();
            let (csv, count) = output_text(&[parsed], Some(399));
            assert_eq!(count, 1);
            assert!(csv.contains("1,1,399,0,1,2,"));
            assert!(csv.contains("true\n"));
            assert!(csv.contains("-8.64000000000000000e4"));
            assert!(csv.contains("8.64000000000000000e4"));
        }
    }
}
