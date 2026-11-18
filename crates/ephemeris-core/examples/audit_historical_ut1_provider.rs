// SPDX-License-Identifier: CPAL-1.0
//! Owner-local example: full reader versus actual manager, UT1 scenarios.
use std::{
    error::Error,
    io::{self, BufRead},
    path::PathBuf,
};
use virgostars_ephemeris_core::{
    apparent_ecliptic_of_date::ExperimentalProviderApparentError,
    calendar::{Calendar, JulianDate, TimeScale},
    de441::{Body, De441},
    de441_pack_manager::{De441PackError, De441PackManager},
    delta_t_long_range::espenak_meeus_2006_long_range,
    historical_delta_t_scenarios::{
        historical_delta_t_scenarios, historical_delta_t_scenarios_with_provider,
        HistoricalProviderScenarioError, ScenarioFrame, ScenarioSample,
    },
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 5 {
        return Err("usage: EXE KERNEL PACK_DIR ABSENT CORRUPT".into());
    }
    let mut manager = De441PackManager::from_mobile_directory(&args[2])?;
    let specs: Vec<_> = manager.inventory().into_iter().map(|x| x.spec).collect();
    let mut full = De441::open(&args[1])?;
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
    let mut count = 0;
    let mut max = [0.0_f64; 6];
    for spec in &specs {
        let ut1 = (spec.start_tdb.0 + spec.end_tdb.0) * 0.5;
        let date =
            JulianDate::from_f64(ut1, TimeScale::Ut1)?.to_calendar(Calendar::ProlepticGregorian)?;
        let year = f64::from(date.year) + (f64::from(date.month) - 0.5) / 12.0;
        let delta = espenak_meeus_2006_long_range(year)?;
        for (id, body) in bodies.into_iter().enumerate() {
            let a = historical_delta_t_scenarios_with_provider(
                &mut manager,
                body,
                ut1,
                delta,
                60.0,
                "Espenak-Meeus-2006-example",
            )
            .map_err(|e| io::Error::other(format!("{e:?}")))?;
            let b = historical_delta_t_scenarios(
                &mut full,
                body,
                ut1,
                delta,
                60.0,
                ScenarioFrame::PoleBased,
            )?;
            for (label, x, y) in [
                ("lower", a.lower, b.lower),
                ("nominal", a.nominal, b.nominal),
                ("upper", a.upper, b.upper),
            ] {
                let values =
                    |v: virgostars_ephemeris_core::six_field_units::FixedSixFieldsDegreesAu| {
                        [
                            v.longitude_deg,
                            v.latitude_deg,
                            v.distance_au,
                            v.longitude_speed_deg_per_tdb_day,
                            v.latitude_speed_deg_per_tdb_day,
                            v.distance_speed_au_per_tdb_day,
                        ]
                    };
                let xvals = values(x.fields);
                let yvals = values(y.fields);
                let limits = [1e-8, 1e-8, 1e-12, 1e-8, 1e-8, 1e-12];
                for i in 0..6 {
                    let d = if i == 0 {
                        ((xvals[i] - yvals[i] + 180.0).rem_euclid(360.0) - 180.0).abs()
                    } else {
                        (xvals[i] - yvals[i]).abs()
                    };
                    max[i] = max[i].max(d);
                    if d > limits[i] {
                        return Err("source/provider mismatch".into());
                    }
                }
                println!("case,{},{id},{label},{ut1:.17},{:.17},{:.17},{delta:.12},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17}",spec.id,x.epoch.jd_tt,x.epoch.jd_tdb.0,xvals[0],xvals[1],xvals[2],xvals[3],xvals[4],xvals[5]);
                count += 1;
            }
        }
    }
    for i in 1..specs.len() {
        let mut altered = specs.clone();
        altered[i].file_path = PathBuf::from(&args[3]);
        let mut missing = De441PackManager::from_specs(altered)?;
        let jd_ut1 = specs[i].start_tdb.0 - 124.0 / 86400.0;
        assert!(
            matches!(historical_delta_t_scenarios_with_provider(&mut missing,Body::Sun,jd_ut1,64.0,120.0,"illustrative"),Err(HistoricalProviderScenarioError::Calculation{sample:ScenarioSample::Upper,source:ExperimentalProviderApparentError::Provider(De441PackError::MissingPack{id,..})}) if id==specs[i].id)
        );
    }
    let mut bad = specs.clone();
    bad[8].file_path = PathBuf::from(&args[4]);
    let mut corrupt = De441PackManager::from_specs(bad)?;
    assert!(matches!(
        historical_delta_t_scenarios_with_provider(
            &mut corrupt,
            Body::Sun,
            2451545.0,
            64.0,
            60.0,
            "illustrative"
        ),
        Err(HistoricalProviderScenarioError::Calculation {
            sample: ScenarioSample::Lower,
            source: ExperimentalProviderApparentError::Provider(De441PackError::Reader { .. })
        })
    ));
    assert!(matches!(
        historical_delta_t_scenarios_with_provider(
            &mut manager,
            Body::Sun,
            specs[9].end_tdb.0 + 1.0,
            64.0,
            60.0,
            "illustrative"
        ),
        Err(HistoricalProviderScenarioError::Calculation {
            sample: ScenarioSample::Lower,
            source: ExperimentalProviderApparentError::Provider(De441PackError::NoPackForEpoch(_))
        })
    ));
    println!("summary,cases={count},boundary_missing_upper=9,corrupt=1,outside=1,max={max:?}");
    // Optional stdin conversion stream for independent ERFA reference checks.
    for line in io::stdin().lock().lines() {
        let text = line?;
        if text.trim().is_empty() {
            continue;
        }
        let (a, b) = text.split_once(',').ok_or("expected UT1,delta")?;
        let e = virgostars_ephemeris_core::explicit_delta_t::ut1_to_tdb_with_explicit_delta_t(
            a.parse()?,
            b.parse()?,
            0.0,
        )?;
        let correction =
            virgostars_ephemeris_core::explicit_delta_t::approximate_tdb_minus_tt_seconds(e.jd_tt)?;
        println!(
            "time,{:.17},{:.17},{:.17},{correction:.17}",
            e.jd_ut1, e.jd_tt, e.jd_tdb.0
        );
    }
    Ok(())
}
