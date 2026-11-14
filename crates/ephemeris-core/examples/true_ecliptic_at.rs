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
use virgostars_ephemeris_core::de441::{Body, De441};
use virgostars_ephemeris_core::first_sky::CivilTimestamp;
use virgostars_ephemeris_core::house_angles;
use virgostars_ephemeris_core::lunar_points::{calculate_lunar_point, LunarPoint};
use virgostars_ephemeris_core::true_ecliptic::geocentric_true_ecliptic_from_civil;

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
    if args.len() != 3 {
        return Err(
            "usage: true_ecliptic_at <classic-de441.eph> <ISO timestamp> | lunar <classic-de441.eph> <ISO timestamp> | angles <JD-UT1> <JD-TT> <latitude> <east-longitude> | julday <year> <month> <day> <decimal-hour> <calendar> <scale> | revjul <JD> <calendar> <scale>".into(),
        );
    }
    let timestamp = args[2].to_str().ok_or("timestamp must be UTF-8")?;
    let civil = CivilTimestamp::parse(timestamp)?;
    let mut eph = De441::open(&args[1])?;
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
    let mut results = Vec::with_capacity(bodies.len());
    for body in bodies {
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
