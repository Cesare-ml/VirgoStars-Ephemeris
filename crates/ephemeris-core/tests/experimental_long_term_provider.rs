// SPDX-License-Identifier: CPAL-1.0
use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    experimental_geocentric_apparent_with_provider, experimental_long_term_apparent_with_provider,
    ApparentFrame, ExperimentalLongTermStateProvider, ExperimentalProviderApparentError,
};
use virgostars_ephemeris_core::de441::{
    Body, JulianDateTdb, StateVector, StateVectorWithAcceleration,
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
fn standard_frame_provider_preserves_missing_file_errors() {
    let mut missing = provider();
    missing.fail_next = true;
    assert!(matches!(
        experimental_geocentric_apparent_with_provider(
            &mut missing,
            Body::MarsBarycenter,
            JulianDateTdb(BOUNDARY - 1.0 / 86400.0),
            ApparentFrame::TrueEquatorOfDate,
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
