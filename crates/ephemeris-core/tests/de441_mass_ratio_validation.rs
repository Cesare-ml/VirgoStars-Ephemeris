/*
SPDX-License-Identifier: CPAL-1.0
Copyright (c) 2026 Cesare-ml (VirgoStars project).
See LICENSE and NOTICE for the applicable CPAL-1.0 notices.
*/

//! Regression checks for corrupt Earth/Moon mass ratios in classic DE441 headers.
//! Synthetic fixtures only; these tests do not establish NASA/JPL numerical parity.

use std::io::Cursor;
use virgostars_ephemeris_core::de441::{De441, De441Error};

const HEADER_SIZE: usize = 2856;
const RECORD_SIZE: usize = 3000;

fn minimal_file(ratio: f64) -> Vec<u8> {
    let mut bytes = vec![0u8; RECORD_SIZE * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    bytes[2652..2660].copy_from_slice(&1000.0f64.to_le_bytes());
    bytes[2660..2668].copy_from_slice(&1032.0f64.to_le_bytes());
    bytes[2668..2676].copy_from_slice(&32.0f64.to_le_bytes());
    bytes[2688..2696].copy_from_slice(&ratio.to_le_bytes());
    bytes[2840..2844].copy_from_slice(&441i32.to_le_bytes());
    // A valid pointer extends the record beyond the fixed header size.
    bytes[2696..2700].copy_from_slice(&373i32.to_le_bytes());
    bytes[2700..2704].copy_from_slice(&1i32.to_le_bytes());
    bytes[2704..2708].copy_from_slice(&1i32.to_le_bytes());
    assert!(RECORD_SIZE > HEADER_SIZE);
    bytes[6000..6008].copy_from_slice(&1000.0f64.to_le_bytes());
    bytes[6008..6016].copy_from_slice(&1032.0f64.to_le_bytes());
    bytes
}

#[test]
fn rejects_nonpositive_or_nonfinite_earth_moon_mass_ratios() {
    for ratio in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(
            matches!(
                De441::new(Cursor::new(minimal_file(ratio))),
                Err(De441Error::InvalidFile("invalid coverage or Earth/Moon mass ratio"))
            ),
            "ratio {ratio:?} should be rejected"
        );
    }
}
