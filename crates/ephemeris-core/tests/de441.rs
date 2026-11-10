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

/// A reader that opens normally but fails on later reads, modelling removable
/// media or a damaged stream without requiring a real DE441 file.
struct FailAfterOpen {
    inner: Cursor<Vec<u8>>,
    fail_reads: std::rc::Rc<std::cell::Cell<bool>>,
}

impl std::io::Read for FailAfterOpen {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.fail_reads.get() {
            return Err(std::io::Error::other("injected record read failure"));
        }
        std::io::Read::read(&mut self.inner, buf)
    }
}

impl std::io::Seek for FailAfterOpen {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.inner, pos)
    }
}

#[test]
fn propagates_record_io_failure_without_returning_a_partial_state() {
    let fail_reads = std::rc::Rc::new(std::cell::Cell::new(false));
    let reader = FailAfterOpen {
        inner: Cursor::new(fixture()),
        fail_reads: fail_reads.clone(),
    };
    let mut eph = De441::new(reader).expect("synthetic file should open");
    fail_reads.set(true);
    let err = eph
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1010.0),
        )
        .expect_err("record read failure must not return a state");
    assert!(matches!(err, De441Error::Io(ref e) if e.kind() == std::io::ErrorKind::Other));

    // A failed read must not poison the reader's ability to serve later epochs.
    fail_reads.set(false);
    let recovered = eph
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1010.0),
        )
        .expect("reader should recover once the injected failure is removed");
    assert!(recovered.position_km.iter().all(|x| x.is_finite()));
    assert!(recovered.velocity_km_per_day.iter().all(|x| x.is_finite()));
}
