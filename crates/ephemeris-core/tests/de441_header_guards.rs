// SPDX-License-Identifier: CPAL-1.0
// Modification (2026-11-12): deterministic malformed-header regression cases.
// Modification (2026-11-13): adjacent-series and incomplete-pointer boundary cases.
// Modification (2026-11-13): reject mismatched DE identity and truncated headers.
// Modification (2026-11-13): degenerate coverage and fractional record counts.
// Modification (2026-11-13): guard extreme coefficient-pointer integers.
// Modification (2026-11-13): align extreme first-word expectation with length validation.
// See the repository LICENSE, NOTICE and CPAL Exhibit A.
use std::io::Cursor;
use virgostars_ephemeris_core::de441::{De441, De441Error};

fn f64_field(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn i32_field(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// Header and declared length are internally consistent. No external JPL data.
fn synthetic_header() -> Vec<u8> {
    let mut bytes = vec![0; 4 * 3000];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    f64_field(&mut bytes, 2652, 1000.0);
    f64_field(&mut bytes, 2660, 1064.0);
    f64_field(&mut bytes, 2668, 32.0);
    f64_field(&mut bytes, 2688, 81.0);
    i32_field(&mut bytes, 2840, 441);
    // A harmless pointer near the end of a 3000-byte record.
    i32_field(&mut bytes, 2844, 373);
    i32_field(&mut bytes, 2848, 1);
    i32_field(&mut bytes, 2852, 1);
    bytes
}

#[test]
fn rejects_nonfinite_header_numbers() {
    for (offset, value) in [
        (2652, f64::NAN),
        (2660, f64::INFINITY),
        (2668, f64::NEG_INFINITY),
        (2688, f64::NAN),
    ] {
        let mut bytes = synthetic_header();
        f64_field(&mut bytes, offset, value);
        assert!(
            matches!(
                De441::new(Cursor::new(bytes)),
                Err(De441Error::InvalidFile(
                    "invalid coverage or Earth/Moon mass ratio"
                ))
            ),
            "offset {offset}"
        );
    }
}

#[test]
fn rejects_invalid_coverage_step_and_mass_ratio() {
    for (offset, value) in [
        (2660, 1000.0),
        (2668, 0.0),
        (2668, -32.0),
        (2688, 0.0),
        (2688, -81.0),
    ] {
        let mut bytes = synthetic_header();
        f64_field(&mut bytes, offset, value);
        assert!(
            matches!(
                De441::new(Cursor::new(bytes)),
                Err(De441Error::InvalidFile(
                    "invalid coverage or Earth/Moon mass ratio"
                ))
            ),
            "offset {offset}, value {value}"
        );
    }
}

#[test]
fn rejects_fractional_record_count_and_incorrect_length() {
    let mut bytes = synthetic_header();
    f64_field(&mut bytes, 2660, 1065.0);
    assert!(matches!(
        De441::new(Cursor::new(bytes)),
        Err(De441Error::InvalidFile("invalid record count or size"))
    ));

    let mut bytes = synthetic_header();
    bytes.push(0);
    assert!(matches!(
        De441::new(Cursor::new(bytes)),
        Err(De441Error::InvalidFile("file length does not match header"))
    ));
}

#[test]
fn accepts_consistent_synthetic_header() {
    let reader = De441::new(Cursor::new(synthetic_header())).unwrap();
    let (start, end) = reader.coverage();
    assert_eq!((start.0, end.0), (1000.0, 1064.0));
}

#[test]
fn rejects_negative_and_partial_coefficient_pointers() {
    for (offset, value) in [
        (2844, -1),
        (2844, 2),
        (2848, 0),
        (2848, 101),
        (2852, 0),
        (2852, 101),
    ] {
        let mut bytes = synthetic_header();
        i32_field(&mut bytes, offset, value);
        assert!(
            matches!(
                De441::new(Cursor::new(bytes)),
                Err(De441Error::InvalidFile("invalid coefficient pointer"))
            ),
            "offset {offset}, value {value}"
        );
    }
}

#[test]
fn rejects_overlapping_coefficient_series() {
    let mut bytes = synthetic_header();
    // Series 0 occupies coefficient words [373, 376). Give series 1
    // a valid pointer whose three words overlap that interval.
    i32_field(&mut bytes, 2696, 373);
    i32_field(&mut bytes, 2700, 1);
    i32_field(&mut bytes, 2704, 1);
    assert!(matches!(
        De441::new(Cursor::new(bytes)),
        Err(De441Error::InvalidFile("overlapping coefficient pointers"))
    ));
}

#[test]
fn accepts_adjacent_nonoverlapping_coefficient_series() {
    let mut bytes = synthetic_header();
    // Series 1 occupies [370, 373), immediately before series 0 [373, 376).
    // This must not be rejected as an overlap, and the record size is unchanged.
    i32_field(&mut bytes, 2696, 370);
    i32_field(&mut bytes, 2700, 1);
    i32_field(&mut bytes, 2704, 1);
    assert!(De441::new(Cursor::new(bytes)).is_ok());
}

#[test]
fn rejects_incomplete_zero_first_pointer() {
    for (offset, value) in [(2848, 1), (2852, 1)] {
        let mut bytes = synthetic_header();
        i32_field(&mut bytes, 2844, 0);
        i32_field(&mut bytes, 2848, 0);
        i32_field(&mut bytes, 2852, 0);
        i32_field(&mut bytes, offset, value);
        assert!(matches!(
            De441::new(Cursor::new(bytes)),
            Err(De441Error::InvalidFile("invalid coefficient pointer"))
        ));
    }
}

#[test]
fn rejects_mismatched_classic_de441_identity() {
    let mut wrong_title = synthetic_header();
    wrong_title[0] = b'X';
    assert!(matches!(
        De441::new(Cursor::new(wrong_title)),
        Err(De441Error::InvalidFile(
            "expected classic little-endian JPL DE441"
        ))
    ));

    let mut wrong_version = synthetic_header();
    i32_field(&mut wrong_version, 2840, 440);
    assert!(matches!(
        De441::new(Cursor::new(wrong_version)),
        Err(De441Error::InvalidFile(
            "expected classic little-endian JPL DE441"
        ))
    ));
}

#[test]
fn rejects_truncated_header_without_panicking() {
    // Every size below the fixed 2856-byte header must fail with I/O,
    // before any indexed field access or record-size calculation.
    for length in [0, 1, 2652, 2840, 2855] {
        let bytes = synthetic_header()[..length].to_vec();
        assert!(
            matches!(De441::new(Cursor::new(bytes)), Err(De441Error::Io(_))),
            "truncated length {length}"
        );
    }
}

#[test]
fn rejects_degenerate_coverage_and_fractional_record_count() {
    for (start, end, expected) in [
        (1000.0, 1000.0, "invalid coverage or Earth/Moon mass ratio"),
        (1064.0, 1000.0, "invalid coverage or Earth/Moon mass ratio"),
        (1000.0, 1016.0, "invalid record count or size"),
    ] {
        let mut bytes = synthetic_header();
        f64_field(&mut bytes, 2652, start);
        f64_field(&mut bytes, 2660, end);
        let result = De441::new(Cursor::new(bytes));
        assert!(
            matches!(result, Err(De441Error::InvalidFile(message)) if message == expected),
            "coverage {start}..{end}"
        );
    }
}

#[test]
fn rejects_extreme_coefficient_pointer_values_without_panicking() {
    // Integer extremes must fail validation before pointer arithmetic can
    // address coefficient storage outside the declared classic record.
    for (offset, value) in [
        (2844, i32::MAX),
        (2848, i32::MAX),
        (2852, i32::MAX),
        (2848, i32::MIN),
        (2852, i32::MIN),
    ] {
        let mut bytes = synthetic_header();
        i32_field(&mut bytes, offset, value);
        let result = De441::new(Cursor::new(bytes));
        // A positive first-word index is structurally valid; the declared
        // record length must still match the actual file. Coefficient counts
        // and granule counts outside 1..=100 fail earlier.
        let expected = if offset == 2844 {
            "file length does not match header"
        } else {
            "invalid coefficient pointer"
        };
        assert!(
            matches!(result, Err(De441Error::InvalidFile(message)) if message == expected),
            "offset {offset}, value {value}"
        );
    }
}
