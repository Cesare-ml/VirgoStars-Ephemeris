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

//! Direct geometric NAIF/J2000 SPK type-2 and type-3 segment states.
//!
//! A single target's highest-priority covering segment in one indexed kernel
//! is evaluated. The state remains relative to the segment's declared center;
//! there is no target-center graph composition, frame transformation,
//! light-time correction, or multiple-kernel load-order resolution.
//!
//! Reference: <https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html>
//!
//! Modified 2026-11-14: reject unresolvable record midpoints and cap the
//! directory/record midpoint tolerance below one quarter of a record.

use std::error::Error;
use std::fmt;
use std::io::{self, Read, Seek, SeekFrom};

use crate::chebyshev::{evaluate_chebyshev, evaluate_chebyshev_value};
use crate::error::EphemerisError;
use crate::spk_index::{SpkByteOrder, SpkIndex, SpkSegmentDescriptor};

const MAX_RECORD_WORDS: usize = 1_048_576;

/// Position and velocity of an SPK segment target relative to its center.
///
/// Units: fixed NAIF frame 1 (J2000/ICRF) axes, km and km/s. The instant
/// is TDB/ET seconds past J2000. No apparent or topocentric corrections.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpkDirectState {
    pub target_id: i32,
    pub center_id: i32,
    pub frame_id: i32,
    pub tdb_seconds_past_j2000: f64,
    pub position_km: [f64; 3],
    pub velocity_km_per_second: [f64; 3],
}

#[derive(Debug)]
pub enum SpkStateError {
    Io(io::Error),
    InvalidEpoch,
    MissingSegment { target_id: i32 },
    UnsupportedFrame(i32),
    UnsupportedType(i32),
    InvalidSegment(&'static str),
    Interpolation(EphemerisError),
}

impl fmt::Display for SpkStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "SPK data I/O error: {error}"),
            Self::InvalidEpoch => write!(f, "SPK TDB/ET epoch must be finite"),
            Self::MissingSegment { target_id } => {
                write!(f, "no SPK segment covers target {target_id} at this epoch")
            }
            Self::UnsupportedFrame(frame) => write!(f, "unsupported SPK frame ID {frame}"),
            Self::UnsupportedType(kind) => write!(f, "unsupported SPK data type {kind}"),
            Self::InvalidSegment(message) => write!(f, "invalid SPK segment: {message}"),
            Self::Interpolation(error) => write!(f, "SPK Chebyshev interpolation: {error}"),
        }
    }
}

impl Error for SpkStateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Interpolation(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for SpkStateError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<EphemerisError> for SpkStateError {
    fn from(error: EphemerisError) -> Self {
        Self::Interpolation(error)
    }
}

fn read_f64(bytes: &[u8], offset: usize, order: SpkByteOrder) -> f64 {
    let word: [u8; 8] = bytes[offset..offset + 8]
        .try_into()
        .expect("checked word width");
    match order {
        SpkByteOrder::LittleEndian => f64::from_le_bytes(word),
        SpkByteOrder::BigEndian => f64::from_be_bytes(word),
    }
}

fn exact_count(value: f64) -> Option<u64> {
    if value.is_finite() && value >= 1.0 && value.fract() == 0.0 && value < 9_007_199_254_740_992.0
    {
        Some(value as u64)
    } else {
        None
    }
}

fn read_words<R: Read + Seek>(
    source: &mut R,
    initial_address: u64,
    count: usize,
) -> Result<Vec<u8>, SpkStateError> {
    let offset = initial_address
        .checked_sub(1)
        .and_then(|n| n.checked_mul(8))
        .ok_or(SpkStateError::InvalidSegment("DAF word offset overflow"))?;
    let byte_length = count
        .checked_mul(8)
        .ok_or(SpkStateError::InvalidSegment("record byte count overflow"))?;
    source.seek(SeekFrom::Start(offset))?;
    let mut buffer = vec![0u8; byte_length];
    source.read_exact(&mut buffer)?;
    Ok(buffer)
}

struct RecordLayout {
    first_address: u64,
    count: u64,
    record_words: usize,
    component_coefficients: usize,
    init: f64,
    interval: f64,
}

fn record_layout<R: Read + Seek>(
    source: &mut R,
    segment: SpkSegmentDescriptor,
    order: SpkByteOrder,
) -> Result<RecordLayout, SpkStateError> {
    // INIT, INTLEN, RSIZE, N occupy the *last four 8-byte words*.
    let footer_address = segment
        .final_address
        .checked_sub(3)
        .ok_or(SpkStateError::InvalidSegment("missing SPK directory"))?;
    if footer_address <= segment.initial_address {
        return Err(SpkStateError::InvalidSegment(
            "missing SPK coefficient records",
        ));
    }
    let bytes = read_words(source, footer_address, 4)?;
    let init = read_f64(&bytes, 0, order);
    let interval = read_f64(&bytes, 8, order);
    let record_words = exact_count(read_f64(&bytes, 16, order))
        .and_then(|n| usize::try_from(n).ok())
        .filter(|n| *n <= MAX_RECORD_WORDS)
        .ok_or(SpkStateError::InvalidSegment("invalid SPK record size"))?;
    let count = exact_count(read_f64(&bytes, 24, order))
        .ok_or(SpkStateError::InvalidSegment("invalid SPK record count"))?;
    let component_count = match segment.spk_type {
        2 => 3,
        3 => 6,
        _ => return Err(SpkStateError::UnsupportedType(segment.spk_type)),
    };
    let coeffs = record_words
        .checked_sub(2)
        .filter(|n| *n >= component_count && n % component_count == 0)
        .ok_or(SpkStateError::InvalidSegment(
            "invalid SPK coefficient layout",
        ))?
        / component_count;
    let record_count_words = count
        .checked_mul(record_words as u64)
        .ok_or(SpkStateError::InvalidSegment("SPK record address overflow"))?;
    let expected_final = segment
        .initial_address
        .checked_add(record_count_words)
        .and_then(|n| n.checked_add(3))
        .ok_or(SpkStateError::InvalidSegment(
            "SPK descriptor address overflow",
        ))?;
    if expected_final != segment.final_address {
        return Err(SpkStateError::InvalidSegment(
            "SPK directory disagrees with address span",
        ));
    }
    if !init.is_finite() || !interval.is_finite() || interval <= 0.0 {
        return Err(SpkStateError::InvalidSegment("invalid SPK record interval"));
    }
    let end = init + count as f64 * interval;
    if !end.is_finite() || segment.start_tdb_seconds < init || segment.end_tdb_seconds > end {
        return Err(SpkStateError::InvalidSegment(
            "SPK record times disagree with coverage",
        ));
    }
    // Modification (2026-11-14): each record width must advance binary64 ET
    // at both ends of its declared grid. Otherwise distinct record epochs
    // collapse onto the same f64 instant, making time selection ambiguous.
    if init + interval == init || end - interval == end {
        return Err(SpkStateError::InvalidSegment(
            "SPK record interval unresolvable at f64 ET precision",
        ));
    }
    Ok(RecordLayout {
        first_address: segment.initial_address,
        count,
        record_words,
        component_coefficients: coeffs,
        init,
        interval,
    })
}

/// Select the last covering descriptor in one indexed kernel.
///
/// Centralizing this lookup ensures that direct and relative states obey
/// identical file-order priority, even for competing centers or frames.
pub(crate) fn select_covering_segment(
    index: &SpkIndex,
    target_id: i32,
    et: f64,
) -> Option<SpkSegmentDescriptor> {
    index
        .segments
        .iter()
        .rev()
        .find(|segment| {
            segment.target_id == target_id
                && et >= segment.start_tdb_seconds
                && et <= segment.end_tdb_seconds
        })
        .copied()
}

/// Evaluate the last covering segment for one target in a *single* SPK index.
///
/// The selected descriptor is checked for J2000/ICRF frame ID 1 and supported
/// SPK type **after** applying last-covering file-order precedence. A later
/// unsupported segment must fail explicitly, never fall back to an earlier
/// overlapping segment. The index must belong to the supplied source.
///
/// This returns the primitive target-minus-segment-center state; it does not
/// assemble an Earth-relative or solar-system-barycentric state from chains.
/// Type 2 differentiates position coefficients with respect to TDB seconds;
/// Type 3 evaluates independently supplied velocity coefficients. Neither
/// requires a second astronomy implementation beyond the Rust Chebyshev core.
pub fn direct_spk_state<R: Read + Seek>(
    source: &mut R,
    index: &SpkIndex,
    target_id: i32,
    et: f64,
) -> Result<SpkDirectState, SpkStateError> {
    if !et.is_finite() {
        return Err(SpkStateError::InvalidEpoch);
    }
    let segment = select_covering_segment(index, target_id, et)
        .ok_or(SpkStateError::MissingSegment { target_id })?;

    if segment.frame_id != 1 {
        return Err(SpkStateError::UnsupportedFrame(segment.frame_id));
    }
    if segment.spk_type != 2 && segment.spk_type != 3 {
        return Err(SpkStateError::UnsupportedType(segment.spk_type));
    }
    let layout = record_layout(source, segment, index.byte_order)?;
    let interval_end = layout.init + layout.count as f64 * layout.interval;
    if et < layout.init || et > interval_end {
        return Err(SpkStateError::InvalidSegment(
            "epoch outside SPK record directory",
        ));
    }
    // On an exact boundary choose the following record, except at the final
    // inclusive endpoint. Dividing (et - INIT) by INTLEN can round an epoch
    // immediately *before* a boundary up to an integer and select the wrong
    // record. Compare the original f64 ET directly against the directory's
    // representable record starts instead; do not round the epoch first.
    // Modification (2026-11-14): fail closed if a boundary overflows.
    let mut record_index = 0_u64;
    let mut upper = layout.count;
    while upper - record_index > 1 {
        let middle = record_index + (upper - record_index) / 2;
        let boundary = layout.init + middle as f64 * layout.interval;
        if !boundary.is_finite() {
            return Err(SpkStateError::InvalidSegment(
                "nonfinite SPK record boundary",
            ));
        }
        if et < boundary {
            upper = middle;
        } else {
            record_index = middle;
        }
    }
    let address = layout
        .first_address
        .checked_add(
            record_index
                .checked_mul(layout.record_words as u64)
                .ok_or(SpkStateError::InvalidSegment("SPK record offset overflow"))?,
        )
        .ok_or(SpkStateError::InvalidSegment("SPK record address overflow"))?;
    let bytes = read_words(source, address, layout.record_words)?;
    let midpoint = read_f64(&bytes, 0, index.byte_order);
    let radius = read_f64(&bytes, 8, index.byte_order);
    if !midpoint.is_finite() || !radius.is_finite() || radius <= 0.0 {
        return Err(SpkStateError::InvalidSegment(
            "invalid record midpoint or radius",
        ));
    }
    // Type 2/3 directory INIT and INTLEN define an equal-width record grid.
    // A corrupt record could otherwise shift/resize its own Chebyshev time
    // domain yet still evaluate to a plausible finite position and velocity.
    // Allow only binary64 rounding at the scale of the encoded ET/interval.
    let expected_midpoint = layout.init + (record_index as f64 + 0.5) * layout.interval;
    let expected_radius = layout.interval * 0.5;
    // The directory can advance by one interval even when its half-step
    // rounds to an endpoint at this ET magnitude. Such a record has no
    // representable interior midpoint and cannot be interpolated safely.
    let record_start = layout.init + record_index as f64 * layout.interval;
    let record_end = layout.init + (record_index as f64 + 1.0) * layout.interval;
    if !expected_midpoint.is_finite()
        || !record_start.is_finite()
        || !record_end.is_finite()
        || expected_midpoint <= record_start
        || expected_midpoint >= record_end
    {
        return Err(SpkStateError::InvalidSegment(
            "SPK record midpoint unresolvable at f64 ET precision",
        ));
    }
    let epoch_scale = layout
        .init
        .abs()
        .max(expected_midpoint.abs())
        .max(layout.interval.abs());
    // Pure relative tolerance grows with |INIT| and can otherwise exceed
    // the whole record width at extreme finite ET. A >25%-record midpoint
    // shift cannot be justified as harmless binary64 rounding.
    let midpoint_tolerance =
        (32.0 * f64::EPSILON * epoch_scale).min(0.25 * layout.interval);
    let radius_tolerance = 32.0 * f64::EPSILON * layout.interval.abs();
    if !expected_midpoint.is_finite()
        || expected_radius <= 0.0
        || (midpoint - expected_midpoint).abs() > midpoint_tolerance
        || (radius - expected_radius).abs() > radius_tolerance
    {
        return Err(SpkStateError::InvalidSegment(
            "SPK record midpoint/radius disagree with directory",
        ));
    }
    let x = (et - midpoint) / radius;
    if !x.is_finite() || !(-1.0..=1.0).contains(&x) {
        return Err(SpkStateError::InvalidSegment(
            "epoch outside Chebyshev record",
        ));
    }
    let components = if segment.spk_type == 2 { 3 } else { 6 };
    let mut values = [0.0; 6];
    let mut derivatives = [0.0; 3];
    for component in 0..components {
        let start = 2 + component * layout.component_coefficients;
        let mut coeffs = Vec::with_capacity(layout.component_coefficients);
        for i in 0..layout.component_coefficients {
            coeffs.push(read_f64(&bytes, (start + i) * 8, index.byte_order));
        }
        if segment.spk_type == 2 {
            // Type 2 has positions only: its velocity is the derivative.
            let sample = evaluate_chebyshev(&coeffs, x)?;
            values[component] = sample.value;
            derivatives[component] = sample.derivative_dx / radius;
        } else {
            // Type 3 stores position and velocity independently. Never
            // require an unused derivative to be representable.
            values[component] = evaluate_chebyshev_value(&coeffs, x)?;
        }
    }
    let position_km = [values[0], values[1], values[2]];
    let velocity_km_per_second = if segment.spk_type == 2 {
        derivatives
    } else {
        [values[3], values[4], values[5]]
    };
    if position_km
        .iter()
        .chain(velocity_km_per_second.iter())
        .any(|v| !v.is_finite())
    {
        return Err(SpkStateError::InvalidSegment("nonfinite SPK state"));
    }
    Ok(SpkDirectState {
        target_id: segment.target_id,
        center_id: segment.center_id,
        frame_id: segment.frame_id,
        tdb_seconds_past_j2000: et,
        position_km,
        velocity_km_per_second,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spk_index::inspect_spk;
    use std::io::Cursor;

    const RECORD_BYTES: usize = 1024;
    const FIRST_WORD: usize = 513;

    fn put_i32(bytes: &mut [u8], at: usize, value: i32, order: SpkByteOrder) {
        let data = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[at..at + 4].copy_from_slice(&data);
    }

    fn put_f64(bytes: &mut [u8], at: usize, value: f64, order: SpkByteOrder) {
        let data = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[at..at + 8].copy_from_slice(&data);
    }

    fn make_kernel(order: SpkByteOrder, kind: i32) -> Vec<u8> {
        let components = if kind == 2 { 3 } else { 6 };
        let rsize = 2 + 2 * components;
        let final_address = FIRST_WORD + 2 * rsize + 3;
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
        put_f64(&mut bytes, summary + 24, 0.0, order);
        put_f64(&mut bytes, summary + 32, 20.0, order);
        for (i, value) in [499, 10, 1, kind, FIRST_WORD as i32, final_address as i32]
            .into_iter()
            .enumerate()
        {
            put_i32(&mut bytes, summary + 40 + i * 4, value, order);
        }
        for record in 0..2 {
            let base = (FIRST_WORD - 1 + record * rsize) * 8;
            put_f64(&mut bytes, base, 5.0 + 10.0 * record as f64, order);
            put_f64(&mut bytes, base + 8, 5.0, order);
            // Type-2: derivative of position is +/-2 km/s.
            // Type-3: independent velocity coefficients intentionally differ.
            let coeffs: &[[f64; 2]] = if kind == 2 {
                if record == 0 {
                    &[[100.0, 10.0], [20.0, 0.0], [30.0, 0.0]]
                } else {
                    &[[200.0, -10.0], [40.0, 0.0], [50.0, 0.0]]
                }
            } else if record == 0 {
                &[
                    [100.0, 10.0],
                    [20.0, 0.0],
                    [30.0, 0.0],
                    [7.0, 1.0],
                    [8.0, 0.0],
                    [-3.0, 0.0],
                ]
            } else {
                &[
                    [200.0, -10.0],
                    [40.0, 0.0],
                    [50.0, 0.0],
                    [7.0, 1.0],
                    [8.0, 0.0],
                    [-3.0, 0.0],
                ]
            };
            for (i, item) in coeffs.iter().enumerate() {
                for (j, value) in item.iter().enumerate() {
                    put_f64(&mut bytes, base + (2 + i * 2 + j) * 8, *value, order);
                }
            }
        }
        let directory = (final_address - 4) * 8;
        for (i, v) in [0.0, 10.0, rsize as f64, 2.0].into_iter().enumerate() {
            put_f64(&mut bytes, directory + 8 * i, v, order);
        }
        bytes
    }

    fn evaluate(bytes: Vec<u8>, target: i32, et: f64) -> Result<SpkDirectState, SpkStateError> {
        let mut reader = Cursor::new(bytes);
        let index = inspect_spk(&mut reader).unwrap();
        direct_spk_state(&mut reader, &index, target, et)
    }

    #[test]
    fn type2_derivative_km_per_second_in_both_endiannesses() {
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            let s = evaluate(make_kernel(order, 2), 499, 5.0).unwrap();
            assert_eq!((s.target_id, s.center_id, s.frame_id), (499, 10, 1));
            assert_eq!(s.position_km, [100.0, 20.0, 30.0]);
            assert_eq!(s.velocity_km_per_second, [2.0, 0.0, 0.0]);
            assert_eq!(s.tdb_seconds_past_j2000, 5.0);
        }
    }

    // Modification (2026-11-14): a type-3 segment needs only the six
    // polynomial values, never the derivative of the velocity polynomial.
    #[test]
    fn type3_does_not_reject_finite_state_for_unused_derivative_overflow() {
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            // Repack the existing synthetic Type-3 fixture as one 4-term
            // record covering [0, 10] TDB/ET seconds.
            let mut bytes = make_kernel(order, 3);
            let record_words = 2 + 6 * 4;
            let final_address = FIRST_WORD + record_words + 3;
            let summary = 2 * RECORD_BYTES;
            put_f64(&mut bytes, summary + 32, 10.0, order);
            put_i32(
                &mut bytes,
                summary + 40 + 5 * 4,
                final_address as i32,
                order,
            );
            put_i32(&mut bytes, 84, (final_address + 1) as i32, order);
            let record_base = (FIRST_WORD - 1) * 8;
            let coefficients = [
                [0.0, 0.0, 0.0, 8.0e307],
                [20.0, 0.0, 0.0, 0.0],
                [30.0, 0.0, 0.0, 0.0],
                [7.0, 0.0, 0.0, 0.0],
                [8.0, 0.0, 0.0, 0.0],
                [-3.0, 0.0, 0.0, 0.0],
            ];
            for (component, row) in coefficients.iter().enumerate() {
                for (index, &coefficient) in row.iter().enumerate() {
                    put_f64(
                        &mut bytes,
                        record_base + (2 + 4 * component + index) * 8,
                        coefficient,
                        order,
                    );
                }
            }
            let footer = (final_address - 4) * 8;
            for (index, value) in [0.0, 10.0, record_words as f64, 1.0]
                .into_iter()
                .enumerate()
            {
                put_f64(&mut bytes, footer + index * 8, value, order);
            }
            let s = evaluate(bytes, 499, 5.0).unwrap();
            assert_eq!(s.position_km, [0.0, 20.0, 30.0]);
            assert_eq!(s.velocity_km_per_second, [7.0, 8.0, -3.0]);
            assert_eq!(s.tdb_seconds_past_j2000, 5.0);
        }
    }

    #[test]
    fn type3_independent_velocity_km_per_second_in_both_endiannesses() {
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            let s = evaluate(make_kernel(order, 3), 499, 5.0).unwrap();
            assert_eq!(s.position_km, [100.0, 20.0, 30.0]);
            assert_eq!(s.velocity_km_per_second, [7.0, 8.0, -3.0]);
            let other = evaluate(make_kernel(order, 3), 499, 0.0).unwrap();
            assert_eq!(other.velocity_km_per_second, [6.0, 8.0, -3.0]);
        }
    }

    #[test]
    fn record_boundary_uses_second_record_and_final_endpoint_is_inclusive() {
        let bytes = make_kernel(SpkByteOrder::LittleEndian, 2);
        assert_eq!(
            evaluate(bytes.clone(), 499, 0.0).unwrap().position_km[0],
            90.0
        );
        let at_boundary = evaluate(bytes.clone(), 499, 10.0).unwrap();
        assert_eq!(at_boundary.position_km[0], 210.0);
        assert_eq!(at_boundary.velocity_km_per_second[0], -2.0);
        assert_eq!(evaluate(bytes, 499, 20.0).unwrap().position_km[0], 190.0);
    }

    // The smallest finite ET before zero must remain in the first record.
    // (et - INIT) / INTLEN rounds to 1.0 at this boundary and the previous
    // floor-based implementation silently selected record two instead.
    // Modification (2026-11-14): exercise both SPK types/byte orders.
    #[test]
    fn near_zero_record_boundary_uses_original_tdb_epoch() {
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            for kind in [2, 3] {
                let mut bytes = make_kernel(order, kind);
                let summary = 2 * RECORD_BYTES;
                put_f64(&mut bytes, summary + 24, -10.0, order);
                put_f64(&mut bytes, summary + 32, 10.0, order);

                let components = if kind == 2 { 3 } else { 6 };
                let record_words = 2 + 2 * components;
                for record in 0..2 {
                    let offset = (FIRST_WORD - 1 + record * record_words) * 8;
                    put_f64(
                        &mut bytes,
                        offset,
                        if record == 0 { -5.0 } else { 5.0 },
                        order,
                    );
                }
                let final_address = FIRST_WORD + 2 * record_words + 3;
                let directory = (final_address - 4) * 8;
                put_f64(&mut bytes, directory, -10.0, order);

                let just_before = -f64::from_bits(1);
                let just_after = f64::from_bits(1);
                assert_eq!(
                    ((just_before - (-10.0)) / 10.0).floor(),
                    1.0,
                    "fixture must reproduce f64 quotient rounding"
                );

                let before = evaluate(bytes.clone(), 499, just_before).unwrap();
                assert_eq!(before.position_km[0], 110.0);
                let at = evaluate(bytes.clone(), 499, 0.0).unwrap();
                assert_eq!(at.position_km[0], 210.0);
                let after = evaluate(bytes, 499, just_after).unwrap();
                assert_eq!(after.position_km[0], 210.0);
            }
        }
    }

    #[test]
    fn rejects_spk_record_grid_with_unresolvable_tdb_seconds() {
        // At 2^53 ET seconds, a 1-second record width cannot advance a
        // binary64 epoch. The 2-second two-record span remains finite and
        // would otherwise allow a plausible-looking state at the start.
        let start = 9_007_199_254_740_992.0_f64;
        let interval = 1.0;
        assert_eq!(start + interval, start);
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            for kind in [2, 3] {
                let mut bytes = make_kernel(order, kind);
                let record_words = if kind == 2 { 8 } else { 14 };
                let final_address = FIRST_WORD + 2 * record_words + 3;
                let summary = 2 * RECORD_BYTES;
                put_f64(&mut bytes, summary + 24, start, order);
                put_f64(&mut bytes, summary + 32, start + 2.0, order);
                let directory = (final_address - 4) * 8;
                put_f64(&mut bytes, directory, start, order);
                put_f64(&mut bytes, directory + 8, interval, order);
                for index in 0..2 {
                    let record = (FIRST_WORD - 1 + index * record_words) * 8;
                    let midpoint = if index == 0 { start } else { start + 2.0 };
                    put_f64(&mut bytes, record, midpoint, order);
                    put_f64(&mut bytes, record + 8, 0.5, order);
                }
                assert!(matches!(
                    evaluate(bytes, 499, start),
                    Err(SpkStateError::InvalidSegment(
                        "SPK record interval unresolvable at f64 ET precision"
                    ))
                ));
            }
        }
    }

    #[test]
    fn accepts_spk_record_grid_with_resolvable_large_tdb_epochs() {
        // At the same extreme epoch a four-second interval advances ET
        // distinctly at both ends. Keep that representable case valid.
        let start = 9_007_199_254_740_992.0_f64;
        let interval = 4.0;
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            for kind in [2, 3] {
                let mut bytes = make_kernel(order, kind);
                let record_words = if kind == 2 { 8 } else { 14 };
                let final_address = FIRST_WORD + 2 * record_words + 3;
                let summary = 2 * RECORD_BYTES;
                put_f64(&mut bytes, summary + 24, start, order);
                put_f64(&mut bytes, summary + 32, start + 2.0 * interval, order);
                let directory = (final_address - 4) * 8;
                put_f64(&mut bytes, directory, start, order);
                put_f64(&mut bytes, directory + 8, interval, order);
                for index in 0..2 {
                    let record = (FIRST_WORD - 1 + index * record_words) * 8;
                    put_f64(
                        &mut bytes,
                        record,
                        start + (index as f64 + 0.5) * interval,
                        order,
                    );
                    put_f64(&mut bytes, record + 8, interval * 0.5, order);
                }
                let first = evaluate(bytes.clone(), 499, start + 2.0).unwrap();
                let second = evaluate(bytes, 499, start + 6.0).unwrap();
                assert_eq!(first.position_km, [100.0, 20.0, 30.0]);
                assert_eq!(second.position_km, [200.0, 40.0, 50.0]);
            }
        }
    }

    #[test]
    fn missing_target_invalid_epoch_and_out_of_coverage_fail() {
        let bytes = make_kernel(SpkByteOrder::LittleEndian, 2);
        assert!(matches!(
            evaluate(bytes.clone(), 123, 5.0),
            Err(SpkStateError::MissingSegment { .. })
        ));
        assert!(matches!(
            evaluate(bytes.clone(), 499, -0.01),
            Err(SpkStateError::MissingSegment { .. })
        ));
        assert!(matches!(
            evaluate(bytes.clone(), 499, 20.01),
            Err(SpkStateError::MissingSegment { .. })
        ));
        assert!(matches!(
            evaluate(bytes, 499, f64::NAN),
            Err(SpkStateError::InvalidEpoch)
        ));
    }

    #[test]
    fn later_covering_unsupported_frame_or_type_never_falls_back() {
        let mut reader = Cursor::new(make_kernel(SpkByteOrder::LittleEndian, 2));
        let mut index = inspect_spk(&mut reader).unwrap();
        let mut overlay = index.segments[0];
        overlay.frame_id = 17;
        index.segments.push(overlay);
        assert!(matches!(
            direct_spk_state(&mut reader, &index, 499, 5.0),
            Err(SpkStateError::UnsupportedFrame(17))
        ));
        index.segments.pop();
        overlay.frame_id = 1;
        overlay.spk_type = 9;
        index.segments.push(overlay);
        assert!(matches!(
            direct_spk_state(&mut reader, &index, 499, 5.0),
            Err(SpkStateError::UnsupportedType(9))
        ));
    }

    #[test]
    fn rejects_bad_directory_count_size_interval_and_span() {
        let order = SpkByteOrder::LittleEndian;
        let valid = make_kernel(order, 2);
        let footer = (FIRST_WORD + 2 * 8 + 3 - 4) * 8;
        for (at, replacement) in [
            (footer + 8, 0.0),
            (footer + 16, 7.0),
            (footer + 24, 3.0),
            (footer + 24, f64::NAN),
            (footer, f64::INFINITY),
        ] {
            let mut bytes = valid.clone();
            put_f64(&mut bytes, at, replacement, order);
            assert!(
                matches!(
                    evaluate(bytes, 499, 5.0),
                    Err(SpkStateError::InvalidSegment(_))
                ),
                "bad directory field at {at}"
            );
        }
    }

    #[test]
    fn rejects_nonfinite_coefficients_and_invalid_record_geometry() {
        let order = SpkByteOrder::LittleEndian;
        let valid = make_kernel(order, 2);
        for (at, value) in [(FIRST_WORD * 8, 0.0), ((FIRST_WORD - 1) * 8 + 16, f64::NAN)] {
            let mut bytes = valid.clone();
            put_f64(&mut bytes, at, value, order);
            assert!(evaluate(bytes, 499, 5.0).is_err());
        }
    }

    #[test]
    fn type2_and_type3_reject_record_grid_mismatch_in_both_byte_orders() {
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            for kind in [2, 3] {
                let record_words = if kind == 2 { 8 } else { 14 };
                let valid = make_kernel(order, kind);
                for record_index in 0..2 {
                    let et = if record_index == 0 { 5.0 } else { 15.0 };
                    let offset = (FIRST_WORD - 1 + record_index * record_words) * 8;
                    for (word, wrong) in [(0, et + 1.0), (8, 4.0), (8, 6.0)] {
                        let mut bytes = valid.clone();
                        put_f64(&mut bytes, offset + word, wrong, order);
                        assert!(
                            matches!(
                                evaluate(bytes, 499, et),
                                Err(SpkStateError::InvalidSegment(
                                    "SPK record midpoint/radius disagree with directory"
                                ))
                            ),
                            "type {kind}, record {record_index}, field {word}"
                        );
                    }
                }
            }
        }
    }

    // Modification (2026-11-14): a large finite ET can have an ULP wider
    // than an appreciable fraction of the physical record duration.
    fn large_epoch_kernel(order: SpkByteOrder, kind: i32, interval: f64) -> Vec<u8> {
        let start = 9_007_199_254_740_992.0_f64; // 2^53 seconds
        let mut bytes = make_kernel(order, kind);
        let record_words = if kind == 2 { 8 } else { 14 };
        let final_address = FIRST_WORD + 2 * record_words + 3;
        let summary = 2 * RECORD_BYTES;
        put_f64(&mut bytes, summary + 24, start, order);
        put_f64(&mut bytes, summary + 32, start + 2.0 * interval, order);
        let directory = (final_address - 4) * 8;
        put_f64(&mut bytes, directory, start, order);
        put_f64(&mut bytes, directory + 8, interval, order);
        for index in 0..2 {
            let offset = (FIRST_WORD - 1 + index * record_words) * 8;
            put_f64(
                &mut bytes,
                offset,
                start + (index as f64 + 0.5) * interval,
                order,
            );
            put_f64(&mut bytes, offset + 8, interval * 0.5, order);
        }
        bytes
    }

    #[test]
    fn rejects_midpoint_shift_that_exceeds_record_fraction_but_not_old_ulp_tolerance() {
        // INTLEN=4 seconds is resolvable at 2^53, but the former
        // 32*EPS*ET tolerance exceeds 60 seconds. A two-second shift is
        // an entire half-record, not harmless rounding. At start+2 the
        // corrupted record would still produce x=-1 (a finite state).
        let start = 9_007_199_254_740_992.0_f64;
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            for kind in [2, 3] {
                let mut bytes = large_epoch_kernel(order, kind, 4.0);
                put_f64(&mut bytes, (FIRST_WORD - 1) * 8, start + 4.0, order);
                assert!(matches!(
                    evaluate(bytes, 499, start + 2.0),
                    Err(SpkStateError::InvalidSegment(
                        "SPK record midpoint/radius disagree with directory"
                    ))
                ));
            }
        }
    }

    #[test]
    fn rejects_unrepresentable_record_midpoints_on_both_sides_of_grid() {
        // INTLEN=2 advances between f64 ET boundaries at 2^53, but
        // its half-step rounds onto the record start or end. Returning
        // a value at x=0 would silently shift time inside the record.
        let start = 9_007_199_254_740_992.0_f64;
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            for kind in [2, 3] {
                let bytes = large_epoch_kernel(order, kind, 2.0);
                for et in [start, start + 2.0] {
                    assert!(matches!(
                        evaluate(bytes.clone(), 499, et),
                        Err(SpkStateError::InvalidSegment(
                            "SPK record midpoint unresolvable at f64 ET precision"
                        ))
                    ));
                }
            }
        }
    }

    #[test]
    fn type2_and_type3_accept_sub_ulp_scale_directory_roundoff() {
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            for kind in [2, 3] {
                let mut bytes = make_kernel(order, kind);
                let offset = (FIRST_WORD - 1) * 8;
                put_f64(&mut bytes, offset, 5.0 + 10.0 * f64::EPSILON, order);
                put_f64(&mut bytes, offset + 8, 5.0 + 10.0 * f64::EPSILON, order);
                let state = evaluate(bytes, 499, 5.0).unwrap();
                assert_eq!((state.target_id, state.center_id), (499, 10));
                assert!(state.position_km.iter().all(|v| v.is_finite()));
                assert!(state.velocity_km_per_second.iter().all(|v| v.is_finite()));
            }
        }
    }
}
