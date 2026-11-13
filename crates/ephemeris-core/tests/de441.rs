use std::io::Cursor;
use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};

fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// Two 32-day records. Mercury has two granules with T0, T1 and T2 terms;
// EMB and Moon use constant vectors, so Earth/Moon reconstruction is analytical.
fn fixture() -> Vec<u8> {
    let size = 3000;
    let mut b = vec![0u8; size * 4];
    let title = b"JPL Planetary Ephemeris DE441";
    b[..title.len()].copy_from_slice(title);
    put_f64(&mut b, 2652, 1000.0);
    put_f64(&mut b, 2660, 1064.0);
    put_f64(&mut b, 2668, 32.0);
    put_f64(&mut b, 2688, 81.0);
    put_i32(&mut b, 2840, 441);
    for (index, first, coefficients, granules) in
        [(0, 3, 3, 2), (2, 21, 1, 1), (9, 24, 1, 1), (12, 373, 1, 1)]
    {
        let at = if index == 12 { 2844 } else { 2696 + index * 12 };
        put_i32(&mut b, at, first);
        put_i32(&mut b, at + 4, coefficients);
        put_i32(&mut b, at + 8, granules);
    }
    for r in 0..2 {
        let base = size * (r + 2);
        put_f64(&mut b, base, 1000.0 + r as f64 * 32.0);
        put_f64(&mut b, base + 8, 1032.0 + r as f64 * 32.0);
        for granule in 0..2 {
            for axis in 0..3 {
                let at = base + (2 + (granule * 3 + axis) * 3) * 8;
                put_f64(&mut b, at, (r * 1000 + granule * 100 + axis * 10) as f64);
                put_f64(&mut b, at + 8, 2.0);
                put_f64(&mut b, at + 16, 3.0);
            }
        }
        for axis in 0..3 {
            put_f64(&mut b, base + (20 + axis) * 8, 1000.0 + axis as f64);
            put_f64(&mut b, base + (23 + axis) * 8, 82.0 * (axis + 1) as f64);
        }
    }
    b
}

#[test]
fn interpolates_position_and_velocity_at_record_and_granule_boundaries() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    for (jd, position, velocity) in [
        (1000.0, [1.0, 11.0, 21.0], -1.25),
        (1008.0, [-3.0, 7.0, 17.0], 0.25),
        (1016.0, [101.0, 111.0, 121.0], -1.25),
        (1032.0, [1001.0, 1011.0, 1021.0], -1.25),
        (1064.0, [1105.0, 1115.0, 1125.0], 1.75),
    ] {
        let state = eph
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(jd),
            )
            .unwrap();
        assert_eq!(state.position_km, position);
        assert_eq!(state.velocity_km_per_day, [velocity; 3]);
        assert_eq!(state.epoch, JulianDateTdb(jd));
    }
}

#[test]
fn reconstructs_earth_and_moon_and_respects_center() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    let jd = JulianDateTdb(1010.0);
    let earth = eph
        .state(Body::Earth, Body::SolarSystemBarycenter, jd)
        .unwrap();
    assert_eq!(earth.position_km, [999.0, 999.0, 999.0]);
    let moon = eph.state(Body::Moon, Body::Earth, jd).unwrap();
    assert_eq!(moon.position_km, [82.0, 164.0, 246.0]);
    let inverse = eph.state(Body::Earth, Body::Moon, jd).unwrap();
    assert_eq!(inverse.position_km, [-82.0, -164.0, -246.0]);
    assert_eq!(
        eph.state(Body::Sun, Body::Sun, jd).unwrap().position_km,
        [0.0; 3]
    );
}

#[test]
fn lunar_relative_state_preserves_small_offsets_with_large_barycenter() {
    let mut bytes = fixture();
    // Deliberately huge Earth-Moon barycenter; lunar offsets are much smaller
    // than one f64 spacing at this scale. The Moon series is linear.
    put_i32(&mut bytes, 2696 + 9 * 12 + 4, 2);
    for record in 0..2 {
        let base = 6000 + record * 3000;
        for axis in 0..3 {
            put_f64(&mut bytes, base + (20 + axis) * 8, 1.0e20);
            put_f64(
                &mut bytes,
                base + (23 + axis * 2) * 8,
                82.0 * (axis + 1) as f64,
            );
            put_f64(
                &mut bytes,
                base + (24 + axis * 2) * 8,
                16.0 * (axis + 1) as f64,
            );
        }
    }
    let mut eph = De441::new(Cursor::new(bytes)).unwrap();
    let epoch = JulianDateTdb(1016.0);
    let moon = eph.state(Body::Moon, Body::Earth, epoch).unwrap();
    assert_eq!(moon.position_km, [82.0, 164.0, 246.0]);
    assert_eq!(moon.velocity_km_per_day, [1.0, 2.0, 3.0]);
    let earth = eph.state(Body::Earth, Body::Moon, epoch).unwrap();
    assert_eq!(earth.position_km, [-82.0, -164.0, -246.0]);
    assert_eq!(earth.velocity_km_per_day, [-1.0, -2.0, -3.0]);
}

#[test]
fn lunar_relative_state_requires_moon_series_but_not_barycenter_series() {
    let mut bytes = fixture();
    // A geocentric Moon vector is sufficient for Earth-relative lunar states.
    for offset in [2720, 2724, 2728] {
        put_i32(&mut bytes, offset, 0);
    }
    let mut eph = De441::new(Cursor::new(bytes)).unwrap();
    let epoch = JulianDateTdb(1016.0);
    assert_eq!(
        eph.state(Body::Moon, Body::Earth, epoch)
            .unwrap()
            .position_km,
        [82.0, 164.0, 246.0]
    );
    assert!(matches!(
        eph.state(Body::Moon, Body::SolarSystemBarycenter, epoch),
        Err(De441Error::MissingSeries("Earth-Moon barycenter"))
    ));

    let mut bytes = fixture();
    for offset in [2804, 2808, 2812] {
        put_i32(&mut bytes, offset, 0);
    }
    let mut eph = De441::new(Cursor::new(bytes)).unwrap();
    assert!(matches!(
        eph.state(Body::Moon, Body::Earth, epoch),
        Err(De441Error::MissingSeries("geocentric Moon"))
    ));
}

#[test]
fn rejects_bad_input_missing_series_and_corrupt_data() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    for jd in [999.0, 1065.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            eph.state(Body::Earth, Body::Sun, JulianDateTdb(jd)),
            Err(De441Error::OutsideCoverage { .. })
        ));
    }
    assert!(matches!(
        eph.state(Body::Sun, Body::Earth, JulianDateTdb(1010.0)),
        Err(De441Error::MissingSeries(_))
    ));
    let mut bad = fixture();
    bad.truncate(5000);
    assert!(matches!(
        De441::new(Cursor::new(bad)),
        Err(De441Error::InvalidFile(_))
    ));
    let mut bad = fixture();
    put_i32(&mut bad, 2696, -1);
    assert!(matches!(
        De441::new(Cursor::new(bad)),
        Err(De441Error::InvalidFile(_))
    ));
    let mut bad = fixture();
    put_f64(&mut bad, 6000, 1001.0);
    let mut eph = De441::new(Cursor::new(bad)).unwrap();
    assert!(matches!(
        eph.state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1000.0)
        ),
        Err(De441Error::InvalidFile(_))
    ));
}

#[test]
#[ignore = "requires DE441_PATH pointing to the external JPL binary"]
fn reads_real_external_de441_when_configured() {
    let path = std::env::var_os("DE441_PATH").expect("set DE441_PATH to the external DE441 file");
    let mut eph = De441::open(path).unwrap();
    assert_eq!(
        eph.coverage(),
        (JulianDateTdb(-3027215.5), JulianDateTdb(7930192.5))
    );
    // Regression values from the previous JS reader, not independent scientific references.
    let earth = eph
        .state(
            Body::Earth,
            Body::SolarSystemBarycenter,
            JulianDateTdb(2451545.0),
        )
        .unwrap();
    for (actual, expected) in earth.position_km.into_iter().zip([
        -27566740.482811444,
        132361381.15354912,
        57418653.28625383,
    ]) {
        assert!((actual - expected).abs() < 1e-6);
    }
    for jd in [eph.coverage().0, JulianDateTdb(2451545.0), eph.coverage().1] {
        let earth = eph
            .state(Body::Earth, Body::SolarSystemBarycenter, jd)
            .unwrap();
        let moon = eph
            .state(Body::Moon, Body::SolarSystemBarycenter, jd)
            .unwrap();
        let relative = eph.state(Body::Moon, Body::Earth, jd).unwrap();
        for axis in 0..3 {
            assert!(
                (moon.position_km[axis] - earth.position_km[axis] - relative.position_km[axis])
                    .abs()
                    < 1e-6
            );
        }
    }
}

#[test]
fn rejects_non_finite_chebyshev_coefficients_without_returning_nan_states() {
    // Mercury's first granule, first axis: T0, T1 and T2 respectively.
    for (offset, invalid) in [
        (0usize, f64::NAN),
        (8, f64::INFINITY),
        (16, f64::NEG_INFINITY),
    ] {
        let mut bytes = fixture();
        put_f64(&mut bytes, 6000 + (2 * 8) + offset, invalid);
        let mut eph = De441::new(Cursor::new(bytes)).unwrap();
        assert!(matches!(
            eph.state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(1008.0)
            ),
            Err(De441Error::InvalidFile("non-finite Chebyshev coefficient"))
        ));
    }
}

#[test]
fn rejects_overflowed_interpolated_state() {
    let mut bytes = fixture();
    // Finite but extreme T2 coefficient: Chebyshev evaluation overflows
    // near the end of the first granule even though the input is finite.
    put_f64(&mut bytes, 6000 + (4 * 8), f64::MAX);
    let mut eph = De441::new(Cursor::new(bytes)).unwrap();
    assert!(matches!(
        eph.state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1015.0)
        ),
        Err(De441Error::InvalidFile("non-finite interpolated state"))
    ));
}

#[test]
fn rejects_overflow_when_subtracting_finite_barycentric_positions() {
    let mut bytes = fixture();
    // Each constant barycentric series is finite, but their difference is not.
    let large = f64::MAX * 0.75;
    put_f64(&mut bytes, 6000 + 2 * 8, large);
    put_f64(&mut bytes, 6000 + 3 * 8, 0.0);
    put_f64(&mut bytes, 6000 + 4 * 8, 0.0);
    put_f64(&mut bytes, 6000 + 20 * 8, -large);
    let mut eph = De441::new(Cursor::new(bytes)).unwrap();
    assert!(matches!(
        eph.state(
            Body::Mercury,
            Body::EarthMoonBarycenter,
            JulianDateTdb(1008.0)
        ),
        Err(De441Error::InvalidFile("non-finite relative state"))
    ));
}

#[test]
fn rejects_overflow_when_subtracting_finite_barycentric_velocities() {
    let mut bytes = fixture();
    // Two one-day records, with a separate Venus series in unused words.
    put_f64(&mut bytes, 2660, 1002.0);
    put_f64(&mut bytes, 2668, 1.0);
    put_f64(&mut bytes, 6008, 1001.0);
    put_f64(&mut bytes, 9000, 1001.0);
    put_f64(&mut bytes, 9008, 1002.0);
    put_i32(&mut bytes, 2696 + 12, 27);
    put_i32(&mut bytes, 2696 + 16, 2);
    put_i32(&mut bytes, 2696 + 20, 1);
    put_f64(&mut bytes, 6000 + 2 * 8, 0.0);
    put_f64(&mut bytes, 6000 + 3 * 8, f64::MAX * 0.1875);
    put_f64(&mut bytes, 6000 + 4 * 8, 0.0);
    put_f64(&mut bytes, 6000 + 27 * 8, -f64::MAX * 0.375);
    let mut eph = De441::new(Cursor::new(bytes)).unwrap();
    assert!(matches!(
        eph.state(Body::Mercury, Body::Venus, JulianDateTdb(1000.25)),
        Err(De441Error::InvalidFile("non-finite relative state"))
    ));
}

#[test]
fn chebyshev_velocity_agrees_with_central_position_difference_inside_granules() {
    // Independent numerical check of the derivative evaluation: finite
    // differences of positions, not the derivative recurrence under test.
    // Epochs avoid granule/record boundaries; h is expressed in TDB days.
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    let h = 1.0e-4;
    for jd in [1004.0, 1012.0, 1020.0, 1028.0, 1040.0, 1052.0] {
        let state = eph
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(jd),
            )
            .unwrap();
        let before = eph
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(jd - h),
            )
            .unwrap();
        let after = eph
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(jd + h),
            )
            .unwrap();
        for axis in 0..3 {
            let finite_difference =
                (after.position_km[axis] - before.position_km[axis]) / (2.0 * h);
            assert!(
                (state.velocity_km_per_day[axis] - finite_difference).abs() < 1.0e-7,
                "JD {jd}, axis {axis}: velocity={} km/day, finite difference={} km/day",
                state.velocity_km_per_day[axis],
                finite_difference
            );
        }
    }
}

#[test]
fn audit_record_dates_visits_both_records_and_preserves_later_state_access() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    assert_eq!(eph.audit_record_dates().unwrap(), 2);
    // The full audit leaves the cursor positioned at the end of the last
    // date pair; subsequent state lookups must still seek correctly.
    let state = eph
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1000.0),
        )
        .unwrap();
    assert_eq!(state.position_km, [1.0, 11.0, 21.0]);
}

#[test]
fn audit_rejects_corrupt_later_record_even_when_header_and_length_are_valid() {
    // Opening the file validates its header and total length, not every
    // internal record. The first record remains readable in each case.
    for (offset, invalid) in [
        (9000usize, 1033.0),
        (9008, 1063.0),
        (9000, f64::NAN),
        (9008, f64::INFINITY),
    ] {
        let mut bytes = fixture();
        put_f64(&mut bytes, offset, invalid);
        let mut eph = De441::new(Cursor::new(bytes)).unwrap();
        assert!(eph
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(1000.0)
            )
            .is_ok());
        assert!(matches!(
            eph.audit_record_dates(),
            Err(De441Error::InvalidFile("coefficient record has unexpected dates"))
        ));
    }
}

#[test]
fn audit_rejects_corrupt_first_record_without_interpolating_coefficients() {
    let mut bytes = fixture();
    put_f64(&mut bytes, 6008, 1033.0);
    let mut eph = De441::new(Cursor::new(bytes)).unwrap();
    assert!(matches!(
        eph.audit_record_dates(),
        Err(De441Error::InvalidFile("coefficient record has unexpected dates"))
    ));
}
