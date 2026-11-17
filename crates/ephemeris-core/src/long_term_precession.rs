// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification (2026-11-17): experimental Vondrak long-term mean axes.
// Modification (2026-11-17): extend the experimental guard to ancient epochs.
// Coefficient data from ERFA ltpecl/ltpequ: Copyright 2013-2023 NumFOCUS
// Foundation; BSD-3, see third_party/ERFA_LICENSE.txt and THIRD_PARTY_NOTICES.md.
//! Experimental mean equator/ecliptic of date for Julian epochs −13000..5001.
//! The input is TT JD; matrices take J2000 **mean equatorial** axes to date.
//! This model does not include frame bias, nutation or Earth rotation.
//! The extended numerical guard is not an accuracy or release guarantee.
//! Vondrak, Capitaine and Wallace (2011), A&A 534 A22, corrigendum (2012).

use crate::earth_orientation::{
    Matrix3x3, OrientationError, ARCSEC_TO_RAD, EXPERIMENTAL_MAX_JULIAN_EPOCH,
    EXPERIMENTAL_MIN_JULIAN_EPOCH, J2000_TT_JD,
};
use std::f64::consts::TAU;

const DAYS_PER_JULIAN_YEAR: f64 = 365.25;
const P_POLY: [[f64; 4]; 2] = [
    [5851.607687, -0.1189000, -0.00028913, 0.000000101],
    [-1600.886300, 1.1689818, -0.00000020, -0.000000437],
];
// Rows: period in Julian centuries, cosine P/Q, sine P/Q (arcseconds).
const P_PERIODIC: [[f64; 5]; 8] = [
    [708.15, -5486.751211, -684.661560, 667.666730, -5523.863691],
    [2309.00, -17.127623, 2446.283880, -2354.886252, -549.747450],
    [1620.00, -617.517403, 399.671049, -428.152441, -310.998056],
    [492.20, 413.442940, -356.652376, 376.202861, 421.535876],
    [1183.00, 78.614193, -186.387003, 184.778874, -36.776172],
    [622.00, -180.732815, -316.800070, 335.321713, -145.278396],
    [882.00, -87.676083, 198.296701, -185.138669, -34.744450],
    [547.00, 46.140315, 101.135679, -120.972830, 22.885731],
];
const E_POLY: [[f64; 4]; 2] = [
    [5453.282155, 0.4252841, -0.00037173, -0.000000152],
    [-73750.930350, -0.7675452, -0.00018725, 0.000000231],
];
// Rows: period, cosine X/Y, sine X/Y.
const E_PERIODIC: [[f64; 5]; 14] = [
    [256.75, -819.940624, 75004.344875, 81491.287984, 1558.515853],
    [708.15, -8444.676815, 624.033993, 787.163481, 7774.939698],
    [274.20, 2600.009459, 1251.136893, 1251.296102, -2219.534038],
    [
        241.45,
        2755.175630,
        -1102.212834,
        -1257.950837,
        -2523.969396,
    ],
    [2309.00, -167.659835, -2660.664980, -2966.799730, 247.850422],
    [492.20, 871.855056, 699.291817, 639.744522, -846.485643],
    [396.10, 44.769698, 153.167220, 131.600209, -1393.124055],
    [288.90, -512.313065, -950.865637, -445.040117, 368.526116],
    [231.10, -819.415595, 499.754645, 584.522874, 749.045012],
    [1610.00, -538.071099, -145.188210, -89.756563, 444.704518],
    [620.00, -189.793622, 558.116553, 524.429630, 235.934465],
    [157.87, -402.922932, -23.923029, -13.549067, 374.049623],
    [220.30, 179.516345, -165.405086, -210.157124, -171.330180],
    [1200.00, -9.814756, 9.344131, -44.919798, -22.899655],
];

fn series(t: f64, poly: &[[f64; 4]; 2], periodic: &[[f64; 5]]) -> [f64; 2] {
    let mut result = [0.0; 2];
    for k in 0..2 {
        result[k] = poly[k]
            .iter()
            .rev()
            .fold(0.0, |acc, coefficient| acc * t + coefficient);
    }
    for row in periodic {
        let (s, c) = (TAU * t / row[0]).sin_cos();
        result[0] += row[1] * c + row[3] * s;
        result[1] += row[2] * c + row[4] * s;
    }
    [result[0] * ARCSEC_TO_RAD, result[1] * ARCSEC_TO_RAD]
}

fn poles(epj: f64) -> ([f64; 3], [f64; 3]) {
    let t = (epj - 2000.0) / 100.0;
    let [p, q] = series(t, &P_POLY, &P_PERIODIC);
    let [x, y] = series(t, &E_POLY, &E_PERIODIC);
    let w = (1.0 - p * p - q * q).sqrt();
    let (s, c) = (84381.406 * ARCSEC_TO_RAD).sin_cos();
    let ecliptic = [p, -q * c - w * s, -q * s + w * c];
    let equator = [x, y, (1.0 - x * x - y * y).sqrt()];
    (equator, ecliptic)
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// J2000 mean equatorial axes to mean equator and ecliptic of TT date.
/// Experimental numerical guard: Julian epochs −13000 through 5001; no frame
/// bias or nutation. This range is not an accuracy guarantee.
pub fn mean_date_matrices(tt_jd: f64) -> Result<(Matrix3x3, Matrix3x3), OrientationError> {
    if !tt_jd.is_finite() {
        return Err(OrientationError::NonFinite { field: "tt_jd" });
    }
    let epj = 2000.0 + (tt_jd - J2000_TT_JD) / DAYS_PER_JULIAN_YEAR;
    if !(EXPERIMENTAL_MIN_JULIAN_EPOCH..=EXPERIMENTAL_MAX_JULIAN_EPOCH).contains(&epj) {
        return Err(OrientationError::UnsupportedEpoch { epoch_tt_jd: tt_jd });
    }
    let (equator, ecliptic) = poles(epj);
    let mut equinox = cross(equator, ecliptic);
    let norm = equinox.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !norm.is_finite() || norm <= 0.0 {
        return Err(OrientationError::SingularVector);
    }
    for value in &mut equinox {
        *value /= norm;
    }
    Ok((
        [equinox, cross(equator, equinox), equator],
        [equinox, cross(ecliptic, equinox), ecliptic],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equator_matrix_matches_erfa_reference() {
        // ERFA t_ltp at Julian epoch 1666.666; numerical conformance fixture.
        let expected = [
            [
                0.9967044141159214,
                0.074_378_018_931_932_1,
                0.03237624409345603,
            ],
            [
                -0.07437802731819618,
                0.9972293894454533,
                -0.0012057688427235933,
            ],
            [
                -0.03237622482766575,
                -0.001206286039697609,
                0.9994750246704011,
            ],
        ];
        let tt = J2000_TT_JD + (1666.666 - 2000.0) * DAYS_PER_JULIAN_YEAR;
        let actual = mean_date_matrices(tt).unwrap().0;
        for i in 0..3 {
            for j in 0..3 {
                assert!((actual[i][j] - expected[i][j]).abs() < 1e-14);
            }
        }
    }
    #[test]
    fn matrices_remain_orthonormal_over_supported_range_and_fail_closed() {
        for year in [
            -13000.0, -12999.0, -10000.0, -5000.0, 0.0, 1000.0, 2000.0, 3000.0, 4000.0, 5000.0,
            5001.0,
        ] {
            let jd = J2000_TT_JD + (year - 2000.0) * DAYS_PER_JULIAN_YEAR;
            let (a, b) = mean_date_matrices(jd).unwrap();
            for matrix in [a, b] {
                for i in 0..3 {
                    for j in 0..3 {
                        let dot: f64 = (0..3).map(|k| matrix[i][k] * matrix[j][k]).sum();
                        assert!((dot - if i == j { 1.0 } else { 0.0 }).abs() < 1e-14);
                    }
                }
            }
        }
        assert!(mean_date_matrices(f64::NAN).is_err());
        assert!(mean_date_matrices(J2000_TT_JD - 15001.0 * DAYS_PER_JULIAN_YEAR).is_err());
        assert!(mean_date_matrices(J2000_TT_JD + 3002.0 * DAYS_PER_JULIAN_YEAR).is_err());
    }

    #[test]
    #[ignore = "requires caller-generated ERFA reference CSV"]
    fn external_erfa_mean_axes_reference_six_epochs() {
        let path = std::env::var("LONG_TERM_ERFA_REFERENCE").expect("LONG_TERM_ERFA_REFERENCE");
        let data = std::fs::read_to_string(path).expect("reference CSV");
        let mut lines = data.lines();
        assert_eq!(
            lines.next(),
            Some("jd_tt,eq00,eq01,eq02,eq10,eq11,eq12,eq20,eq21,eq22,ecl0,ecl1,ecl2")
        );
        let expected_dates = [
            1721242.0, 2086484.0, 2451727.0, 2816969.0, 3182212.0, 3547454.0,
        ];
        let rows: Vec<_> = lines.collect();
        assert_eq!(rows.len(), expected_dates.len());
        let mut max_error = 0.0_f64;
        for (row, date) in rows.iter().zip(expected_dates) {
            let fields: Vec<f64> = row
                .split(',')
                .map(|x| x.parse().expect("numeric reference"))
                .collect();
            assert_eq!(fields.len(), 13);
            assert!(fields.iter().all(|value| value.is_finite()));
            assert_eq!(fields[0], date);
            let (equator, ecliptic) = mean_date_matrices(date).unwrap();
            for i in 0..3 {
                for j in 0..3 {
                    max_error = max_error.max((equator[i][j] - fields[1 + 3 * i + j]).abs());
                }
            }
            for i in 0..3 {
                max_error = max_error.max((ecliptic[2][i] - fields[10 + i]).abs());
            }
        }
        assert!(
            max_error < 2e-14,
            "maximum ERFA component residual: {max_error}"
        );
        println!("ERFA 2.0.1.5 maximum matrix/pole component residual: {max_error}");
    }

    #[test]
    #[ignore = "requires caller-generated extended ERFA reference CSV"]
    fn external_erfa_mean_axes_extended_epochs() {
        let path = std::env::var("LONG_TERM_ERFA_EXTENDED_REFERENCE")
            .expect("LONG_TERM_ERFA_EXTENDED_REFERENCE");
        let data = std::fs::read_to_string(path).expect("reference CSV");
        let mut lines = data.lines();
        assert_eq!(
            lines.next(),
            Some("jd_tt,eq00,eq01,eq02,eq10,eq11,eq12,eq20,eq21,eq22,ecl0,ecl1,ecl2")
        );
        let dates = [
            -3026546.0, -1931183.0, -104971.0, 1721242.0, 2451727.0, 3181846.0,
        ];
        let rows: Vec<_> = lines.collect();
        assert_eq!(rows.len(), dates.len());
        let mut max_error = 0.0_f64;
        for (row, date) in rows.iter().zip(dates) {
            let fields: Vec<f64> = row.split(',').map(|x| x.parse().unwrap()).collect();
            assert_eq!(fields.len(), 13);
            assert!(fields.iter().all(|value| value.is_finite()));
            assert_eq!(fields[0], date);
            let (equator, ecliptic) = mean_date_matrices(date).unwrap();
            for i in 0..3 {
                for j in 0..3 {
                    max_error = max_error.max((equator[i][j] - fields[1 + 3 * i + j]).abs());
                }
                max_error = max_error.max((ecliptic[2][i] - fields[10 + i]).abs());
            }
        }
        println!("extended ERFA max matrix/pole component residual: {max_error}");
        assert!(max_error < 2e-14);
    }

    #[test]
    #[ignore = "requires caller-generated ERFA 2000A reference CSV"]
    fn external_erfa_nutation_diagnostic_101_epochs() {
        let path =
            std::env::var("LONG_TERM_NUTATION_REFERENCE").expect("LONG_TERM_NUTATION_REFERENCE");
        let data = std::fs::read_to_string(path).expect("reference CSV");
        let mut lines = data.lines();
        assert_eq!(lines.next(), Some("jd_tt,dpsi_arcsec"));
        let rows: Vec<_> = lines.collect();
        assert_eq!(rows.len(), 101);
        let mut max_error = 0.0_f64;
        for (index, row) in rows.iter().enumerate() {
            let fields: Vec<f64> = row
                .split(',')
                .map(|x| x.parse().expect("numeric reference"))
                .collect();
            assert_eq!(fields.len(), 2);
            assert!(fields.iter().all(|value| value.is_finite()));
            let year = (index * 50) as f64;
            assert_eq!(
                fields[0],
                J2000_TT_JD + (year - 2000.0) * DAYS_PER_JULIAN_YEAR
            );
            let got =
                crate::earth_orientation::experimental_extrapolated_nutation(fields[0]).unwrap();
            max_error = max_error.max((got.d_psi_arcsec - fields[1]).abs());
        }
        println!("ERFA 2000A max longitude-nutation difference: {max_error} arcsec");
        assert!(
            max_error < 0.1,
            "predeclared diagnostic threshold: 0.1 arcsec"
        );
    }
}
