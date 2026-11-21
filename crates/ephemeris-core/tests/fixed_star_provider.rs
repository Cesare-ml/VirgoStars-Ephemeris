// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Synthetic provider routing; no external ephemeris or catalog.
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};
use virgostars_ephemeris_core::fixed_star::{
    calculate_star_with_provider, experimental_long_term_star_direction_with_provider,
    experimental_long_term_star_with_provider, star_direction_with_provider, FixedStarCatalogEntry,
    FixedStarError, StarStateProvider, StellarSolarDeflection,
};

#[derive(Default)]
struct Provider {
    epochs: Vec<f64>,
    fail_before: Option<f64>,
    wrong_center: bool,
}

#[test]
fn long_term_stars_use_extended_orientation_and_remain_close_near_j2000() {
    for year in [800.0, 1800.0, 2200.0] {
        let jd = 2_451_545.0 + (year - 2000.0) * 365.25;
        let mut provider = Provider::default();
        let p = experimental_long_term_star_with_provider(
            &mut provider,
            star(),
            jd,
            JulianDateTdb(jd),
            StellarSolarDeflection::PointMass,
        )
        .unwrap();
        assert!((0.0..360.0).contains(&p.longitude_deg));
        assert!((-90.0..=90.0).contains(&p.latitude_deg));
        assert!(p.longitude_speed_deg_per_tdb_day.is_finite());
        assert_eq!(provider.epochs.len(), 6);
    }
    for year in [1900.1, 2000.0, 2099.9] {
        let jd = 2_451_545.0 + (year - 2000.0) * 365.25;
        let modern = calculate_star_with_provider(
            &mut Provider::default(),
            star(),
            jd,
            JulianDateTdb(jd),
            StellarSolarDeflection::PointMass,
        )
        .unwrap();
        let extended = experimental_long_term_star_with_provider(
            &mut Provider::default(),
            star(),
            jd,
            JulianDateTdb(jd),
            StellarSolarDeflection::PointMass,
        )
        .unwrap();
        assert!((modern.longitude_deg - extended.longitude_deg).abs() < 0.001);
        assert!((modern.latitude_deg - extended.latitude_deg).abs() < 0.001);
    }
    let jd = 2_451_545.0;
    let modern = calculate_star_with_provider(
        &mut Provider::default(),
        star(),
        jd,
        JulianDateTdb(jd),
        StellarSolarDeflection::PointMass,
    )
    .unwrap();
    let mut one_epoch = Provider {
        fail_before: Some(jd),
        ..Default::default()
    };
    let direction = star_direction_with_provider(
        &mut one_epoch,
        star(),
        jd,
        JulianDateTdb(jd),
        StellarSolarDeflection::PointMass,
    )
    .unwrap();
    assert_eq!(one_epoch.epochs, vec![jd, jd]);
    assert_eq!(direction.longitude_deg, modern.longitude_deg);
    let historic_jd = 2_451_545.0 - 1200.0 * 365.25;
    let mut historical = Provider {
        fail_before: Some(historic_jd),
        ..Default::default()
    };
    let direction = experimental_long_term_star_direction_with_provider(
        &mut historical,
        star(),
        historic_jd,
        JulianDateTdb(historic_jd),
        StellarSolarDeflection::PointMass,
    )
    .unwrap();
    assert_eq!(historical.epochs, vec![historic_jd, historic_jd]);
    assert!((0.0..360.0).contains(&direction.longitude_deg));
}

impl StarStateProvider for Provider {
    fn barycentric_state(
        &mut self,
        body: Body,
        epoch: JulianDateTdb,
    ) -> Result<StateVector, FixedStarError> {
        self.epochs.push(epoch.0);
        if self.fail_before.is_some_and(|limit| epoch.0 < limit) {
            return Err(FixedStarError::InvalidGeometry);
        }
        Ok(StateVector {
            target: body,
            center: if self.wrong_center {
                Body::Earth
            } else {
                Body::SolarSystemBarycenter
            },
            epoch,
            position_km: if body == Body::Earth {
                [149_597_870.7, 0.0, 0.0]
            } else {
                [0.0; 3]
            },
            velocity_km_per_day: if body == Body::Earth {
                [0.0, 2_573_000.0, 0.0]
            } else {
                [0.0; 3]
            },
        })
    }
}

fn star() -> FixedStarCatalogEntry {
    FixedStarCatalogEntry {
        epoch_tdb: JulianDateTdb(2_451_545.0),
        right_ascension_deg: 90.0,
        declination_deg: 20.0,
        proper_motion_ra_cos_dec_mas_per_year: 0.0,
        proper_motion_dec_mas_per_year: 0.0,
        parallax_mas: 10.0,
        radial_velocity_km_per_second: 0.0,
    }
}

#[test]
fn samples_observer_and_sun_across_a_boundary() {
    let boundary = 2_451_545.0;
    let mut provider = Provider::default();
    calculate_star_with_provider(
        &mut provider,
        star(),
        boundary,
        JulianDateTdb(boundary),
        StellarSolarDeflection::PointMass,
    )
    .unwrap();
    assert_eq!(provider.epochs.len(), 6);
    assert!(provider.epochs.iter().any(|e| *e < boundary));
    assert!(provider.epochs.iter().any(|e| *e > boundary));
}

#[test]
fn missing_or_incoherent_provider_states_fail_closed() {
    let boundary = 2_451_545.0;
    let mut missing = Provider {
        fail_before: Some(boundary),
        ..Default::default()
    };
    assert!(matches!(
        calculate_star_with_provider(
            &mut missing,
            star(),
            boundary,
            JulianDateTdb(boundary),
            StellarSolarDeflection::PointMass
        ),
        Err(FixedStarError::InvalidGeometry)
    ));
    let mut wrong = Provider {
        wrong_center: true,
        ..Default::default()
    };
    assert!(matches!(
        calculate_star_with_provider(
            &mut wrong,
            star(),
            boundary,
            JulianDateTdb(boundary),
            StellarSolarDeflection::PointMass
        ),
        Err(FixedStarError::InvalidProviderState)
    ));
}
