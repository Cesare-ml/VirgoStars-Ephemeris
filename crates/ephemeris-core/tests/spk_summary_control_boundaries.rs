/*
EXHIBIT A. Common Public Attribution License Version 1.0.
The contents of this file are subject to the Common Public Attribution License
Version 1.0 (CPAL-1.0). The Original Developer is Cesare-ml (VirgoStars
project), copyright (c) 2026 Cesare-ml. See LICENSE and NOTICE for the full
Exhibit A, Exhibit B, and attribution obligations.
SPDX-License-Identifier: CPAL-1.0
*/

//! Deterministic DAF summary-control floating-point boundary regressions.
//! Synthetic metadata only: not independent NASA/JPL numerical validation.

use std::io::Cursor;
use virgostars_ephemeris_core::spk_index::{inspect_spk, SpkIndexError};

const RECORD: usize = 1024;
const SUMMARY: usize = 2 * RECORD;

fn put_i32(bytes: &mut [u8], offset: usize, value: i32, big: bool) {
    let raw = if big { value.to_be_bytes() } else { value.to_le_bytes() };
    bytes[offset..offset + 4].copy_from_slice(&raw);
}

fn put_f64(bytes: &mut [u8], offset: usize, value: f64, big: bool) {
    let raw = if big { value.to_be_bytes() } else { value.to_le_bytes() };
    bytes[offset..offset + 8].copy_from_slice(&raw);
}

fn minimal_spk(big: bool) -> Vec<u8> {
    // File, reserved, summary, names, coefficient data.
    let mut bytes = vec![0; 5 * RECORD];
    bytes[..8].copy_from_slice(b"DAF/SPK ");
    bytes[88..96].copy_from_slice(if big { b"BIG-IEEE" } else { b"LTL-IEEE" });
    put_i32(&mut bytes, 8, 2, big); // ND
    put_i32(&mut bytes, 12, 6, big); // NI
    put_i32(&mut bytes, 76, 3, big); // first summary
    put_i32(&mut bytes, 80, 3, big); // last summary
    put_i32(&mut bytes, 84, 517, big); // first free word
    put_f64(&mut bytes, SUMMARY + 16, 1.0, big); // summary count
    let descriptor = SUMMARY + 24;
    put_f64(&mut bytes, descriptor, 0.0, big);
    put_f64(&mut bytes, descriptor + 8, 100.0, big);
    for (index, value) in [399, 0, 1, 2, 513, 516].iter().enumerate() {
        put_i32(&mut bytes, descriptor + 16 + index * 4, *value, big);
    }
    bytes
}

#[test]
fn rejects_unrepresentable_or_nonintegral_summary_controls_in_both_byte_orders() {
    // 2^53 is the first integer outside the accepted exact-address range.
    // Fractional, negative, NaN and infinite controls must also be rejected
    // before interpreting any as pointers or a loop bound.
    let invalid = [
        (9_007_199_254_740_992.0, "2^53"),
        (1.5, "fractional"),
        (-1.0, "negative"),
        (f64::NAN, "NaN"),
        (f64::INFINITY, "positive infinity"),
        (f64::NEG_INFINITY, "negative infinity"),
    ];
    for big in [false, true] {
        assert_eq!(
            inspect_spk(&mut Cursor::new(minimal_spk(big)))
                .expect("valid synthetic SPK")
                .segments
                .len(),
            1
        );
        for (offset, message) in [
            (0, "invalid next summary pointer"),
            (8, "invalid previous summary pointer"),
            (16, "invalid DAF summary count"),
        ] {
            for (value, case) in invalid {
                let mut bytes = minimal_spk(big);
                put_f64(&mut bytes, SUMMARY + offset, value, big);
                assert!(
                    matches!(
                        inspect_spk(&mut Cursor::new(bytes)),
                        Err(SpkIndexError::InvalidFile(reason)) if reason == message
                    ),
                    "byte_order_big={big}, field_offset={offset}, case={case}"
                );
            }
        }
    }
}
