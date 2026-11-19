// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Synthetic routing tests; no third-party ephemeris data.
use virgostars_ephemeris_core::apparent_ecliptic_of_date::ApparentFrame;
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};
use virgostars_ephemeris_core::lunar_points::{
    lunar_point_with_provider, LunarPoint, LunarPointError, LunarStateProvider,
};

#[derive(Default)]
struct SyntheticProvider {
    epochs: Vec<f64>,
    fail_before: Option<f64>,
    wrong_center: bool,
}

impl LunarStateProvider for SyntheticProvider {
    fn lunar_geocentric_state(
        &mut self,
        epoch: JulianDateTdb,
    ) -> Result<StateVector, LunarPointError> {
        self.epochs.push(epoch.0);
        if self.fail_before.is_some_and(|boundary| epoch.0 < boundary) {
            return Err(LunarPointError::EphemerisRequired);
        }
        Ok(StateVector {
            target: Body::Moon,
            center: if self.wrong_center {
                Body::Sun
            } else {
                Body::Earth
            },
            epoch,
            position_km: [384_400.0, 0.0, 5_000.0],
            velocity_km_per_day: [0.0, 80_000.0, 7_000.0],
        })
    }
}

#[test]
fn samples_retarded_and_rate_epochs_on_both_sides_of_boundary() {
    let mut provider = SyntheticProvider::default();
    let boundary = 2_451_545.0;
    lunar_point_with_provider(
        &mut provider,
        LunarPoint::TrueNode,
        ApparentFrame::TrueEclipticOfDate,
        boundary,
    )
    .unwrap();
    assert!(provider.epochs.len() >= 12);
    assert!(provider.epochs.iter().any(|epoch| *epoch < boundary));
    assert!(provider.epochs.iter().any(|epoch| *epoch > boundary));
}

#[test]
fn missing_provider_data_is_not_replaced() {
    let mut provider = SyntheticProvider {
        fail_before: Some(2_451_545.0),
        ..Default::default()
    };
    assert!(matches!(
        lunar_point_with_provider(
            &mut provider,
            LunarPoint::TrueNode,
            ApparentFrame::TrueEclipticOfDate,
            2_451_545.0
        ),
        Err(LunarPointError::EphemerisRequired)
    ));
}

#[test]
fn rejects_a_state_with_wrong_center() {
    let mut provider = SyntheticProvider {
        wrong_center: true,
        ..Default::default()
    };
    assert!(matches!(
        lunar_point_with_provider(
            &mut provider,
            LunarPoint::TrueNode,
            ApparentFrame::TrueEclipticOfDate,
            2_451_545.0
        ),
        Err(LunarPointError::InvalidProviderState)
    ));
}
