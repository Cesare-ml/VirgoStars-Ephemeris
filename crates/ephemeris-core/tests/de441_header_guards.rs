// SPDX-License-Identifier: CPAL-1.0
// Modification (2026-11-12): deterministic malformed-header regression cases.
// Modification (2026-11-13): adjacent-series and incomplete-pointer boundary cases.
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
