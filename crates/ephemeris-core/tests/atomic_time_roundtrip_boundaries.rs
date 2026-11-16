// The contents of this file are subject to the Common Public Attribution License Version 1.0 (CPAL-1.0).
// See the repository LICENSE (including Exhibits A and B), NOTICE, and the
// full source notice in crates/ephemeris-core/src/time.rs.
// Original contribution: VirgoStars project, 2026-11-10.
// Modification record: 2026-11-10 — add deterministic TAI/TT boundary regressions.

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::time::{tai_to_tt, tt_to_tai, SECONDS_PER_DAY};

#[test]
fn atomic_offset_roundtrips_across_midnight_and_negative_epochs() {
    let days = [-2_000_000_i64, 0, 2_451_545, 2_460_000, 7_000_000];
    let seconds = [0.0, 0.001, 31.0, 43_200.0, 86_367.0, 86_399.999];

    for day in days {
        for second in seconds {
            let original = JulianDate::new(day, second / SECONDS_PER_DAY, TimeScale::Tai).unwrap();
            let tt = tai_to_tt(original).unwrap();
            assert_eq!(tt.scale(), TimeScale::Tt);
            let recovered = tt_to_tai(tt).unwrap();
            assert_eq!(
                recovered.day_number(),
                original.day_number(),
                "day={day}, second={second}"
            );
            let drift_seconds =
                (recovered.midnight_fraction() - original.midnight_fraction()) * SECONDS_PER_DAY;
            assert!(
                drift_seconds.abs() < 1e-8,
                "day={day}, second={second}, drift={drift_seconds} s"
            );
        }
    }
}

#[test]
fn tt_to_tai_roundtrip_preserves_midnight_boundary() {
    for second in [0.0, 0.001, 32.184, 86_399.999] {
        let original = JulianDate::new(2_451_545, second / SECONDS_PER_DAY, TimeScale::Tt).unwrap();
        let recovered = tai_to_tt(tt_to_tai(original).unwrap()).unwrap();
        assert_eq!(recovered.day_number(), original.day_number());
        let drift_seconds =
            (recovered.midnight_fraction() - original.midnight_fraction()) * SECONDS_PER_DAY;
        assert!(
            drift_seconds.abs() < 1e-8,
            "second={second}, drift={drift_seconds} s"
        );
    }
}

#[test]
fn tai_midnight_maps_to_next_tt_day_when_offset_crosses_boundary() {
    // Independent civil-clock convention: TT = TAI + 32.184 SI seconds.
    // This checks the *direction and magnitude* of the offset, not merely
    // roundtrip reversibility (which could pass with the wrong sign).
    let tai = JulianDate::new(2_451_545, 1.0 - 20.0 / SECONDS_PER_DAY, TimeScale::Tai)
        .unwrap();
    let tt = tai_to_tt(tai).unwrap();
    assert_eq!(tt.day_number(), 2_451_546);
    assert_eq!(tt.scale(), TimeScale::Tt);
    let expected_tt_seconds_after_midnight = 12.184;
    let actual = tt.midnight_fraction() * SECONDS_PER_DAY;
    assert!(
        (actual - expected_tt_seconds_after_midnight).abs() < 1e-8,
        "TT seconds after midnight: {actual}"
    );
}
