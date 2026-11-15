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

//! Offline experimental DE441 true-ecliptic CSV, added 2026-11-14.
use std::error::Error;
use virgostars_ephemeris_core::calendar::{Calendar, TimeScale};
use virgostars_ephemeris_core::calendar_decimal_hours::{
    julday_f64, revjul_f64, DecimalHourDateTime,
};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::equatorial_horizontal::{
    noaa_standard_refraction, true_equatorial_to_horizontal,
};
use virgostars_ephemeris_core::first_sky::{CivilTimestamp, Observer};
use virgostars_ephemeris_core::fixed_star::{calculate_star, FixedStarCatalogEntry};
use virgostars_ephemeris_core::horizon_events::{
    find_geometric_event, find_standard_solar_event, find_topocentric_geometric_event,
    topocentric_equatorial_direction, HorizonEventKind, HorizonEventSearch,
    StandardSolarEventSearch,
};
use virgostars_ephemeris_core::house_angles;
use virgostars_ephemeris_core::lunar_points::{calculate_lunar_point, LunarPoint};
use virgostars_ephemeris_core::true_ecliptic::{
    geocentric_true_ecliptic, geocentric_true_ecliptic_from_civil,
    geocentric_true_equatorial_from_civil, heliocentric_true_ecliptic_from_civil,
};

const BODIES: [Body; 10] = [
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

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    let arg = |index: usize| -> Result<&str, Box<dyn Error>> {
        Ok(args
            .get(index)
            .ok_or("missing argument")?
            .to_str()
            .ok_or("argument must be UTF-8")?)
    };
    let calendar = |value: &str| -> Result<Calendar, Box<dyn Error>> {
        match value {
            "gregorian" => Ok(Calendar::ProlepticGregorian),
            "julian" => Ok(Calendar::ProlepticJulian),
            _ => Err("calendar must be gregorian or julian".into()),
        }
    };
    let scale = |value: &str| -> Result<TimeScale, Box<dyn Error>> {
        match value {
            "ut1" => Ok(TimeScale::Ut1),
            "tt" => Ok(TimeScale::Tt),
            "tai" => Ok(TimeScale::Tai),
            "tdb" => Ok(TimeScale::Tdb),
            _ => Err("scale must be ut1, tt, tai, or tdb".into()),
        }
    };
    if args.len() == 12 && args[1] == "star_epoch" {
        let tt = arg(3)?.parse()?;
        let tdb = JulianDateTdb(arg(4)?.parse()?);
        let entry = FixedStarCatalogEntry {
            epoch_tdb: JulianDateTdb(arg(5)?.parse()?),
            right_ascension_deg: arg(6)?.parse()?,
            declination_deg: arg(7)?.parse()?,
            proper_motion_ra_cos_dec_mas_per_year: arg(8)?.parse()?,
            proper_motion_dec_mas_per_year: arg(9)?.parse()?,
            parallax_mas: arg(10)?.parse()?,
            radial_velocity_km_per_second: arg(11)?.parse()?,
        };
        let mut eph = De441::open(&args[2])?;
        let star = calculate_star(&mut eph, entry, tt, tdb).map_err(|error| error.to_string())?;
        println!("tt_jd,tdb_jd,longitude_deg,latitude_deg,distance_au,longitude_deg_per_tdb_day,latitude_deg_per_tdb_day,distance_au_per_tdb_day");
        println!(
            "{:.17},{:.17},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}",
            star.tt_jd,
            star.tdb.0,
            star.longitude_deg,
            star.latitude_deg,
            star.distance_au,
            star.longitude_speed_deg_per_tdb_day,
            star.latitude_speed_deg_per_tdb_day,
            star.distance_speed_au_per_tdb_day
        );
        return Ok(());
    }
    if args.len() == 6 && args[1] == "calc_epoch" {
        let target = BODIES
            .into_iter()
            .find(|body| format!("{body:?}") == arg(3).unwrap_or(""))
            .ok_or("unsupported target")?;
        let tt = arg(4)?.parse()?;
        let tdb = JulianDateTdb(arg(5)?.parse()?);
        let mut eph = De441::open(&args[2])?;
        let state = geocentric_true_ecliptic(&mut eph, target, tdb, tt)?;
        println!("target,tt_jd,tdb_jd,longitude_deg,latitude_deg,distance_au,longitude_deg_per_tdb_day,latitude_deg_per_tdb_day,distance_au_per_tdb_day");
        println!(
            "{:?},{:.17},{:.17},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}",
            state.target,
            state.reception_tt_jd,
            state.reception_tdb.0,
            state.longitude_deg,
            state.latitude_deg,
            state.distance_au,
            state.longitude_speed_deg_per_tdb_day,
            state.latitude_speed_deg_per_tdb_day,
            state.distance_speed_au_per_tdb_day
        );
        return Ok(());
    }
    if args.len() == 11 && args[1] == "standard_solar_event" {
        let kind = match arg(3)? {
            "rise" => HorizonEventKind::Rise,
            "set" => HorizonEventKind::Set,
            _ => return Err("solar event kind must be rise or set".into()),
        };
        let input = StandardSolarEventSearch {
            kind,
            start_ut1_jd: arg(4)?.parse()?,
            start_tt_jd: arg(5)?.parse()?,
            start_tdb: JulianDateTdb(arg(6)?.parse()?),
            latitude_deg: arg(7)?.parse()?,
            longitude_deg_east: arg(8)?.parse()?,
            window_days: arg(9)?.parse()?,
            step_minutes: arg(10)?.parse()?,
        };
        let mut eph = De441::open(&args[2])?;
        let event =
            find_standard_solar_event(&mut eph, input).map_err(|error| error.to_string())?;
        println!("event,target,ut1_jd,tt_jd,tdb_jd,bracket_seconds");
        println!(
            "{},Sun,{:.17},{:.17},{:.17},{:.12}",
            arg(3)?,
            event.ut1_jd,
            event.tt_jd,
            event.tdb.0,
            event.bracket_seconds
        );
        return Ok(());
    }
    if args.len() == 3 && args[1] == "noaa_refraction" {
        let r = noaa_standard_refraction(arg(2)?.parse()?).map_err(|error| error.to_string())?;
        println!(
            "true_altitude_deg,correction_deg,apparent_altitude_deg\n{:.12},{:.12},{:.12}",
            r.true_altitude_deg, r.correction_deg, r.apparent_altitude_deg
        );
        return Ok(());
    }
    if args.len() == 10 && args[1] == "topocentric" {
        let site = Observer {
            latitude_deg: arg(4)?.parse()?,
            longitude_deg_east: arg(5)?.parse()?,
            height_m: arg(6)?.parse()?,
        };
        let (ra, dec) = topocentric_equatorial_direction(
            arg(2)?.parse()?,
            arg(3)?.parse()?,
            site,
            arg(7)?.parse()?,
            arg(8)?.parse()?,
            arg(9)?.parse()?,
        )
        .map_err(|error| error.to_string())?;
        println!("ra_deg,declination_deg\n{ra:.12},{dec:.12}");
        return Ok(());
    }
    if (args.len() == 13 && args[1] == "event")
        || (args.len() == 14 && args[1] == "topocentric_event")
    {
        let target = BODIES
            .into_iter()
            .find(|body| format!("{body:?}") == arg(3).unwrap_or(""))
            .ok_or("unsupported event target")?;
        let kind = match arg(4)? {
            "rise" => HorizonEventKind::Rise,
            "set" => HorizonEventKind::Set,
            "upper_transit" => HorizonEventKind::UpperTransit,
            "lower_transit" => HorizonEventKind::LowerTransit,
            _ => return Err("unsupported event kind".into()),
        };
        let input = HorizonEventSearch {
            target,
            kind,
            start_ut1_jd: arg(5)?.parse()?,
            start_tt_jd: arg(6)?.parse()?,
            start_tdb: JulianDateTdb(arg(7)?.parse()?),
            latitude_deg: arg(8)?.parse()?,
            longitude_deg_east: arg(9)?.parse()?,
            threshold_altitude_deg: arg(10)?.parse()?,
            window_days: arg(11)?.parse()?,
            step_minutes: arg(12)?.parse()?,
        };
        let mut eph = De441::open(&args[2])?;
        let event = if args[1] == "event" {
            find_geometric_event(&mut eph, input)
        } else {
            find_topocentric_geometric_event(&mut eph, input, arg(13)?.parse()?)
        }
        .map_err(|error| error.to_string())?;
        println!("event,target,ut1_jd,tt_jd,tdb_jd,bracket_seconds");
        println!(
            "{},{:?},{:.17},{:.17},{:.17},{:.12}",
            arg(4)?,
            event.target,
            event.ut1_jd,
            event.tt_jd,
            event.tdb.0,
            event.bracket_seconds
        );
        return Ok(());
    }
    if args.len() == 8 && args[1] == "horizontal" {
        let output = true_equatorial_to_horizontal(
            arg(2)?.parse()?,
            arg(3)?.parse()?,
            arg(4)?.parse()?,
            arg(5)?.parse()?,
            arg(6)?.parse()?,
            arg(7)?.parse()?,
        )?;
        println!("true_altitude_deg,azimuth_north_east_deg,azimuth_south_west_deg");
        match (output.azimuth_north_east_deg, output.azimuth_south_west_deg) {
            (Some(north), Some(south)) => println!(
                "{:.12},{:.12},{:.12}",
                output.true_altitude_deg, north, south
            ),
            (None, None) => println!("{:.12},undefined,undefined", output.true_altitude_deg),
            _ => return Err("inconsistent horizontal result".into()),
        }
        return Ok(());
    }
    if args.len() == 8 && args[1] == "julday" {
        let input = DecimalHourDateTime {
            year: arg(2)?.parse()?,
            month: arg(3)?.parse()?,
            day: arg(4)?.parse()?,
            decimal_hour: arg(5)?.parse()?,
            scale: scale(arg(7)?)?,
        };
        let jd = julday_f64(input, calendar(arg(6)?)?)?;
        println!("julian_day\n{jd:.17}");
        return Ok(());
    }
    if args.len() == 5 && args[1] == "revjul" {
        let date = revjul_f64(arg(2)?.parse()?, calendar(arg(3)?)?, scale(arg(4)?)?)?;
        println!(
            "year,month,day,decimal_hour\n{},{},{},{:.17}",
            date.year, date.month, date.day, date.decimal_hour
        );
        return Ok(());
    }
    if args.len() == 6 && args[1] == "angles" {
        let number = |index: usize| -> Result<f64, Box<dyn Error>> {
            Ok(args[index]
                .to_str()
                .ok_or("angle input must be UTF-8")?
                .parse()?)
        };
        let angles = house_angles::from_epochs(number(2)?, number(3)?, number(4)?, number(5)?)?;
        println!("armc_deg,ascendant_deg,midheaven_deg,true_obliquity_deg");
        println!(
            "{:.12},{:.12},{:.12},{:.12}",
            angles.armc_deg, angles.ascendant_deg, angles.midheaven_deg, angles.true_obliquity_deg
        );
        return Ok(());
    }
    if args.len() == 4 && args[1] == "lunar" {
        let civil = CivilTimestamp::parse(arg(3)?)?;
        let (tt, tdb) = civil.to_tt_and_approximate_tdb()?;
        let mut eph = De441::open(&args[2])?;
        let points = [
            LunarPoint::MeanNode,
            LunarPoint::TrueNode,
            LunarPoint::MeanApogee,
            LunarPoint::OsculatingApogee,
        ];
        let results = points
            .into_iter()
            .map(|point| calculate_lunar_point(&mut eph, point, tt, tdb, true))
            .collect::<Result<Vec<_>, _>>()?;
        println!("point,tt_jd,tdb_jd,longitude_deg,latitude_deg,distance_au,longitude_deg_per_tt_day,latitude_deg_per_tt_day,distance_au_per_tt_day");
        for r in results {
            println!(
                "{:?},{:.10},{:.10},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}",
                r.point,
                tt,
                tdb.0,
                r.longitude_deg,
                r.latitude_deg,
                r.distance_au,
                r.longitude_rate_deg_per_day,
                r.latitude_rate_deg_per_day,
                r.distance_rate_au_per_day
            );
        }
        return Ok(());
    }
    if args.len() == 4 && args[1] == "equatorial" {
        let civil = CivilTimestamp::parse(arg(3)?)?;
        let mut eph = De441::open(&args[2])?;
        let results = BODIES
            .into_iter()
            .map(|body| geocentric_true_equatorial_from_civil(&mut eph, body, civil))
            .collect::<Result<Vec<_>, _>>()?;
        println!("target,tt_jd,tdb_jd,ra_deg,declination_deg,distance_au,ra_deg_per_tdb_day,declination_deg_per_tdb_day,distance_au_per_tdb_day");
        for r in results {
            println!(
                "{:?},{:.10},{:.10},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}",
                r.target,
                r.reception_tt_jd,
                r.reception_tdb.0,
                r.right_ascension_deg,
                r.declination_deg,
                r.distance_au,
                r.right_ascension_speed_deg_per_tdb_day,
                r.declination_speed_deg_per_tdb_day,
                r.distance_speed_au_per_tdb_day
            );
        }
        return Ok(());
    }
    if args.len() == 4 && args[1] == "heliocentric" {
        let civil = CivilTimestamp::parse(arg(3)?)?;
        let (tt, tdb) = civil.to_tt_and_approximate_tdb()?;
        let mut eph = De441::open(&args[2])?;
        let results = BODIES
            .into_iter()
            .skip(1)
            .map(|body| heliocentric_true_ecliptic_from_civil(&mut eph, body, civil))
            .collect::<Result<Vec<_>, _>>()?;
        println!("target,tt_jd,tdb_jd,longitude_deg,latitude_deg,distance_au,longitude_deg_per_tdb_day,latitude_deg_per_tdb_day,distance_au_per_tdb_day");
        // The Sun relative to itself has no direction; zeros are an explicit
        // legacy-shaped sentinel, not a physical spherical direction.
        println!("Sun,{:.10},{:.10},0,0,0,0,0,0", tt, tdb.0);
        for r in results {
            println!(
                "{:?},{:.10},{:.10},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}",
                r.target,
                r.reception_tt_jd,
                r.reception_tdb.0,
                r.longitude_deg,
                r.latitude_deg,
                r.distance_au,
                r.longitude_speed_deg_per_tdb_day,
                r.latitude_speed_deg_per_tdb_day,
                r.distance_speed_au_per_tdb_day
            );
        }
        return Ok(());
    }
    if args.len() != 3 {
        return Err(
            "usage: true_ecliptic_at <classic-de441.eph> <ISO timestamp> | lunar|equatorial|heliocentric <classic-de441.eph> <ISO timestamp> | angles <JD-UT1> <JD-TT> <latitude> <east-longitude> | horizontal <JD-UT1> <JD-TT> <latitude> <east-longitude> <RA> <declination> | julday <year> <month> <day> <decimal-hour> <calendar> <scale> | revjul <JD> <calendar> <scale>".into(),
        );
    }
    let timestamp = args[2].to_str().ok_or("timestamp must be UTF-8")?;
    let civil = CivilTimestamp::parse(timestamp)?;
    let mut eph = De441::open(&args[1])?;
    let mut results = Vec::with_capacity(BODIES.len());
    for body in BODIES {
        results.push(geocentric_true_ecliptic_from_civil(&mut eph, body, civil)?);
    }
    println!("target,tt_jd,tdb_jd,longitude_deg,latitude_deg,distance_au,longitude_deg_per_tdb_day,latitude_deg_per_tdb_day,distance_au_per_tdb_day");
    for r in results {
        println!(
            "{:?},{:.10},{:.10},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}",
            r.target,
            r.reception_tt_jd,
            r.reception_tdb.0,
            r.longitude_deg,
            r.latitude_deg,
            r.distance_au,
            r.longitude_speed_deg_per_tdb_day,
            r.latitude_speed_deg_per_tdb_day,
            r.distance_speed_au_per_tdb_day
        );
    }
    Ok(())
}
