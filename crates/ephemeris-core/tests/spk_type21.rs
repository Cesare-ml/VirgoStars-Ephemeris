// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

//! Type 21 SPK reader. Synthetic files use the exact DAF layout with fictional
//! numbers; the `#[ignore]` tests read a caller-supplied genuine small-body SPK
//! from `SMALL_BODY_SPK_PATH` (nothing is bundled).

use virgostars_ephemeris_core::de441::JulianDateTdb;
use virgostars_ephemeris_core::spk_type21::{SmallBodySpk, SpkType21Error};

fn f64s(out: &mut Vec<u8>, values: &[f64]) {
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}

/// One segment, `md = 4`, two difference lines with zero difference tables
/// (uniform motion): position = REFPOS + (et - TL) * REFVEL.
fn synthetic(kind: i32, frame: i32, center: i32) -> Vec<u8> {
    let md = 4usize;
    let line = 4 * md + 11;
    let mut bytes = vec![0u8; 3 * 1024];
    bytes[0..8].copy_from_slice(b"DAF/SPK ");
    bytes[8..12].copy_from_slice(&2i32.to_le_bytes());
    bytes[12..16].copy_from_slice(&6i32.to_le_bytes());
    bytes[76..80].copy_from_slice(&2i32.to_le_bytes()); // FWARD
    bytes[80..84].copy_from_slice(&2i32.to_le_bytes()); // BWARD
    bytes[88..96].copy_from_slice(b"LTL-IEEE");
    // Summary record: next 0, previous 0, one summary.
    let s = 1024;
    bytes[s..s + 8].copy_from_slice(&0.0f64.to_le_bytes());
    bytes[s + 16..s + 24].copy_from_slice(&1.0f64.to_le_bytes());
    let begin = 3 * 128 + 1;
    let end = begin + 2 * line + 2 + 2 - 1;
    bytes[s + 24..s + 32].copy_from_slice(&0.0f64.to_le_bytes());
    bytes[s + 32..s + 40].copy_from_slice(&2000.0f64.to_le_bytes());
    for (k, v) in [20_000_001, center, frame, kind, begin as i32, end as i32]
        .iter()
        .enumerate()
    {
        bytes[s + 40 + 4 * k..s + 44 + 4 * k].copy_from_slice(&v.to_le_bytes());
    }
    let mut data = Vec::new();
    for (tl, refpos, refvel) in [
        (0.0, [100.0, 200.0, 300.0], [1.0, 2.0, 3.0]),
        (1000.0, [1100.0, 2200.0, 3300.0], [-1.0, -2.0, -3.0]),
    ] {
        let mut record = vec![tl];
        record.extend((1..=md).map(|j| 100.0 * j as f64)); // G
        for k in 0..3 {
            record.push(refpos[k]);
            record.push(refvel[k]);
        }
        record.extend(std::iter::repeat_n(0.0, 3 * md)); // DT
        record.push(3.0); // KQMAX1
        record.extend([1.0, 1.0, 1.0]); // KQ
        assert_eq!(record.len(), line);
        data.extend(record);
    }
    data.extend([1000.0, 2000.0]); // final epochs
    data.extend([md as f64, 2.0]); // MAXDIM, N
    f64s(&mut bytes, &data);
    bytes
}

#[test]
fn uniform_motion_segments_are_evaluated_and_selected_by_final_epoch() {
    let spk = SmallBodySpk::from_bytes(synthetic(21, 1, 10)).unwrap();
    assert_eq!((spk.target_id(), spk.center_id()), (20_000_001, 10));
    let a = spk.state_at_et(500.0).unwrap();
    assert_eq!(a, [600.0, 1200.0, 1800.0, 1.0, 2.0, 3.0]);
    // 1000 is the final epoch of line 1 (inclusive); 1500 uses line 2.
    assert_eq!(spk.state_at_et(1000.0).unwrap()[0], 1100.0);
    let b = spk.state_at_et(1500.0).unwrap();
    assert_eq!(b, [600.0, 1200.0, 1800.0, -1.0, -2.0, -3.0]);
    // Julian-date wrapper: J2000 + 500 s, velocity per day.
    let (p, v) = spk
        .state_at(JulianDateTdb(2_451_545.0 + 500.0 / 86_400.0))
        .unwrap();
    assert!((p[0] - 600.0).abs() < 1e-4 && (v[0] - 86_400.0).abs() < 1e-6);
}

#[test]
fn no_extrapolation_and_non_finite_epochs_are_errors() {
    let spk = SmallBodySpk::from_bytes(synthetic(21, 1, 10)).unwrap();
    for et in [-0.001, 2000.001, 1.0e9] {
        assert_eq!(
            spk.state_at_et(et),
            Err(SpkType21Error::OutsideCoverage),
            "{et}"
        );
    }
    assert_eq!(spk.state_at_et(f64::NAN), Err(SpkType21Error::NonFinite));
    let (first, last) = spk.coverage_jd_tdb();
    assert_eq!(
        (first, last),
        (2_451_545.0, 2_451_545.0 + 2000.0 / 86_400.0)
    );
}

#[test]
fn other_segment_types_frames_and_damaged_files_are_rejected() {
    assert!(matches!(
        SmallBodySpk::from_bytes(synthetic(2, 1, 10)),
        Err(SpkType21Error::UnsupportedSegment("not type 21"))
    ));
    assert!(matches!(
        SmallBodySpk::from_bytes(synthetic(21, 17, 10)),
        Err(SpkType21Error::UnsupportedSegment("not the J2000 frame"))
    ));
    assert!(matches!(
        SmallBodySpk::from_bytes(vec![0u8; 100]),
        Err(SpkType21Error::InvalidFile(_))
    ));
    let mut not_daf = synthetic(21, 1, 10);
    not_daf[0..8].copy_from_slice(b"NOTADAF ");
    assert!(matches!(
        SmallBodySpk::from_bytes(not_daf),
        Err(SpkType21Error::InvalidFile(_))
    ));
    let mut big_endian = synthetic(21, 1, 10);
    big_endian[88..96].copy_from_slice(b"BIG-IEEE");
    assert!(matches!(
        SmallBodySpk::from_bytes(big_endian),
        Err(SpkType21Error::InvalidFile(_))
    ));
    let mut wrong_tail = synthetic(21, 1, 10);
    let n = wrong_tail.len();
    wrong_tail[n - 8..].copy_from_slice(&3.0f64.to_le_bytes()); // N = 3 does not fit the segment
    assert!(matches!(
        SmallBodySpk::from_bytes(wrong_tail),
        Err(SpkType21Error::InvalidFile(_))
    ));
    let truncated = synthetic(21, 1, 10)[..1500].to_vec();
    assert!(SmallBodySpk::from_bytes(truncated).is_err());
}

#[test]
fn corrupt_records_are_reported_not_evaluated() {
    let mut bytes = synthetic(21, 1, 10);
    // Zero the first step size G(1): irrelevant for KQMAX1=3 (no divisions), so make KQMAX1 invalid instead.
    let md = 4usize;
    let line = 4 * md + 11;
    let kqmax1_at = (3 * 128) * 8 + (7 + 4 * md) * 8;
    assert!(kqmax1_at + 8 < bytes.len());
    let _ = line;
    bytes[kqmax1_at..kqmax1_at + 8].copy_from_slice(&99.0f64.to_le_bytes());
    let spk = SmallBodySpk::from_bytes(bytes).unwrap();
    assert!(matches!(
        spk.state_at_et(500.0),
        Err(SpkType21Error::CorruptRecord(_))
    ));
}

fn genuine() -> Option<SmallBodySpk> {
    let path = std::env::var_os("SMALL_BODY_SPK_PATH")?;
    Some(SmallBodySpk::from_bytes(std::fs::read(path).unwrap()).unwrap())
}

#[test]
#[ignore = "requires a genuine type 21 SPK in SMALL_BODY_SPK_PATH"]
fn genuine_file_is_continuous_consistent_and_a_heliocentric_orbit() {
    let spk = genuine().expect("SMALL_BODY_SPK_PATH is not set");
    assert_eq!(spk.center_id(), 10);
    let (first, last) = spk.coverage_jd_tdb();
    assert!(first < 2_415_100.0 && last > 2_488_000.0, "{first} {last}");
    let au = 149_597_870.7;
    let mut jd = first + 1.0;
    while jd < last - 1.0 {
        let (p, v) = spk.state_at(JulianDateTdb(jd)).unwrap();
        // Velocity is the derivative of the position (exact-step centered difference).
        let h = 1.0 / 256.0;
        let (pp, _) = spk.state_at(JulianDateTdb(jd + h)).unwrap();
        let (pm, _) = spk.state_at(JulianDateTdb(jd - h)).unwrap();
        for i in 0..3 {
            let numeric = (pp[i] - pm[i]) / (2.0 * h);
            assert!(
                (numeric - v[i]).abs() < 1e-3 * v[i].abs().max(1e4),
                "axis {i} at {jd}: {numeric} vs {}",
                v[i]
            );
        }
        let r = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt() / au;
        assert!(r > 8.0 && r < 19.5, "heliocentric distance {r} AU at {jd}");
        jd += 123.456;
    }
}

#[test]
#[ignore = "requires a genuine type 21 SPK of 2060 Chiron in SMALL_BODY_SPK_PATH"]
fn chiron_osculating_elements_at_the_published_epoch_match_the_jpl_header() {
    // JPL's published elements for 2060 Chiron, solution of 2026-06-05, epoch JD 2457916.5 TDB
    // (ecliptic J2000): a = 13.64597936638133 au, e = 0.3824171374703196, i = 6.94959060447848 deg,
    // node = 209.2011323125998 deg, argument of perihelion = 339.6529896907077 deg.
    let spk = genuine().expect("SMALL_BODY_SPK_PATH is not set");
    assert_eq!(spk.target_id(), 20_002_060);
    let (p, v) = spk.state_at(JulianDateTdb(2_457_916.5)).unwrap();
    let eps = 84_381.448_f64 / 3600.0; // IAU 1976 obliquity used by the J2000 ecliptic
    let (s, c) = eps.to_radians().sin_cos();
    let rot = |x: [f64; 3]| [x[0], x[1] * c + x[2] * s, -x[1] * s + x[2] * c];
    let (r, v) = (rot(p), rot(v));
    let mu = 132_712_440_041.939_38 * 86_400.0 * 86_400.0; // GM_sun, km^3/day^2
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let rl = dot(r, r).sqrt();
    let a = 1.0 / (2.0 / rl - dot(v, v) / mu);
    let h = cross(r, v);
    let e_vec: [f64; 3] = std::array::from_fn(|i| cross(v, h)[i] / mu - r[i] / rl);
    let e = dot(e_vec, e_vec).sqrt();
    let inc = (h[2] / dot(h, h).sqrt()).acos().to_degrees();
    let node = h[0].atan2(-h[1]).to_degrees().rem_euclid(360.0);
    let n_vec = [-h[1], h[0], 0.0];
    let mut omega = (dot(n_vec, e_vec) / (dot(n_vec, n_vec).sqrt() * e))
        .acos()
        .to_degrees();
    if e_vec[2] < 0.0 {
        omega = 360.0 - omega;
    }
    let au = 149_597_870.7;
    assert!(
        (a / au - 13.645_979_366_381_33).abs() < 2e-6,
        "a = {}",
        a / au
    );
    assert!((e - 0.382_417_137_470_319_6).abs() < 2e-7, "e = {e}");
    assert!((inc - 6.949_590_604_478_48).abs() < 2e-5, "i = {inc}");
    assert!((node - 209.201_132_312_599_8).abs() < 2e-5, "node = {node}");
    assert!(
        (omega - 339.652_989_690_707_7).abs() < 2e-5,
        "omega = {omega}"
    );
}
