// The contents of this file are subject to the Common Public Attribution License Version 1.0 (CPAL-1.0).
// See LICENSE and NOTICE for attribution and terms.

//! Synthetic, deterministic boundary checks for the explicitly TDB-only epoch bridge.
//! These do not validate the physical TT-TDB offset or any NASA/JPL kernel.

use virgostars_ephemeris_core::{
    EphemerisError, JulianDayTdb, J2000_JD_TDB, SECONDS_PER_DAY,
};

#[test]
fn signed_tdb_day_offsets_preserve_direction_and_units() {
    for (days, expected_seconds) in [
        (-2.0, -172_800.0),
        (-0.5, -43_200.0),
        (0.0, 0.0),
        (0.5, 43_200.0),
        (2.0, 172_800.0),
    ] {
        let epoch = JulianDayTdb::new(J2000_JD_TDB + days).unwrap();
        let et = epoch.to_tdb_seconds_past_j2000().unwrap();
        assert_eq!(et.value(), expected_seconds);
    }
}

#[test]
fn finite_julian_day_that_overflows_seconds_fails_closed() {
    // The JD itself is finite and accepted; converting its magnitude to
    // seconds must not return infinity or a fabricated finite epoch.
    let epoch = JulianDayTdb::new(f64::MAX).unwrap();
    assert_eq!(
        epoch.to_tdb_seconds_past_j2000(),
        Err(EphemerisError::ArithmeticOverflow)
    );
}

#[test]
fn nonfinite_julian_days_are_rejected_before_conversion() {
    for jd in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            JulianDayTdb::new(jd),
            Err(EphemerisError::NonFinite {
                field: "TDB Julian day",
            })
        );
    }
}
