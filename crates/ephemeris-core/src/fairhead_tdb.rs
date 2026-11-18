// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification 2026-11-18: experimental full geocentric Fairhead–Bretagnon series.
//! Geocentric model only; this does not model observer topocentric terms,
//! historical UT1, or the actual physical uncertainty at ancient epochs.
//! Coefficient data are separately licensed; see `fairhead_coefficients.rs`
//! and `third_party/ERFA_LICENSE.txt`. This is ERFA model conformance, not
//! independent validation of the model itself.

use crate::fairhead_coefficients::FAIRHEAD;

/// Accepted TT range in JD: one-day margin around the mobile pack range.
pub const START_TT_JD: f64 = 1_721_058.5;
pub const END_TT_JD: f64 = 2_561_483.5;

/// Geocentric TDB−TT in seconds from the full Fairhead–Bretagnon series and
/// JPL planetary-mass adjustment. TT is substituted for formal TDB in the
/// model argument (ERFA documents this as negligible for its prediction).
/// Range is an engineering guard, not an ancient physical accuracy claim.
pub fn geocentric_tdb_minus_tt_seconds(jd_tt: f64) -> Option<f64> {
    if !jd_tt.is_finite() || !(START_TT_JD..=END_TT_JD).contains(&jd_tt) {
        return None;
    }
    let t = (jd_tt - 2_451_545.0) / 365_250.0;
    let series = |start: usize, end: usize| -> f64 {
        FAIRHEAD[start..end]
            .iter()
            .rev()
            .map(|[amplitude, frequency, phase]| amplitude * (frequency * t + phase).sin())
            .sum()
    };
    let w0 = series(0, 474);
    let w1 = series(474, 679);
    let w2 = series(679, 764);
    let w3 = series(764, 784);
    let w4 = series(784, 787);
    let fairhead = t * (t * (t * (t * w4 + w3) + w2) + w1) + w0;
    let mass = 0.00065e-6 * (6069.776754 * t + 4.021194).sin()
        + 0.00033e-6 * (213.299095 * t + 5.543132).sin()
        - 0.00196e-6 * (6208.294251 * t + 5.696701).sin()
        - 0.00173e-6 * (74.781599 * t + 2.435900).sin()
        + 0.03638e-6 * t * t;
    Some(fairhead + mass)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agrees_with_erfa_geocentric_fixtures() {
        // ERFA 2.0.1.5 dtdb(J2000, TT-J2000, 0, 0, 0, 0); seconds.
        for (jd, expected) in [
            (1_721_059.5, 0.000_914_439_798_206_387),
            (1_749_109.0, -0.001_240_994_556_620_273_3),
            (2_451_545.0, -0.000_099_307_198_943_794_47),
            (2_561_482.5, -0.000_210_546_414_545_653_73),
        ] {
            assert!((geocentric_tdb_minus_tt_seconds(jd).unwrap() - expected).abs() < 1e-9);
        }
    }

    #[test]
    fn rejects_nonfinite_and_outside_guard() {
        assert!(geocentric_tdb_minus_tt_seconds(f64::NAN).is_none());
        assert!(geocentric_tdb_minus_tt_seconds(f64::INFINITY).is_none());
        assert!(geocentric_tdb_minus_tt_seconds(START_TT_JD - 0.01).is_none());
        assert!(geocentric_tdb_minus_tt_seconds(END_TT_JD + 0.01).is_none());
        assert!(geocentric_tdb_minus_tt_seconds(START_TT_JD).is_some());
        assert!(geocentric_tdb_minus_tt_seconds(END_TT_JD).is_some());
    }
}
