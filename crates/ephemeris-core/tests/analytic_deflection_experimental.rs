// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Synthetic model-consistency checks; these are not physical validation.
use virgostars_ephemeris_core::de441::{Body, De441Error, JulianDateTdb, StateVector};
use virgostars_ephemeris_core::light_deflection::{
    deflect_light_time_state_by_sun_with, experimental_analytic_deflect_sun_with,
    AnalyticDeflectionError,
};
use virgostars_ephemeris_core::light_time::{light_time_with_states, GeocentricLightTimeState};
const AU: f64 = 149_597_870.7;
const J2000: f64 = 2_451_545.0;
const AS: f64 = 206_264.806_247_096_36;
fn norm(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn unit_rate(p: [f64; 3], v: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let r = norm(p);
    let u = p.map(|x| x / r);
    let radial = (0..3).map(|i| u[i] * v[i]).sum::<f64>();
    (u, std::array::from_fn(|i| (v[i] - u[i] * radial) / r))
}
fn linear(body: Body, epoch: JulianDateTdb) -> Result<StateVector, De441Error> {
    let (p, v) = match body {
        Body::Sun => ([0.0; 3], [0.0; 3]),
        Body::Earth => ([AU, 0.0, 0.0], [0.0, 2.5e6, 5e4]),
        Body::JupiterBarycenter => ([-4.0 * AU, 0.035 * AU, 0.01 * AU], [0.0, -1.0e6, 2e4]),
        _ => return Err(De441Error::MissingSeries("synthetic body")),
    };
    Ok(StateVector {
        target: body,
        center: Body::SolarSystemBarycenter,
        epoch,
        position_km: std::array::from_fn(|i| p[i] + v[i] * (epoch.0 - J2000)),
        velocity_km_per_day: v,
    })
}
fn result(jd: f64) -> GeocentricLightTimeState {
    let t = JulianDateTdb(jd);
    let lt = light_time_with_states(Body::Earth, Body::JupiterBarycenter, t, linear).unwrap();
    experimental_analytic_deflect_sun_with(&mut linear, &lt).unwrap()
}
#[test]
fn normalized_analytic_rate_differentiates_positions_and_preserves_range() {
    let center = result(J2000);
    let h = 1.0 / 64.0;
    let samples = [
        result(J2000 - 2.0 * h),
        result(J2000 - h),
        result(J2000 + h),
        result(J2000 + 2.0 * h),
    ];
    let (u, rate) = unit_rate(center.position_km, center.velocity_km_per_day);
    let vecs = samples.map(|s| unit_rate(s.position_km, s.velocity_km_per_day).0);
    let numeric: [f64; 3] = std::array::from_fn(|i| {
        (vecs[0][i] - 8.0 * vecs[1][i] + 8.0 * vecs[2][i] - vecs[3][i]) / (12.0 * h)
    });
    let error = norm(std::array::from_fn(|i| rate[i] - numeric[i])) * AS;
    assert!(error < 0.0002, "angular error {error} arcsec/day");
    let lt = light_time_with_states(
        Body::Earth,
        Body::JupiterBarycenter,
        JulianDateTdb(J2000),
        linear,
    )
    .unwrap();
    let old = deflect_light_time_state_by_sun_with(&mut linear, &lt).unwrap();
    assert!((norm(center.position_km) - norm(lt.position_km)).abs() < 1e-5);
    assert!(
        norm(std::array::from_fn(
            |i| center.position_km[i] - old.position_km[i]
        )) < 1e-5
    );
    assert!(u.iter().all(|x| x.is_finite()));
}
fn guarded(denominator: f64) -> Result<GeocentricLightTimeState, AnalyticDeflectionError> {
    let angle = (1.0 - denominator).acos();
    let q = [-angle.cos() * AU, angle.sin() * AU, 0.0];
    let epoch = JulianDateTdb(J2000);
    let emitted = JulianDateTdb(J2000 - 0.01);
    let state = GeocentricLightTimeState {
        target: Body::JupiterBarycenter,
        observation_epoch: epoch,
        emission_epoch: emitted,
        light_time_seconds: 864.0,
        light_time_rate: 0.0,
        position_km: [-2.0 * AU, 0.01 * AU, 0.0],
        velocity_km_per_day: [0.0; 3],
    };
    let mut provider = |body: Body, t: JulianDateTdb| {
        let p = match body {
            Body::Earth => [AU, 0.0, 0.0],
            Body::Sun => [0.0; 3],
            Body::JupiterBarycenter => q,
            _ => return Err(De441Error::MissingSeries("synthetic body")),
        };
        Ok(StateVector {
            target: body,
            center: Body::SolarSystemBarycenter,
            epoch: t,
            position_km: p,
            velocity_km_per_day: [0.0; 3],
        })
    };
    experimental_analytic_deflect_sun_with(&mut provider, &state)
}
#[test]
fn floor_guard_rejects_kink_but_allows_smooth_sides() {
    assert!(matches!(
        guarded(1e-6),
        Err(AnalyticDeflectionError::NonDifferentiableFloor { .. })
    ));
    assert!(guarded(1e-6 - 4e-12).is_ok());
    assert!(guarded(1e-6 + 4e-12).is_ok());
}
#[test]
fn rejects_inconsistent_provider_tags_and_invalid_state() {
    let lt = light_time_with_states(
        Body::Earth,
        Body::JupiterBarycenter,
        JulianDateTdb(J2000),
        linear,
    )
    .unwrap();
    let mut bad = |body, epoch| {
        let mut s = linear(body, epoch)?;
        s.center = Body::Earth;
        Ok(s)
    };
    assert!(matches!(
        experimental_analytic_deflect_sun_with(&mut bad, &lt),
        Err(AnalyticDeflectionError::InvalidGeometry)
    ));
    let mut invalid = lt;
    invalid.velocity_km_per_day[0] = f64::NAN;
    assert!(matches!(
        experimental_analytic_deflect_sun_with(&mut linear, &invalid),
        Err(AnalyticDeflectionError::InvalidGeometry)
    ));
    for invalid in [
        GeocentricLightTimeState {
            light_time_seconds: -1.0,
            ..lt
        },
        GeocentricLightTimeState {
            emission_epoch: lt.observation_epoch,
            ..lt
        },
        GeocentricLightTimeState {
            light_time_rate: 1.0,
            ..lt
        },
    ] {
        assert!(matches!(
            experimental_analytic_deflect_sun_with(&mut linear, &invalid),
            Err(AnalyticDeflectionError::InvalidGeometry)
        ));
    }
}

#[test]
fn sun_is_returned_unchanged_without_querying_provider() {
    let mut state = light_time_with_states(
        Body::Earth,
        Body::JupiterBarycenter,
        JulianDateTdb(J2000),
        linear,
    )
    .unwrap();
    state.target = Body::Sun;
    let mut unused =
        |_, _| -> Result<StateVector, De441Error> { panic!("Sun should not need a provider") };
    let result = experimental_analytic_deflect_sun_with(&mut unused, &state).unwrap();
    assert_eq!(result.position_km, state.position_km);
    assert_eq!(result.velocity_km_per_day, state.velocity_km_per_day);
}
