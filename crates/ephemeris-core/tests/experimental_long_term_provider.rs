// SPDX-License-Identifier: CPAL-1.0
use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    experimental_long_term_apparent_with_provider, ExperimentalLongTermStateProvider,
    ExperimentalProviderApparentError,
};
use virgostars_ephemeris_core::de441::{
    Body, JulianDateTdb, StateVector, StateVectorWithAcceleration,
};
use virgostars_ephemeris_core::historical_delta_t_scenarios::{
    historical_delta_t_scenarios_with_provider, HistoricalProviderScenarioError, ScenarioSample,
};

const BOUNDARY: f64 = 2_451_545.0;
#[derive(Debug, PartialEq)]
enum Fault {
    MissingNext,
    MissingEarth,
}
struct Provider {
    fail_next: bool,
    fail_earth: bool,
    narrow: bool,
}
impl ExperimentalLongTermStateProvider for Provider {
    type Error = Fault;
    fn state(&mut self, body: Body, epoch: JulianDateTdb) -> Result<StateVector, Fault> {
        if self.fail_next && epoch.0 >= BOUNDARY {
            return Err(Fault::MissingNext);
        }
        let position = match body {
            Body::Earth => [149_597_870.7, 0.0, 0.0],
            Body::Sun => [0.0, 0.0, 0.0],
            _ => [
                250_000_000.0,
                100_000_000.0 + 1_000_000.0 * (epoch.0 - BOUNDARY),
                10_000_000.0,
            ],
        };
        Ok(StateVector {
            target: body,
            center: Body::SolarSystemBarycenter,
            epoch,
            position_km: position,
            velocity_km_per_day: if matches!(body, Body::Earth | Body::Sun) {
                [0.0; 3]
            } else {
                [0.0, 1_000_000.0, 0.0]
            },
        })
    }
    fn state_with_acceleration(
        &mut self,
        body: Body,
        epoch: JulianDateTdb,
    ) -> Result<StateVectorWithAcceleration, Fault> {
        if self.fail_earth && body == Body::Earth {
            return Err(Fault::MissingEarth);
        }
        let s = self.state(body, epoch)?;
        Ok(StateVectorWithAcceleration {
            target: s.target,
            center: s.center,
            epoch: s.epoch,
            position_km: s.position_km,
            velocity_km_per_day: s.velocity_km_per_day,
            acceleration_km_per_day2: [0.0; 3],
        })
    }
    fn logical_coverage(
        &mut self,
        epoch: JulianDateTdb,
    ) -> Result<(JulianDateTdb, JulianDateTdb), Fault> {
        Ok(if self.narrow {
            (epoch, JulianDateTdb(epoch.0 + 1.0))
        } else {
            (
                JulianDateTdb(BOUNDARY - 100.0),
                JulianDateTdb(BOUNDARY + 100.0),
            )
        })
    }
}
fn provider() -> Provider {
    Provider {
        fail_next: false,
        fail_earth: false,
        narrow: false,
    }
}

#[test]
fn produces_finite_fields_with_explicit_provider() {
    let result = experimental_long_term_apparent_with_provider(
        &mut provider(),
        Body::MarsBarycenter,
        JulianDateTdb(BOUNDARY - 0.1),
    )
    .unwrap();
    assert!(result.longitude_deg.is_finite());
    assert!(result.distance_au.is_finite() && result.distance_au > 0.0);
}
#[test]
fn preserves_provider_error_for_reception_and_deflection_stencil() {
    let mut p = provider();
    p.fail_earth = true;
    assert!(matches!(
        experimental_long_term_apparent_with_provider(
            &mut p,
            Body::MarsBarycenter,
            JulianDateTdb(BOUNDARY - 0.1)
        ),
        Err(ExperimentalProviderApparentError::Provider(
            Fault::MissingEarth
        ))
    ));
    let mut p = provider();
    p.fail_next = true;
    assert!(matches!(
        experimental_long_term_apparent_with_provider(
            &mut p,
            Body::MarsBarycenter,
            JulianDateTdb(BOUNDARY - 1.0 / 86400.0)
        ),
        Err(ExperimentalProviderApparentError::Provider(
            Fault::MissingNext
        ))
    ));
}
#[test]
fn refinement_at_inclusive_start_accepts_inward_and_rejects_outward_shifts() {
    let mut accepted = 0;
    let mut refused = 0;
    for index in 0..1000 {
        let mut p = provider();
        p.narrow = true;
        match experimental_long_term_apparent_with_provider(
            &mut p,
            Body::MarsBarycenter,
            JulianDateTdb(BOUNDARY - 0.1 + f64::from(index) * 0.000001),
        ) {
            Ok(_) => accepted += 1,
            Err(ExperimentalProviderApparentError::UnsafeRefinementBoundary) => refused += 1,
            other => panic!("unexpected result: {other:?}"),
        }
    }
    assert!(
        accepted > 0 && refused > 0,
        "accepted={accepted}, refused={refused}"
    );
}

#[test]
fn historical_scenarios_preserve_explicit_source_time_and_sample_errors() {
    let result = historical_delta_t_scenarios_with_provider(
        &mut provider(),
        Body::Sun,
        BOUNDARY - 0.1,
        64.0,
        30.0,
        "user supplied",
    )
    .unwrap();
    assert_eq!(result.delta_t_source, "user supplied");
    assert_eq!(result.lower.epoch.jd_ut1, result.upper.epoch.jd_ut1);
    assert!(
        result.lower.epoch.jd_tdb.0 < result.nominal.epoch.jd_tdb.0
            && result.nominal.epoch.jd_tdb.0 < result.upper.epoch.jd_tdb.0
    );
    for bad in ["", " source", "source\n", "\u{7f}"] {
        assert!(matches!(
            historical_delta_t_scenarios_with_provider(
                &mut provider(),
                Body::Sun,
                BOUNDARY,
                64.0,
                1.0,
                bad
            ),
            Err(HistoricalProviderScenarioError::InvalidSource)
        ));
    }
    let mut failing = provider();
    failing.fail_next = true;
    assert!(matches!(
        historical_delta_t_scenarios_with_provider(
            &mut failing,
            Body::Sun,
            BOUNDARY - 65.0 / 86400.0,
            64.0,
            10.0,
            "test"
        ),
        Err(HistoricalProviderScenarioError::Calculation {
            sample: ScenarioSample::Upper,
            source: ExperimentalProviderApparentError::Provider(Fault::MissingNext)
        })
    ));
}
