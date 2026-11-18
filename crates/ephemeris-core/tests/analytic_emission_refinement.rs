// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Sampled model-consistency tests; not independent physical validation.
use virgostars_ephemeris_core::apparent_ecliptic_of_date::experimental_apparent_icrf;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::experimental_analytic_apparent::{
    experimental_analytic_apparent_icrf_refined, ExperimentalAnalyticApparentError,
};
use virgostars_ephemeris_core::light_deflection::AnalyticDeflectionError;

fn norm(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}

#[test]
#[ignore = "requires caller-supplied DE441_PATH"]
fn refined_analytic_and_numerical_paths_share_position_and_guard() {
    let mut eph = De441::open(std::env::var("DE441_PATH").unwrap()).unwrap();
    for (target, jd) in [
        (Body::Sun, 2_451_545.0),
        (Body::Moon, 2_451_545.0),
        (Body::Mercury, 2_451_674.0),
        (Body::JupiterBarycenter, -1_474_550.0),
        (Body::UranusBarycenter, -3_026_437.0),
    ] {
        let old = experimental_apparent_icrf(&mut eph, target, JulianDateTdb(jd)).unwrap();
        let new = experimental_analytic_apparent_icrf_refined(&mut eph, target, JulianDateTdb(jd))
            .unwrap();
        let dp = norm(std::array::from_fn(|i| {
            old.position_km[i] - new.position_km[i]
        }));
        assert!(dp < 0.001, "{target:?} at {jd}: {dp} km");
        assert_eq!(old.emission_epoch, new.emission_epoch);
        assert!(new.velocity_km_per_tdb_day.iter().all(|x| x.is_finite()));
    }
    let error = experimental_analytic_apparent_icrf_refined(
        &mut eph,
        Body::UranusBarycenter,
        JulianDateTdb(-1_474_169.0782648325),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ExperimentalAnalyticApparentError::Deflection(
            AnalyticDeflectionError::NonDifferentiableFloor { .. }
        )
    ));
}

#[test]
#[ignore = "requires caller-supplied DE441_PATH"]
fn refined_analytic_rate_tracks_sampled_positions() {
    let mut eph = De441::open(std::env::var("DE441_PATH").unwrap()).unwrap();
    let h = 1.0 / 256.0;
    for (target, jd) in [
        (Body::Mercury, 2_451_674.0),
        (Body::JupiterBarycenter, -1_474_550.0),
    ] {
        let state =
            experimental_analytic_apparent_icrf_refined(&mut eph, target, JulianDateTdb(jd))
                .unwrap();
        let r = norm(state.position_km);
        let u = state.position_km.map(|x| x / r);
        let radial: f64 = u
            .iter()
            .zip(state.velocity_km_per_tdb_day)
            .map(|(x, v)| x * v)
            .sum();
        let rate: [f64; 3] =
            std::array::from_fn(|i| (state.velocity_km_per_tdb_day[i] - u[i] * radial) / r);
        let samples = [-2.0, -1.0, 1.0, 2.0].map(|offset| {
            let position = experimental_analytic_apparent_icrf_refined(
                &mut eph,
                target,
                JulianDateTdb(jd + offset * h),
            )
            .unwrap()
            .position_km;
            let length = norm(position);
            position.map(|x| x / length)
        });
        let finite: [f64; 3] = std::array::from_fn(|i| {
            (samples[0][i] - 8.0 * samples[1][i] + 8.0 * samples[2][i] - samples[3][i]) / (12.0 * h)
        });
        let difference =
            norm(std::array::from_fn(|i| rate[i] - finite[i])) * 206_264.806_247_096_36;
        assert!(
            difference < 0.0001,
            "{target:?} at {jd}: {difference} arcsec/day"
        );
    }
}
