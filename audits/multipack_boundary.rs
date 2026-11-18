// SPDX-License-Identifier: CPAL-1.0
// Diagnostic only: compiles the unchanged #601 manager against experimental core.
use de441::{Body, De441, De441Error, JulianDateTdb, StateVector};
use std::{error::Error, io, path::PathBuf};
use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    experimental_long_term_apparent_ecliptic, experimental_long_term_apparent_with_provider,
    ExperimentalLongTermStateProvider, ExperimentalProviderApparentError,
};
use virgostars_ephemeris_core::light_time::{geocentric_light_time, light_time_with_states};
pub use virgostars_ephemeris_core::{calendar, de441};

#[allow(dead_code)]
mod pack_manager;
use pack_manager::{De441PackError, De441PackManager};

impl ExperimentalLongTermStateProvider for De441PackManager {
    type Error = De441PackError;
    fn state(&mut self, body: Body, epoch: JulianDateTdb) -> Result<StateVector, Self::Error> {
        De441PackManager::state(self, body, Body::SolarSystemBarycenter, epoch)
    }
    fn state_with_acceleration(
        &mut self,
        body: Body,
        epoch: JulianDateTdb,
    ) -> Result<de441::StateVectorWithAcceleration, Self::Error> {
        De441PackManager::state_with_acceleration(self, body, Body::SolarSystemBarycenter, epoch)
    }
    fn logical_coverage(
        &mut self,
        epoch: JulianDateTdb,
    ) -> Result<(JulianDateTdb, JulianDateTdb), Self::Error> {
        let s = self.select_pack(epoch)?;
        Ok((s.start_tdb, s.end_tdb))
    }
}

fn maxdiff(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}
fn fields(x: virgostars_ephemeris_core::six_field_units::FixedSixFieldsDegreesAu) -> [f64; 6] {
    [
        x.longitude_deg,
        x.latitude_deg,
        x.distance_au,
        x.longitude_speed_deg_per_tdb_day,
        x.latitude_speed_deg_per_tdb_day,
        x.distance_speed_au_per_tdb_day,
    ]
}
fn provider_lt(
    manager: &mut De441PackManager,
    body: Body,
    jd: f64,
) -> Result<
    (
        virgostars_ephemeris_core::light_time::GeocentricLightTimeState,
        Vec<(String, f64)>,
    ),
    String,
> {
    let mut trace = Vec::new();
    let mut actual_error = None;
    let result = light_time_with_states(
        Body::Earth,
        body,
        JulianDateTdb(jd),
        |target, epoch| -> Result<StateVector, De441Error> {
            let selected = match manager.select_pack(epoch) {
                Ok(spec) => spec.id.clone(),
                Err(error) => {
                    actual_error = Some(format!("{error:?}"));
                    return Err(De441Error::Io(io::Error::other("provider selection")));
                }
            };
            trace.push((selected, epoch.0));
            manager
                .state(target, Body::SolarSystemBarycenter, epoch)
                .map_err(|error| {
                    actual_error = Some(format!("{error:?}"));
                    De441Error::Io(io::Error::other("provider state"))
                })
        },
    );
    result
        .map(|x| (x, trace))
        .map_err(|err| actual_error.unwrap_or_else(|| format!("{err:?}")))
}
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<_> = std::env::args_os().collect();
    if a.len() != 5 {
        return Err("usage: EXE PACK_DIR SOURCE MISSING_PATH CORRUPT_PATH".into());
    }
    let mut manager = De441PackManager::from_mobile_directory(&a[1])?;
    let specs: Vec<_> = manager.inventory().into_iter().map(|p| p.spec).collect();
    assert_eq!(specs.len(), 10);
    let mut source = De441::open(&a[2])?;
    let bodies = [
        Body::Sun,
        Body::Moon,
        Body::Mercury,
        Body::Venus,
        Body::MarsBarycenter,
        Body::JupiterBarycenter,
        Body::SaturnBarycenter,
        Body::UranusBarycenter,
        Body::NeptuneBarycenter,
        Body::PlutoBarycenter,
    ];
    let offsets = [-0.08, -0.02, -1.0 / 86400.0, 0.0, 1.0 / 86400.0, 0.02, 0.08];
    let mut count = 0;
    let mut crossings = 0;
    let mut errors = 0;
    let mut max = [0.0_f64; 12];
    let mut continuity_max = [0.0_f64; 3];
    let mut continuity_count = 0;
    let mut provider_max = [0.0_f64; 6];
    println!("kind,boundary,jd_tdb,body,reception,emission,crossing,position_km,velocity_km_day,acceleration_km_day2,lt_position_km,lt_velocity_km_day,lt_epoch_day,lon_deg,lat_deg,distance_au,lon_rate_deg_day,lat_rate_deg_day,distance_rate_au_day");
    for i in 1..specs.len() {
        let bound = specs[i].start_tdb.0;
        assert_eq!(specs[i - 1].end_tdb.0, bound);
        let mut before = [[0.0_f64; 6]; 10];
        let mut after = [[0.0_f64; 6]; 10];
        for offset in offsets {
            let jd = bound + offset;
            let expected = if offset < 0.0 {
                &specs[i - 1]
            } else {
                &specs[i]
            };
            let selected = manager.select_pack(JulianDateTdb(jd))?;
            assert_eq!(selected.id, expected.id);
            let mut slice = De441::open(&selected.file_path)?;
            for (body_index, body) in bodies.into_iter().enumerate() {
                let p = manager.state_with_acceleration(
                    body,
                    Body::SolarSystemBarycenter,
                    JulianDateTdb(jd),
                )?;
                let f = source.state_with_acceleration(
                    body,
                    Body::SolarSystemBarycenter,
                    JulianDateTdb(jd),
                )?;
                let (lt, trace) = provider_lt(&mut manager, body, jd).map_err(io::Error::other)?;
                let reference = geocentric_light_time(&mut source, body, JulianDateTdb(jd))?;
                let emission = manager.select_pack(lt.emission_epoch)?.id.clone();
                let crossing = trace.iter().any(|(id, _)| id != &expected.id);
                if crossing {
                    crossings += 1;
                }
                let x = fields(experimental_long_term_apparent_ecliptic(
                    &mut slice,
                    body,
                    JulianDateTdb(jd),
                )?);
                let y = fields(experimental_long_term_apparent_ecliptic(
                    &mut source,
                    body,
                    JulianDateTdb(jd),
                )?);
                let provider_fields = fields(
                    experimental_long_term_apparent_with_provider(
                        &mut manager,
                        body,
                        JulianDateTdb(jd),
                    )
                    .map_err(|error| io::Error::other(format!("provider failed: {error:?}")))?,
                );
                for k in 0..6 {
                    let difference = if k == 0 {
                        ((provider_fields[k] - y[k] + 180.0).rem_euclid(360.0) - 180.0).abs()
                    } else {
                        (provider_fields[k] - y[k]).abs()
                    };
                    provider_max[k] = provider_max[k].max(difference);
                    if difference > [1e-8, 1e-8, 1e-12, 1e-8, 1e-8, 1e-12][k] {
                        errors += 1;
                    }
                }
                if offset == -1.0 / 86400.0 {
                    before[body_index] = x;
                }
                if offset == 1.0 / 86400.0 {
                    after[body_index] = x;
                }
                let delta = [
                    maxdiff(&p.position_km, &f.position_km),
                    maxdiff(&p.velocity_km_per_day, &f.velocity_km_per_day),
                    maxdiff(&p.acceleration_km_per_day2, &f.acceleration_km_per_day2),
                    maxdiff(&lt.position_km, &reference.position_km),
                    maxdiff(&lt.velocity_km_per_day, &reference.velocity_km_per_day),
                    (lt.emission_epoch.0 - reference.emission_epoch.0).abs(),
                    ((x[0] - y[0] + 180.0).rem_euclid(360.0) - 180.0).abs(),
                    (x[1] - y[1]).abs(),
                    (x[2] - y[2]).abs(),
                    (x[3] - y[3]).abs(),
                    (x[4] - y[4]).abs(),
                    (x[5] - y[5]).abs(),
                ];
                let limits = [
                    1e-6, 1e-6, 1e-6, 1e-6, 1e-6, 1e-9, 1e-8, 1e-8, 1e-12, 1e-8, 1e-8, 1e-12,
                ];
                for k in 0..12 {
                    max[k] = max[k].max(delta[k]);
                    if delta[k] > limits[k] {
                        errors += 1;
                    }
                }
                println!("case,{bound:.9},{jd:.9},{body:?},{},{emission},{crossing},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",expected.id,delta[0],delta[1],delta[2],delta[3],delta[4],delta[5],delta[6],delta[7],delta[8],delta[9],delta[10],delta[11]);
                count += 1;
            }
        }
        let dt = (bound + 1.0 / 86400.0) - (bound - 1.0 / 86400.0);
        for body_index in 0..bodies.len() {
            let a = before[body_index];
            let b = after[body_index];
            let difference = [
                (b[0] - a[0] + 180.0).rem_euclid(360.0) - 180.0,
                b[1] - a[1],
                b[2] - a[2],
            ];
            let limits = [1e-8, 1e-8, 1e-12];
            for k in 0..3 {
                let residual = (difference[k] - 0.5 * (a[k + 3] + b[k + 3]) * dt).abs();
                continuity_max[k] = continuity_max[k].max(residual);
                if residual > limits[k] {
                    errors += 1;
                }
            }
            continuity_count += 1;
        }
        // Missing prior package: reception succeeds, solver must fail when the
        // earlier Pluto emission is requested. Do not hide this with overlap.
        let mut missing = specs.clone();
        missing[i - 1].file_path = PathBuf::from(&a[3]);
        let mut m = De441PackManager::from_specs(missing)?;
        let jd = bound + 1.0 / 86400.0;
        assert!(m
            .state(Body::Earth, Body::SolarSystemBarycenter, JulianDateTdb(jd))
            .is_ok());
        let err = provider_lt(&mut m, Body::PlutoBarycenter, jd)
            .expect_err("missing emission pack must fail");
        assert!(
            err.contains("MissingPack") && err.contains(&specs[i - 1].id),
            "{err}"
        );
        assert!(
            matches!(experimental_long_term_apparent_with_provider(&mut m,Body::PlutoBarycenter,JulianDateTdb(jd)), Err(ExperimentalProviderApparentError::Provider(De441PackError::MissingPack{id,..})) if id==specs[i-1].id)
        );
        let mut reception_only = De441::open(&specs[i].file_path)?;
        assert!(
            experimental_long_term_apparent_ecliptic(
                &mut reception_only,
                Body::PlutoBarycenter,
                JulianDateTdb(jd)
            )
            .is_ok(),
            "expected record padding to expose policy mismatch"
        );
        let mut missing = specs.clone();
        missing[i].file_path = PathBuf::from(&a[3]);
        let mut m = De441PackManager::from_specs(missing)?;
        assert!(
            matches!(m.state(Body::Earth,Body::SolarSystemBarycenter,JulianDateTdb(jd)),Err(De441PackError::MissingPack{id,..}) if id==specs[i].id)
        );
        assert!(
            matches!(experimental_long_term_apparent_with_provider(&mut m,Body::PlutoBarycenter,JulianDateTdb(jd)), Err(ExperimentalProviderApparentError::Provider(De441PackError::MissingPack{id,..})) if id==specs[i].id)
        );
        // The later pack is needed by the positive deflection-rate stencil.
        assert!(
            matches!(experimental_long_term_apparent_with_provider(&mut m,Body::PlutoBarycenter,JulianDateTdb(bound-1.0/86400.0)), Err(ExperimentalProviderApparentError::Provider(De441PackError::MissingPack{id,..})) if id==specs[i].id)
        );
        println!("missing,{bound:.9},{},{},ok", specs[i - 1].id, specs[i].id);
    }
    assert!(matches!(
        manager.select_pack(JulianDateTdb(specs[0].start_tdb.0 - 1.0 / 86400.0)),
        Err(De441PackError::NoPackForEpoch(_))
    ));
    assert!(matches!(
        manager.select_pack(specs[9].end_tdb),
        Err(De441PackError::NoPackForEpoch(_))
    ));
    assert!(matches!(
        manager.select_pack(JulianDateTdb(f64::NAN)),
        Err(De441PackError::InvalidEpoch(_))
    ));
    let mut corrupt = specs.clone();
    corrupt[4].file_path = PathBuf::from(&a[4]);
    let mut bad = De441PackManager::from_specs(corrupt)?;
    assert!(matches!(
        bad.state(Body::Earth, Body::SolarSystemBarycenter, specs[4].start_tdb),
        Err(De441PackError::Reader { .. })
    ));
    assert!(matches!(
        experimental_long_term_apparent_with_provider(
            &mut bad,
            Body::MarsBarycenter,
            JulianDateTdb(specs[4].start_tdb.0 + 1.0)
        ),
        Err(ExperimentalProviderApparentError::Provider(
            De441PackError::Reader { .. }
        ))
    ));
    assert!(matches!(
        experimental_long_term_apparent_with_provider(
            &mut manager,
            Body::MarsBarycenter,
            specs[9].end_tdb
        ),
        Err(ExperimentalProviderApparentError::Provider(
            De441PackError::NoPackForEpoch(_)
        ))
    ));
    println!("summary,cases={count},crossings={crossings},over_limit_components={errors},missing_prior=9,missing_reception=9,missing_deflection_stencil=9,reception_only_ignores_missing_prior=9,invalid_checks=4,continuity_cases={continuity_count},continuity_max={continuity_max:?},provider_max={provider_max:?},max={max:?}");
    if errors != 0 {
        return Err("comparison exceeded declared limit".into());
    }
    Ok(())
}
