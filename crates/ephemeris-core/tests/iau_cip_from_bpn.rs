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

//! Original BPN→CIP adapter tests using independently published
//! SOFA/ERFA t_pnm06a and t_bpn2xy numerical matrix fixtures.
//! No IAU precession/nutation model is implemented in this adapter.

use std::f64::consts::PI;

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::eop_cip_cio::wgs84_gcrs_with_eop_corrected_iau_cip;
use virgostars_ephemeris_core::eop_polar_csv::{CelestrakEopPolarCatalog, EopPredictionPolicy};
use virgostars_ephemeris_core::eop_ut1_era::eop_ut1_era_from_utc_si_seconds;
use virgostars_ephemeris_core::iau_cip_from_bpn::{
    modeled_cip_cio_from_complete_bpn, CompleteBpnAndCioAtTt, IauCipFromBpnError,
};
use virgostars_ephemeris_core::observer_gcrs::Wgs84ObserverSite;

/// Independent ERFA t_pnm06a, TT=2400000.5 + 50123.9999,
/// GCRS→true-equator/equinox-of-date 3x3 matrix.
const ERFA_PNM06A: [[f64; 3]; 3] = [
    [
        0.999_999_583_279_420_5,
        0.837_238_277_263_096_2e-3,
        0.363_968_477_114_062_3e-3,
    ],
    [
        -0.837_253_374_474_368_4e-3,
        0.999_999_648_649_286_2,
        0.413_290_594_461_102e-4,
    ],
    [
        -0.363_933_746_962_946_5e-3,
        -0.416_337_760_591_066_4e-4,
        0.999_999_932_909_426,
    ],
];

/// Separate ERFA t_bpn2xy, X=1.093465510215479e-3,
/// Y=-4.281337229063151e-5 from the **third row**.
const ERFA_BPN2XY: [[f64; 3]; 3] = [
    [
        9.999_962_358_680_738e-1,
        -2.516_417_057_665_452e-3,
        -1.093_569_785_342_37e-3,
    ],
    [
        2.516_462_370_370_876e-3,
        9.999_968_329_010_883e-1,
        4.006_159_587_358_31e-5,
    ],
    [
        1.093_465_510_215_479e-3,
        -4.281_337_229_063_151e-5,
        9.999_994_012_499_173e-1,
    ],
];

fn tt() -> JulianDate {
    JulianDate::new(2_450_124, 0.9999, TimeScale::Tt).unwrap()
}

fn supplied<'a>(
    matrix: [[f64; 3]; 3],
    epoch: JulianDate,
    q: f64,
    source: &'a str,
) -> CompleteBpnAndCioAtTt<'a> {
    CompleteBpnAndCioAtTt {
        tt_epoch: epoch,
        gcrs_to_true_equator: matrix,
        cio_series_q_rad: q,
        source_label: source,
    }
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}; actual {actual}; tolerance {tolerance}"
    );
}

#[test]
fn independent_erfa_pnm06a_extracts_cip_signs_from_third_row() {
    let out = modeled_cip_cio_from_complete_bpn(supplied(
        ERFA_PNM06A,
        tt(),
        2.1e-8,
        "ERFA t_pnm06a matrix fixture",
    ))
    .unwrap();
    near(out.model_x, -0.363_933_746_962_946_5e-3, 1e-15);
    near(out.model_y, -0.416_337_760_591_066_4e-4, 1e-15);
    assert_eq!(out.tt_epoch, tt());
    assert_eq!(out.cio_s_plus_xy_half_rad, 2.1e-8);
    assert_eq!(out.source_label, "ERFA t_pnm06a matrix fixture");
}

#[test]
fn separate_erfa_bpn2xy_fixture_proves_row_not_column_convention() {
    let out = modeled_cip_cio_from_complete_bpn(supplied(
        ERFA_BPN2XY,
        tt(),
        0.0,
        "ERFA t_bpn2xy matrix fixture",
    ))
    .unwrap();
    near(out.model_x, 1.093_465_510_215_479e-3, 1e-15);
    near(out.model_y, -4.281_337_229_063_151e-5, 1e-15);
    assert!((out.model_x - ERFA_BPN2XY[0][2]).abs() > 0.002);
}

#[test]
fn identity_and_exact_orthogonal_rotation_preserve_north_pole_geometry() {
    let id = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let zero =
        modeled_cip_cio_from_complete_bpn(supplied(id, tt(), 0.0, "synthetic identity")).unwrap();
    assert_eq!((zero.model_x, zero.model_y), (0.0, 0.0));
    let a: f64 = 0.2;
    let rotated = [
        [a.cos(), 0.0, a.sin()],
        [0.0, 1.0, 0.0],
        [-a.sin(), 0.0, a.cos()],
    ];
    let out =
        modeled_cip_cio_from_complete_bpn(supplied(rotated, tt(), 3.1e-9, "synthetic y-rotation"))
            .unwrap();
    near(out.model_x, -a.sin(), 1e-15);
    assert_eq!(out.model_y, 0.0);
    assert_eq!(out.cio_s_plus_xy_half_rad, 3.1e-9);
}

#[test]
fn nonfinite_or_non_orthogonal_matrices_fail_closed() {
    let mut bad = ERFA_PNM06A;
    bad[0][1] = f64::NAN;
    assert_eq!(
        modeled_cip_cio_from_complete_bpn(supplied(bad, tt(), 0.0, "synthetic")),
        Err(IauCipFromBpnError::NonFiniteMatrix)
    );
    let mut bad = ERFA_PNM06A;
    bad[0][0] *= 1.001;
    assert_eq!(
        modeled_cip_cio_from_complete_bpn(supplied(bad, tt(), 0.0, "synthetic")),
        Err(IauCipFromBpnError::NonOrthogonalMatrix)
    );
    let mut bad = ERFA_PNM06A;
    bad[1] = bad[0];
    assert_eq!(
        modeled_cip_cio_from_complete_bpn(supplied(bad, tt(), 0.0, "synthetic")),
        Err(IauCipFromBpnError::NonOrthogonalMatrix)
    );
}

#[test]
fn reflection_and_south_facing_pole_are_rejected() {
    let reflection = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]];
    assert_eq!(
        modeled_cip_cio_from_complete_bpn(supplied(reflection, tt(), 0.0, "reflection")),
        Err(IauCipFromBpnError::WrongHandedness)
    );
    let south = [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]];
    assert_eq!(
        modeled_cip_cio_from_complete_bpn(supplied(south, tt(), 0.0, "south-facing")),
        Err(IauCipFromBpnError::InvalidNorthCip)
    );
}

#[test]
fn epoch_scale_series_and_provenance_are_explicit() {
    let ut1 = JulianDate::new(2_450_124, 0.9999, TimeScale::Ut1).unwrap();
    assert_eq!(
        modeled_cip_cio_from_complete_bpn(supplied(ERFA_PNM06A, ut1, 0.0, "wrong-scale")),
        Err(IauCipFromBpnError::RequiresTt(TimeScale::Ut1))
    );
    for source in ["", " ", "\t"] {
        assert_eq!(
            modeled_cip_cio_from_complete_bpn(supplied(ERFA_PNM06A, tt(), 0.0, source)),
            Err(IauCipFromBpnError::MissingSource)
        );
    }
    for q in [f64::NAN, f64::INFINITY, PI + 0.01] {
        assert_eq!(
            modeled_cip_cio_from_complete_bpn(supplied(ERFA_PNM06A, tt(), q, "invalid q")),
            Err(IauCipFromBpnError::InvalidCioSeries)
        );
    }
}

#[test]
fn modeled_cip_x_y_flow_into_existing_eop_corrected_gcrs_site() {
    // Integration is intentionally synthetic: the t_pnm06a matrix
    // values below are NOT computed at the TT epoch of 2000-01-01.
    // This proves parameter wiring, not physical CIP/EOP agreement.
    let csv = concat!(
        "DATE,MJD,X,Y,UT1-UTC,LOD,DPSI,DEPS,DX,DY,DAT,DATA_TYPE\n",
        "2000-01-01,51544,0.1,0.2,0.3553880,0,0,0,-0.000005,-0.000050,32,O\n",
        "2000-01-02,51545,0.1,0.2,0.3553880,0,0,0,-0.000005,-0.000050,32,O\n",
    );
    let eop = CelestrakEopPolarCatalog::parse_csv("synthetic model source", csv).unwrap();
    let t = eop_ut1_era_from_utc_si_seconds(&eop, 51544, 0.0, EopPredictionPolicy::ObservedOnly)
        .unwrap()
        .tt_epoch;
    let model = modeled_cip_cio_from_complete_bpn(supplied(
        ERFA_PNM06A,
        t,
        0.0,
        "ERFA matrix numbers at synthetic TT",
    ))
    .unwrap();
    let site = Wgs84ObserverSite {
        longitude_rad_east: 3.1,
        latitude_rad_north: -0.5,
        ellipsoidal_height_m: 2_500.0,
    };
    let out = wgs84_gcrs_with_eop_corrected_iau_cip(
        &eop,
        51544,
        0.0,
        EopPredictionPolicy::ObservedOnly,
        site,
        model,
    )
    .unwrap();
    near(
        out.corrected_cip_cio.corrected_x,
        ERFA_PNM06A[2][0] - 0.000_005 * PI / (180.0 * 3600.0),
        1e-16,
    );
    near(
        out.corrected_cip_cio.corrected_y,
        ERFA_PNM06A[2][1] - 0.000_050 * PI / (180.0 * 3600.0),
        1e-16,
    );
    assert!(out
        .observer
        .observer_gcrs
        .position_gcrs_km
        .iter()
        .all(|x| x.is_finite()));
}

#[test]
fn input_is_not_recast_as_unverified_nutation_or_cio_model() {
    // A complete IAU physical model cannot be inferred from matrix
    // orthogonality, model name, or constant q. The API preserves
    // these caller-controlled values without computing any series.
    let q = -4.0e-8;
    let input = supplied(ERFA_PNM06A, tt(), q, "unverified physical provenance");
    let result = modeled_cip_cio_from_complete_bpn(input).unwrap();
    assert_eq!(result.cio_s_plus_xy_half_rad, q);
    assert_eq!(result.source_label, input.source_label);
}
