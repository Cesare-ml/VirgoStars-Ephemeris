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

//! Thin Node.js (N-API) binding over `virgostars-ephemeris-core`.
//!
//! No astronomy lives here: every calculation is a call into the Rust core.
//! The binding only validates arguments, maps Swiss-style integer codes to
//! core types, and turns typed core errors into messages that start with a
//! stable code (`E_UNSUPPORTED_FLAG`, `E_UNSUPPORTED_BODY`, `E_RANGE`, …) so
//! that the JavaScript facade can map them without parsing prose.
//!
//! An [`Engine`] owns one open DE441 file and its own state: two engines never
//! share mutable data, which gives per-instance isolation.
//!
//! Experimental. Not a drop-in `swisseph` replacement; see
//! `docs/MVP1_NODE_BINDING_2026-11-16.md`.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    applied_corrections, geocentric_apparent_six_fields_many_with_options,
    small_body_apparent_six_fields, tdb_to_tt_jd, ApparentCenter, ApparentFrame, ApparentOptions,
};
use virgostars_ephemeris_core::calendar::{Calendar, TimeScale};
use virgostars_ephemeris_core::calendar_decimal_hours::{
    julday_f64, revjul_f64, DecimalHourDateTime,
};
use virgostars_ephemeris_core::civil_apparent::{
    jd_ut_to_tdb_with_policy, jd_ut_to_tdb_with_table, CivilTimeSource,
};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::delta_t_table::DeltaTTable;
use virgostars_ephemeris_core::earth_orientation::ecliptic_nutation;
use virgostars_ephemeris_core::eclipse::{next_lunar_eclipse, EclipseError, LunarKind, SunMoon};
use virgostars_ephemeris_core::fixed_star::{
    calculate_star_with_solar_deflection, StellarSolarDeflection,
};
use virgostars_ephemeris_core::hipparcos::{
    read_hip2_selected, Hip2QualityPolicy, Hip2Record, RadialVelocity,
};
use virgostars_ephemeris_core::horizon::{
    ecliptic_to_equatorial, equatorial_to_horizontal, estimated_pressure_hpa,
    extended_apparent_to_true, extended_true_to_apparent, horizontal_to_equatorial,
    local_apparent_sidereal_deg, plain_apparent_to_true, plain_true_to_apparent,
    STANDARD_LAPSE_RATE,
};
use virgostars_ephemeris_core::house_angles::house_angles;
use virgostars_ephemeris_core::lunar_points::{lunar_point, LunarPoint};
use virgostars_ephemeris_core::rise_transit::{
    find_event, Atmosphere, Disc, Event, Limb, Request, RiseError, Site, SkyPosition,
};
use virgostars_ephemeris_core::solar_eclipse::{next_solar_eclipse, SolarKind};
use virgostars_ephemeris_core::spk_type21::SmallBodySpk;

/// Swiss-style flag bits this binding understands (values are the public
/// constants of the C API: `SEFLG_JPLEPH = 1`, `SEFLG_SPEED = 256`,
/// `SEFLG_NOGDEFL = 512`). Anything else is rejected, never ignored.
const FLAG_JPLEPH: i32 = 1;
const FLAG_HELCTR: i32 = 8;
const FLAG_SPEED: i32 = 256;
const FLAG_NOGDEFL: i32 = 512;
const FLAG_NOABERR: i32 = 1024;
const FLAG_EQUATORIAL: i32 = 2048;
const SUPPORTED_FLAGS: i32 =
    FLAG_JPLEPH | FLAG_HELCTR | FLAG_SPEED | FLAG_NOGDEFL | FLAG_EQUATORIAL;
/// Swiss body number of the Earth; computable only as a heliocentric target.
const BODY_EARTH: i32 = 14;
/// Swiss body number of Chiron; computable only after `load_chiron`.
const BODY_CHIRON: i32 = 15;
/// NAIF id JPL uses for 2060 Chiron in its small-body SPK files.
const CHIRON_NAIF_ID: i32 = 20_002_060;

fn fail(code: &str, detail: impl std::fmt::Display) -> Error {
    Error::new(Status::GenericFailure, format!("{code}: {detail}"))
}

#[napi(object)]
pub struct CalcResult {
    /// `[longitude°, latitude°, distance AU, dlon°/day, dlat°/day, ddist AU/day]`.
    /// The three speeds are zero unless `SEFLG_SPEED` (256) was requested.
    pub xx: Vec<f64>,
    /// Flags actually used; always carries `SEFLG_JPLEPH` (1), the bit that
    /// states "a JPL DE file", never a Swiss data claim.
    pub rflag: i32,
    /// How the epoch was obtained: `TDB_INPUT`, `IERS_LEAP_SECONDS` or
    /// `DELTA_T_POLYNOMIAL:<model>:<seconds>`.
    pub time_source: String,
    /// Corrections applied, plus any assumption the caller must know about.
    pub notes: Vec<String>,
}

/// Diagnostic flags only; passing a numerical filter is not a review decision.
#[napi(object)]
pub struct HipparcosQualityResult {
    pub hip: u32,
    pub solution_type: u32,
    pub components: u32,
    pub strict_eligible: bool,
    pub traditional_eligible: bool,
}

/// Inspect selected external HIP2 records without modifying any Engine.
#[napi]
pub fn inspect_hipparcos_quality(
    hip2_path: String,
    hips: Vec<u32>,
) -> Result<Vec<HipparcosQualityResult>> {
    let unique: HashSet<u32> = hips.iter().copied().collect();
    if hips.is_empty() || hips.len() > 10_000 || unique.len() != hips.len() || unique.contains(&0) {
        return Err(fail(
            "E_RANGE",
            "HIP list must contain 1 to 10000 unique positive IDs",
        ));
    }
    let file = File::open(&hip2_path).map_err(|e| fail("E_CATALOG_FILE", e))?;
    let records =
        read_hip2_selected(BufReader::new(file), &hips).map_err(|e| fail("E_CATALOG_FILE", e))?;
    Ok(records
        .into_iter()
        .map(|record| HipparcosQualityResult {
            hip: record.hip,
            solution_type: u32::from(record.solution_type),
            components: u32::from(record.components),
            strict_eligible: Hip2QualityPolicy::STRICT.accepts(&record),
            traditional_eligible: Hip2QualityPolicy::TRADITIONAL_STARS.accepts(&record),
        })
        .collect())
}

#[napi(object)]
pub struct RevJul {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: f64,
}

fn body_from_ipl(ipl: i32) -> Result<Body> {
    // Swiss body numbers; outer planets are DE441 system barycenters, as in
    // Swiss's own JPL mode, and are labeled so in `notes`.
    Ok(match ipl {
        0 => Body::Sun,
        1 => Body::Moon,
        2 => Body::Mercury,
        3 => Body::Venus,
        4 => Body::MarsBarycenter,
        5 => Body::JupiterBarycenter,
        6 => Body::SaturnBarycenter,
        7 => Body::UranusBarycenter,
        8 => Body::NeptuneBarycenter,
        9 => Body::PlutoBarycenter,
        BODY_EARTH => Body::Earth,
        other => {
            return Err(fail(
                "E_UNSUPPORTED_BODY",
                format!("body {other} is not available (supported: 0..=9, Sun to Pluto)"),
            ))
        }
    })
}

fn check_flags(flags: i32) -> Result<()> {
    if flags & !SUPPORTED_FLAGS != 0 {
        return Err(fail(
            "E_UNSUPPORTED_FLAG",
            format!(
                "flags {:#x} contain unsupported bits; supported: SEFLG_JPLEPH(1), SEFLG_HELCTR(8), SEFLG_SPEED(256), SEFLG_NOGDEFL(512), SEFLG_EQUATORIAL(2048)",
                flags & !SUPPORTED_FLAGS
            ),
        ));
    }
    Ok(())
}

fn options(flags: i32) -> ApparentOptions {
    ApparentOptions {
        solar_light_deflection: flags & FLAG_NOGDEFL == 0,
        frame: if flags & FLAG_EQUATORIAL != 0 {
            ApparentFrame::TrueEquatorOfDate
        } else {
            ApparentFrame::TrueEclipticOfDate
        },
        center: if flags & FLAG_HELCTR != 0 {
            ApparentCenter::Heliocentric
        } else {
            ApparentCenter::Geocentric
        },
    }
}

/// Flags reported back: those used, plus the two the heliocentric mode implies
/// (no deflection, no aberration), exactly as the reference binding reports them.
fn reported_flags(flags: i32) -> i32 {
    let mut used =
        (flags & (FLAG_SPEED | FLAG_NOGDEFL | FLAG_HELCTR | FLAG_EQUATORIAL)) | FLAG_JPLEPH;
    if flags & FLAG_HELCTR != 0 {
        used |= FLAG_NOGDEFL | FLAG_NOABERR;
    }
    used
}

fn source_label(source: &CivilTimeSource) -> String {
    match source {
        CivilTimeSource::IersLeapSeconds => "IERS_LEAP_SECONDS".to_string(),
        CivilTimeSource::DeltaTPolynomial {
            model_id,
            delta_t_seconds,
        } => format!("DELTA_T_POLYNOMIAL:{model_id}:{delta_t_seconds}"),
        CivilTimeSource::DeltaTTable {
            delta_t_seconds,
            uncertainty_seconds,
            predicted,
        } => format!(
            "DELTA_T_TABLE:{}:{delta_t_seconds}:{}",
            if *predicted { "PREDICTED" } else { "MEASURED" },
            uncertainty_seconds.map_or_else(|| "unknown".to_string(), |u| u.to_string())
        ),
    }
}

fn zero_speeds_unless_requested(mut six: [f64; 6], flags: i32) -> Vec<f64> {
    if flags & FLAG_SPEED == 0 {
        six[3] = 0.0;
        six[4] = 0.0;
        six[5] = 0.0;
    }
    six.to_vec()
}

fn calendar(gregorian: bool) -> Calendar {
    if gregorian {
        Calendar::ProlepticGregorian
    } else {
        Calendar::ProlepticJulian
    }
}

/// `swe_julday`-shaped: Julian day of a calendar date and decimal hour. The
/// calendar is explicit (`true` = proleptic Gregorian), there is no 1582
/// changeover, no time-scale conversion, and invalid input is an error.
#[napi]
pub fn julday(year: i32, month: u32, day: u32, hour: f64, gregorian: bool) -> Result<f64> {
    let month = u8::try_from(month).map_err(|_| fail("E_RANGE", "month"))?;
    let day = u8::try_from(day).map_err(|_| fail("E_RANGE", "day"))?;
    julday_f64(
        DecimalHourDateTime {
            year,
            month,
            day,
            decimal_hour: hour,
            scale: TimeScale::Ut1,
        },
        calendar(gregorian),
    )
    .map_err(|e| fail("E_RANGE", e))
}

/// `swe_revjul`-shaped inverse of [`julday`].
#[napi]
pub fn revjul(jd: f64, gregorian: bool) -> Result<RevJul> {
    let r = revjul_f64(jd, calendar(gregorian), TimeScale::Ut1).map_err(|e| fail("E_RANGE", e))?;
    Ok(RevJul {
        year: r.year,
        month: u32::from(r.month),
        day: u32::from(r.day),
        hour: r.decimal_hour,
    })
}

#[napi(object)]
pub struct HouseAnglesResult {
    pub ut1_julian_day: f64,
    pub tt_julian_day: f64,
    pub latitude_deg: f64,
    pub longitude_deg_east: f64,
    pub armc_deg: f64,
    pub true_obliquity_deg: f64,
    pub ascendant_deg: f64,
    pub midheaven_deg: f64,
    /// Vertex (western intersection of the ecliptic and the prime vertical).
    pub vertex_deg: f64,
    pub equatorial_ascendant_deg: f64,
    pub koch_co_ascendant_deg: f64,
    pub munkasey_co_ascendant_deg: f64,
    pub munkasey_polar_ascendant_deg: f64,
    /// `EXPLICIT_UT1_TT` when both scales were supplied, otherwise the UT policy label.
    pub time_source: String,
}

fn angles_result(
    ut1: f64,
    tt: f64,
    lat: f64,
    lon: f64,
    time_source: String,
) -> Result<HouseAnglesResult> {
    let a = house_angles(ut1, tt, lat, lon).map_err(|e| fail("E_RANGE", e))?;
    Ok(HouseAnglesResult {
        ut1_julian_day: a.ut1_jd,
        tt_julian_day: a.tt_jd,
        latitude_deg: a.latitude_deg,
        longitude_deg_east: a.longitude_deg_east,
        armc_deg: a.armc_deg,
        true_obliquity_deg: a.true_obliquity_deg,
        ascendant_deg: a.ascendant_deg,
        midheaven_deg: a.midheaven_deg,
        vertex_deg: a.extra.vertex_deg,
        equatorial_ascendant_deg: a.extra.equatorial_ascendant_deg,
        koch_co_ascendant_deg: a.extra.koch_co_ascendant_deg,
        munkasey_co_ascendant_deg: a.extra.munkasey_co_ascendant_deg,
        munkasey_polar_ascendant_deg: a.extra.munkasey_polar_ascendant_deg,
        time_source,
    })
}

/// House astronomy with **both** UT1 and TT supplied by the caller (no
/// conversion is done here).
#[napi]
pub fn house_angles_explicit(
    ut1_jd: f64,
    tt_jd: f64,
    latitude_deg: f64,
    longitude_deg_east: f64,
) -> Result<HouseAnglesResult> {
    angles_result(
        ut1_jd,
        tt_jd,
        latitude_deg,
        longitude_deg_east,
        "EXPLICIT_UT1_TT".to_string(),
    )
}

/// House astronomy from a UT Julian date; TT comes from the declared policy
/// (1900-01-01 .. 2027-06-28 UT).
#[napi]
pub fn house_angles_ut(
    jd_ut: f64,
    latitude_deg: f64,
    longitude_deg_east: f64,
) -> Result<HouseAnglesResult> {
    let (tdb, source) = jd_ut_to_tdb_with_policy(jd_ut).map_err(|e| fail("E_RANGE", e))?;
    let tt = tdb_to_tt_jd(tdb.0).map_err(|e| fail("E_RANGE", e))?;
    angles_result(
        jd_ut,
        tt,
        latitude_deg,
        longitude_deg_east,
        source_label(&source),
    )
}

#[napi(object)]
pub struct EclNut {
    pub true_obliquity_deg: f64,
    pub mean_obliquity_deg: f64,
    pub nutation_longitude_deg: f64,
    pub nutation_obliquity_deg: f64,
}

/// Obliquity and nutation at a **TT** Julian date (IAU 2006 mean obliquity,
/// IAU 2000B nutation); validated 1900-2100 TT.
#[napi]
pub fn ecliptic_nutation_tt(tt_jd: f64) -> Result<EclNut> {
    let n = ecliptic_nutation(tt_jd).map_err(|e| fail("E_RANGE", e))?;
    Ok(EclNut {
        true_obliquity_deg: n.true_obliquity_deg(),
        mean_obliquity_deg: n.mean_obliquity_deg(),
        nutation_longitude_deg: n.nutation_longitude_deg(),
        nutation_obliquity_deg: n.nutation_obliquity_deg(),
    })
}

#[napi(object)]
pub struct RefractionResult {
    pub true_altitude_deg: f64,
    pub apparent_altitude_deg: f64,
    /// Horizon dip of the observer's height (degrees, 0 or negative).
    pub dip_deg: f64,
}

/// `swe_refrac`-shaped (plain model): altitude after refraction, in degrees.
/// `to_apparent` selects true -> apparent; otherwise apparent -> true (the exact
/// numerical inverse, which differs slightly from the reference's own inverse).
#[napi]
pub fn refrac(altitude_deg: f64, pressure_hpa: f64, temperature_c: f64, to_apparent: bool) -> f64 {
    if to_apparent {
        plain_true_to_apparent(altitude_deg, pressure_hpa, temperature_c)
    } else {
        plain_apparent_to_true(altitude_deg, pressure_hpa, temperature_c)
    }
}

/// `swe_refrac_extended`-shaped (the model used by azalt and rise/set): both
/// altitudes and the horizon dip for an observer `height_m` above the surface.
#[napi]
pub fn refrac_extended(
    altitude_deg: f64,
    height_m: f64,
    pressure_hpa: f64,
    temperature_c: f64,
    lapse_rate: f64,
    to_apparent: bool,
) -> RefractionResult {
    let r = if to_apparent {
        extended_true_to_apparent(
            altitude_deg,
            height_m,
            pressure_hpa,
            temperature_c,
            lapse_rate,
        )
    } else {
        extended_apparent_to_true(
            altitude_deg,
            height_m,
            pressure_hpa,
            temperature_c,
            lapse_rate,
        )
    };
    RefractionResult {
        true_altitude_deg: r.true_altitude_deg,
        apparent_altitude_deg: r.apparent_altitude_deg,
        dip_deg: r.dip_deg,
    }
}

#[napi(object)]
pub struct LunarEclipseResult {
    /// Swiss `SE_ECL_*` bit: 4 total, 16 partial, 64 penumbral.
    pub kind: i32,
    pub maximum: f64,
    pub penumbral_begin: f64,
    pub penumbral_end: f64,
    pub partial_begin: Option<f64>,
    pub partial_end: Option<f64>,
    pub total_begin: Option<f64>,
    pub total_end: Option<f64>,
    pub umbral_magnitude: f64,
    pub penumbral_magnitude: f64,
}

#[napi(object)]
pub struct SolarEclipseResult {
    /// Swiss `SE_ECL_*` bits: 4 total, 8 annular, 16 partial, 32 hybrid, plus 1 central or 2 non-central.
    pub kind: i32,
    pub maximum: f64,
    pub partial_begin: f64,
    pub partial_end: f64,
    pub total_begin: Option<f64>,
    pub total_end: Option<f64>,
    pub center_begin: Option<f64>,
    pub center_end: Option<f64>,
    /// Closest approach of the shadow axis to the geocentre, equatorial Earth radii.
    pub gamma: f64,
    /// Local apparent noon at the ground point under the axis, near the maximum (0 if none).
    pub noon: f64,
}

#[napi(object)]
pub struct AzaltResult {
    /// Azimuth measured from the **south** toward the west (the reference convention).
    pub azimuth_south_deg: f64,
    pub true_altitude_deg: f64,
    pub apparent_altitude_deg: f64,
    pub time_source: String,
}

#[napi(object)]
pub struct AzaltRevResult {
    /// Ecliptic longitude or right ascension, degrees.
    pub x0_deg: f64,
    /// Ecliptic latitude or declination, degrees.
    pub x1_deg: f64,
    pub time_source: String,
}

/// Body radius in km used for the semi-diameter in rise/set (Sun 696000, Moon
/// 1737.5, then the planets); `None` for points.
fn body_radius_km(ipl: i32) -> Option<f64> {
    Some(match ipl {
        0 => 696_000.0,
        1 => 1_737.5,
        2 => 2_440.0,
        3 => 6_052.0,
        4 => 3_389.5,
        5 => 69_911.0,
        6 => 58_232.0,
        7 => 25_362.0,
        8 => 24_622.0,
        9 => 1_188.3,
        _ => return None,
    })
}

const RISE: i32 = 1;
const SET: i32 = 2;
const MTRANSIT: i32 = 4;
const ITRANSIT: i32 = 8;
const BIT_GEOCTR_NO_ECL_LAT: i32 = 128;
const BIT_DISC_CENTER: i32 = 256;
const BIT_NO_REFRACTION: i32 = 512;
const BIT_CIVIL_TWILIGHT: i32 = 1024;
const BIT_NAUTIC_TWILIGHT: i32 = 2048;
const BIT_ASTRO_TWILIGHT: i32 = 4096;
const BIT_DISC_BOTTOM: i32 = 8192;
const BIT_FIXED_DISC_SIZE: i32 = 16384;
const BIT_FORCE_SLOW_METHOD: i32 = 32768;
const AU_KM: f64 = 149_597_870.7;
const MEAN_MOON_DISTANCE_KM: f64 = 384_400.0;

#[napi]
pub fn core_version() -> String {
    format!(
        "virgostars-ephemeris-node {} (core {})",
        env!("CARGO_PKG_VERSION"),
        "virgostars-ephemeris-core 0.1.0"
    )
}

/// One open classic-DE441 file plus an optional set of Hipparcos records.
#[napi]
pub struct Engine {
    ephemeris: De441<File>,
    stars: HashMap<u32, Hip2Record>,
    delta_t: Option<DeltaTTable>,
    chiron: Option<SmallBodySpk>,
}

#[napi]
impl Engine {
    /// Open a classic JPL DE441 binary. The path is the caller's; nothing is
    /// searched, downloaded or defaulted. A `.bsp` or `.se1` is rejected by the
    /// core's header validation.
    #[napi(constructor)]
    pub fn new(de441_path: String) -> Result<Self> {
        let ephemeris = De441::open(&de441_path).map_err(|e| fail("E_EPHEMERIS_FILE", e))?;
        Ok(Self {
            ephemeris,
            stars: HashMap::new(),
            delta_t: None,
            chiron: None,
        })
    }

    fn calc(
        &mut self,
        jd_tdb: JulianDateTdb,
        ipl: i32,
        flags: i32,
        time_source: String,
    ) -> Result<CalcResult> {
        check_flags(flags)?;
        if let Some(point) = LunarPoint::from_swiss_number(ipl) {
            return self.calc_lunar_point(point, jd_tdb, flags, time_source);
        }
        if ipl == BODY_CHIRON {
            return self.calc_chiron(jd_tdb, flags, time_source);
        }
        let body = body_from_ipl(ipl)?;
        let helio = flags & FLAG_HELCTR != 0;
        if ipl == BODY_EARTH && !helio {
            return Err(fail(
                "E_UNSUPPORTED_BODY",
                "the Earth has no geocentric position; use SEFLG_HELCTR",
            ));
        }
        let opts = options(flags);
        let mut notes: Vec<String> = applied_corrections(opts)
            .into_iter()
            .map(str::to_string)
            .collect();
        if (4..=9).contains(&ipl) {
            notes.push("target is the planetary SYSTEM BARYCENTER, not the planet center".into());
        }
        let six = if helio && ipl == 0 {
            // The Sun seen from the Sun: all zeros, as the reference binding returns.
            notes.push("the Sun seen from the Sun is the zero vector".into());
            [0.0; 6]
        } else {
            let rows = geocentric_apparent_six_fields_many_with_options(
                &mut self.ephemeris,
                &[body],
                jd_tdb,
                opts,
            )
            .map_err(|e| fail("E_CALC", e))?;
            let f = rows[0].fields;
            [
                f.longitude_deg,
                f.latitude_deg,
                f.distance_au,
                f.longitude_speed_deg_per_tdb_day,
                f.latitude_speed_deg_per_tdb_day,
                f.distance_speed_au_per_tdb_day,
            ]
        };
        Ok(CalcResult {
            xx: zero_speeds_unless_requested(six, flags),
            rflag: reported_flags(flags),
            time_source,
            notes,
        })
    }

    fn calc_chiron(
        &mut self,
        jd_tdb: JulianDateTdb,
        flags: i32,
        time_source: String,
    ) -> Result<CalcResult> {
        let spk = self.chiron.as_ref().ok_or_else(|| {
            fail(
                "E_UNSUPPORTED_BODY",
                "Chiron needs a small-body SPK: call loadChiron(path) first",
            )
        })?;
        let opts = options(flags);
        let f = small_body_apparent_six_fields(&mut self.ephemeris, spk, jd_tdb, opts)
            .map_err(|e| fail("E_CALC", e))?
            .fields;
        let mut notes: Vec<String> = applied_corrections(opts)
            .into_iter()
            .map(str::to_string)
            .collect();
        notes.push(
            "Chiron from the caller-supplied JPL small-body SPK (type 21) plus DE441 Sun".into(),
        );
        Ok(CalcResult {
            xx: zero_speeds_unless_requested(
                [
                    f.longitude_deg,
                    f.latitude_deg,
                    f.distance_au,
                    f.longitude_speed_deg_per_tdb_day,
                    f.latitude_speed_deg_per_tdb_day,
                    f.distance_speed_au_per_tdb_day,
                ],
                flags,
            ),
            rflag: reported_flags(flags),
            time_source,
            notes,
        })
    }

    /// Load a JPL small-body SPK (segment type 21, Sun-centered) for 2060 Chiron
    /// from a caller-supplied file. Body 15 then works through the same apparent
    /// pipeline as the planets; a failed load keeps the previous file. Nothing is
    /// downloaded or bundled.
    #[napi]
    pub fn load_chiron(&mut self, spk_path: String) -> Result<()> {
        let bytes =
            std::fs::read(&spk_path).map_err(|e| fail("E_SPK_FILE", format!("{spk_path}: {e}")))?;
        let spk = SmallBodySpk::from_bytes(bytes).map_err(|e| fail("E_SPK_FILE", e))?;
        if spk.target_id() != CHIRON_NAIF_ID || spk.center_id() != 10 {
            return Err(fail(
                "E_SPK_FILE",
                format!(
                    "expected 2060 Chiron ({CHIRON_NAIF_ID}) relative to the Sun, got {} relative to {}",
                    spk.target_id(),
                    spk.center_id()
                ),
            ));
        }
        self.chiron = Some(spk);
        Ok(())
    }

    fn calc_lunar_point(
        &mut self,
        point: LunarPoint,
        jd_tdb: JulianDateTdb,
        flags: i32,
        time_source: String,
    ) -> Result<CalcResult> {
        let opts = options(flags);
        let mut notes = vec![
            "mathematical lunar point: no light-time, aberration or deflection".to_string(),
            "true ecliptic/equator of date (mean position plus IAU 2000B nutation for the mean points)".to_string(),
        ];
        let six = if opts.center == ApparentCenter::Heliocentric {
            // Seen from the Sun a point of the Moon's orbit is undefined: zeros, as the reference.
            notes.push("heliocentric request: the reference returns the zero vector".into());
            [0.0; 6]
        } else {
            let state = lunar_point(Some(&mut self.ephemeris), point, opts.frame, jd_tdb.0)
                .map_err(|e| fail("E_CALC", e))?;
            [
                state.longitude_deg,
                state.latitude_deg,
                state.distance_au,
                state.longitude_rate_deg_per_day,
                state.latitude_rate_deg_per_day,
                state.distance_rate_au_per_day,
            ]
        };
        Ok(CalcResult {
            xx: zero_speeds_unless_requested(six, flags),
            rflag: reported_flags(flags),
            time_source,
            notes,
        })
    }

    /// `swe_calc`-shaped: the epoch is a **TDB** Julian date (Swiss calls it
    /// ephemeris time; TT and TDB differ by < 2 ms and the difference is not
    /// hidden: it is the caller's input).
    #[napi]
    pub fn calc_tdb(&mut self, jd_tdb: f64, ipl: i32, flags: i32) -> Result<CalcResult> {
        if !jd_tdb.is_finite() {
            return Err(fail("E_RANGE", "non-finite Julian date"));
        }
        self.calc(JulianDateTdb(jd_tdb), ipl, flags, "TDB_INPUT".to_string())
    }

    /// Load measured/predicted ΔT from three caller-supplied USNO files
    /// (`deltat.data`, `historic_deltat.data` and optionally `deltat.preds`). From
    /// then on `calc_ut` converts UT with this table (range: the table, from
    /// 1900-01-01 UT) and says so in `time_source`. A failed load keeps the
    /// previous table. Nothing is downloaded or bundled.
    #[napi]
    pub fn load_delta_t(
        &mut self,
        deltat_data_path: String,
        historic_deltat_path: String,
        deltat_preds_path: Option<String>,
    ) -> Result<()> {
        let read = |path: &str| {
            std::fs::read_to_string(path).map_err(|e| fail("E_DELTAT_FILE", format!("{path}: {e}")))
        };
        let monthly = read(&deltat_data_path)?;
        let historic = read(&historic_deltat_path)?;
        let preds = match &deltat_preds_path {
            Some(path) => Some(read(path)?),
            None => None,
        };
        let table = DeltaTTable::from_usno(&monthly, &historic, preds.as_deref())
            .map_err(|e| fail("E_DELTAT_FILE", e))?;
        self.delta_t = Some(table);
        Ok(())
    }

    /// `swe_calc_ut`-shaped: the epoch is a **UT** Julian date. With a ΔT table
    /// loaded it is converted with measured ΔT (and predictions after the last
    /// observation, flagged); otherwise under the declared polynomial/leap-second
    /// policy (1900-01-01 … 2027-06-28 UT only).
    #[napi]
    pub fn calc_ut(&mut self, jd_ut: f64, ipl: i32, flags: i32) -> Result<CalcResult> {
        check_flags(flags)?;
        let (tdb, source) = match &self.delta_t {
            Some(table) => jd_ut_to_tdb_with_table(jd_ut, table),
            None => jd_ut_to_tdb_with_policy(jd_ut),
        }
        .map_err(|e| fail("E_RANGE", e))?;
        self.calc(tdb, ipl, flags, source_label(&source))
    }

    /// UT -> (TDB, TT, label) under the engine's current time policy.
    fn epoch_from_ut(&self, jd_ut: f64) -> Result<(JulianDateTdb, f64, String)> {
        let (tdb, source) = match &self.delta_t {
            Some(table) => jd_ut_to_tdb_with_table(jd_ut, table),
            None => jd_ut_to_tdb_with_policy(jd_ut),
        }
        .map_err(|e| fail("E_RANGE", e))?;
        let tt = tdb_to_tt_jd(tdb.0).map_err(|e| fail("E_RANGE", e))?;
        Ok((tdb, tt, source_label(&source)))
    }

    /// House astronomy from a UT Julian date under this engine's time policy: with a
    /// loaded ΔT table TT is the measured one (the free function uses the declared
    /// polynomial/leap-second policy only).
    #[napi]
    pub fn house_angles_ut(
        &self,
        jd_ut: f64,
        latitude_deg: f64,
        longitude_deg_east: f64,
    ) -> Result<HouseAnglesResult> {
        let (_, tt, time_source) = self.epoch_from_ut(jd_ut)?;
        angles_result(jd_ut, tt, latitude_deg, longitude_deg_east, time_source)
    }

    /// `swe_azalt`-shaped. `equatorial_input` false: `x0, x1` are true ecliptic
    /// longitude/latitude of date; true: right ascension/declination of date.
    /// A pressure of 0 is estimated from `geoalt`; a temperature of 0 is 0 degrees C.
    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn azalt_ut(
        &self,
        jd_ut: f64,
        equatorial_input: bool,
        longitude_deg_east: f64,
        latitude_deg: f64,
        geoalt_m: f64,
        pressure_hpa: f64,
        temperature_c: f64,
        x0_deg: f64,
        x1_deg: f64,
    ) -> Result<AzaltResult> {
        let (_, tt, time_source) = self.epoch_from_ut(jd_ut)?;
        let (ra, dec) = if equatorial_input {
            (x0_deg, x1_deg)
        } else {
            ecliptic_to_equatorial(tt, x0_deg, x1_deg).map_err(|e| fail("E_RANGE", e))?
        };
        let armc = local_apparent_sidereal_deg(jd_ut, tt, longitude_deg_east)
            .map_err(|e| fail("E_RANGE", e))?;
        let h = equatorial_to_horizontal(ra, dec, armc, latitude_deg)
            .map_err(|e| fail("E_RANGE", e))?;
        let pressure = if pressure_hpa == 0.0 {
            estimated_pressure_hpa(geoalt_m)
        } else {
            pressure_hpa
        };
        let refracted = extended_true_to_apparent(
            h.true_altitude_deg,
            geoalt_m,
            pressure,
            temperature_c,
            STANDARD_LAPSE_RATE,
        );
        Ok(AzaltResult {
            azimuth_south_deg: (h.azimuth_north_east_deg + 180.0).rem_euclid(360.0),
            true_altitude_deg: h.true_altitude_deg,
            apparent_altitude_deg: refracted.apparent_altitude_deg,
            time_source,
        })
    }

    /// `swe_azalt_rev`-shaped: azimuth from the south and TRUE altitude to ecliptic
    /// longitude/latitude (`equatorial_output` false) or right ascension/declination.
    #[napi]
    pub fn azalt_rev_ut(
        &self,
        jd_ut: f64,
        equatorial_output: bool,
        longitude_deg_east: f64,
        latitude_deg: f64,
        azimuth_south_deg: f64,
        true_altitude_deg: f64,
    ) -> Result<AzaltRevResult> {
        let (_, tt, time_source) = self.epoch_from_ut(jd_ut)?;
        let armc = local_apparent_sidereal_deg(jd_ut, tt, longitude_deg_east)
            .map_err(|e| fail("E_RANGE", e))?;
        let north_east = (azimuth_south_deg - 180.0).rem_euclid(360.0);
        let (ra, dec) = horizontal_to_equatorial(north_east, true_altitude_deg, armc, latitude_deg)
            .map_err(|e| fail("E_RANGE", e))?;
        let (x0, x1) = if equatorial_output {
            (ra, dec)
        } else {
            let eps = ecliptic_nutation(tt)
                .map_err(|e| fail("E_RANGE", e))?
                .true_obliquity_rad;
            let (a, d) = (ra.to_radians(), dec.to_radians());
            let (x, y, z) = (d.cos() * a.cos(), d.cos() * a.sin(), d.sin());
            let ye = y * eps.cos() + z * eps.sin();
            let ze = -y * eps.sin() + z * eps.cos();
            (
                ye.atan2(x).to_degrees().rem_euclid(360.0),
                ze.atan2(x.hypot(ye)).to_degrees(),
            )
        };
        Ok(AzaltRevResult {
            x0_deg: x0,
            x1_deg: x1,
            time_source,
        })
    }

    /// Apparent geocentric equatorial-of-date position of a body at a UT instant.
    fn sky_position(&mut self, jd_ut: f64, ipl: i32) -> Result<SkyPosition> {
        let (tdb, tt, _) = self.epoch_from_ut(jd_ut)?;
        let opts = options(FLAG_EQUATORIAL);
        let f = if ipl == BODY_CHIRON {
            let spk = self.chiron.as_ref().ok_or_else(|| {
                fail(
                    "E_UNSUPPORTED_BODY",
                    "Chiron needs a small-body SPK: call loadChiron(path) first",
                )
            })?;
            small_body_apparent_six_fields(&mut self.ephemeris, spk, tdb, opts)
                .map_err(|e| fail("E_CALC", e))?
                .fields
        } else {
            if ipl == BODY_EARTH {
                return Err(fail("E_UNSUPPORTED_BODY", "the Earth has no rise or set"));
            }
            let body = body_from_ipl(ipl)?;
            geocentric_apparent_six_fields_many_with_options(
                &mut self.ephemeris,
                &[body],
                tdb,
                opts,
            )
            .map_err(|e| fail("E_CALC", e))?[0]
                .fields
        };
        Ok(SkyPosition {
            tt_jd: tt,
            right_ascension_deg: f.longitude_deg,
            declination_deg: f.latitude_deg,
            distance_au: f.distance_au,
        })
    }

    /// `swe_rise_trans`-shaped for the Sun, Moon, planets and Chiron: the first
    /// rise, set, upper or lower transit strictly after `jd_ut` (`rsmi` carries the
    /// Swiss event and option bits). `None` means no event within the search window
    /// (polar day or night, circumpolar) in the next `search_days` days (at most 31).
    /// Pressure 0 is estimated from `geoalt`.
    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn rise_trans_ut(
        &mut self,
        jd_ut: f64,
        ipl: i32,
        rsmi: i32,
        longitude_deg_east: f64,
        latitude_deg: f64,
        geoalt_m: f64,
        pressure_hpa: f64,
        temperature_c: f64,
        search_days: f64,
    ) -> Result<Option<f64>> {
        let event = match rsmi & 15 {
            RISE => Event::Rise,
            SET => Event::Set,
            MTRANSIT => Event::UpperTransit,
            ITRANSIT => Event::LowerTransit,
            _ => {
                return Err(fail(
                    "E_RANGE",
                    "rsmi must carry exactly one of RISE(1), SET(2), MTRANSIT(4), ITRANSIT(8)",
                ))
            }
        };
        let known = 15
            | BIT_DISC_CENTER
            | BIT_NO_REFRACTION
            | BIT_CIVIL_TWILIGHT
            | BIT_NAUTIC_TWILIGHT
            | BIT_ASTRO_TWILIGHT
            | BIT_DISC_BOTTOM
            | BIT_FIXED_DISC_SIZE
            | BIT_FORCE_SLOW_METHOD;
        if rsmi & BIT_GEOCTR_NO_ECL_LAT != 0 || rsmi & !known != 0 {
            return Err(fail(
                "E_UNSUPPORTED_FLAG",
                format!(
                    "rise/transit bits {:#x} are not supported (HINDU_RISING and GEOCTR_NO_ECL_LAT need a geocentric ecliptic latitude convention)",
                    rsmi & (BIT_GEOCTR_NO_ECL_LAT | !known)
                ),
            ));
        }
        if ipl != BODY_CHIRON {
            body_from_ipl(ipl)?;
        }
        let twilight_centre = if rsmi & BIT_CIVIL_TWILIGHT != 0 {
            Some(-6.0)
        } else if rsmi & BIT_NAUTIC_TWILIGHT != 0 {
            Some(-12.0)
        } else if rsmi & BIT_ASTRO_TWILIGHT != 0 {
            Some(-18.0)
        } else {
            None
        };
        let limb = if rsmi & BIT_DISC_CENTER != 0 {
            Limb::Center
        } else if rsmi & BIT_DISC_BOTTOM != 0 {
            Limb::Lower
        } else {
            Limb::Upper
        };
        let disc = match body_radius_km(ipl) {
            None => Disc::Point,
            Some(km) if rsmi & BIT_FIXED_DISC_SIZE != 0 && (ipl == 0 || ipl == 1) => {
                let mean_distance_km = if ipl == 0 {
                    AU_KM
                } else {
                    MEAN_MOON_DISTANCE_KM
                };
                Disc::FixedSemiDiameter {
                    degrees: (km / mean_distance_km).asin().to_degrees(),
                    limb,
                }
            }
            Some(km) => Disc::Radius { km, limb },
        };
        let request = Request {
            event,
            disc,
            refraction: rsmi & BIT_NO_REFRACTION == 0 && twilight_centre.is_none(),
            topocentric: true,
            fixed_centre_altitude_deg: twilight_centre,
            search_days,
        };
        let site = Site {
            latitude_deg,
            longitude_deg_east,
            height_m: geoalt_m,
        };
        let atmosphere = Atmosphere {
            pressure_hpa: if pressure_hpa == 0.0 {
                estimated_pressure_hpa(geoalt_m)
            } else {
                pressure_hpa
            },
            temperature_c,
        };
        find_event(
            |ut| self.sky_position(ut, ipl),
            &site,
            &atmosphere,
            &request,
            jd_ut,
        )
        .map_err(|e| match e {
            RiseError::Position(inner) => inner,
            RiseError::Site(site_error) => fail("E_RANGE", site_error),
            RiseError::InvalidRequest(message) => fail("E_RANGE", message),
        })
    }

    /// Apparent geocentric Sun and Moon vectors (km, true equator of date) at a UT instant.
    fn sun_moon_km(&mut self, jd_ut: f64) -> Result<SunMoon> {
        let (tdb, _, _) = self.epoch_from_ut(jd_ut)?;
        let rows = geocentric_apparent_six_fields_many_with_options(
            &mut self.ephemeris,
            &[Body::Sun, Body::Moon],
            tdb,
            options(FLAG_EQUATORIAL),
        )
        .map_err(|e| fail("E_CALC", e))?;
        let vector = |i: usize| {
            let f = rows[i].fields;
            let (ra, dec, d) = (
                f.longitude_deg.to_radians(),
                f.latitude_deg.to_radians(),
                f.distance_au * AU_KM,
            );
            [
                d * dec.cos() * ra.cos(),
                d * dec.cos() * ra.sin(),
                d * dec.sin(),
            ]
        };
        Ok(SunMoon {
            sun_km: vector(0),
            moon_km: vector(1),
        })
    }

    /// `swe_lun_eclipse_when`-shaped: the first lunar eclipse (penumbral included)
    /// whose maximum is after (`backward`: before) `jd_ut`; all times UT. The Earth's shadow
    /// is enlarged by 1 % for the atmosphere.
    #[napi]
    pub fn lunar_eclipse_ut(&mut self, jd_ut: f64, backward: bool) -> Result<LunarEclipseResult> {
        let found =
            next_lunar_eclipse(|ut| self.sun_moon_km(ut), jd_ut, 400.0, backward).map_err(|e| {
                match e {
                    EclipseError::Position(inner) => inner,
                    other => fail("E_CALC", other),
                }
            })?;
        Ok(LunarEclipseResult {
            kind: match found.kind {
                LunarKind::Penumbral => 64,
                LunarKind::Partial => 16,
                LunarKind::Total => 4,
            },
            maximum: found.maximum_ut,
            penumbral_begin: found.penumbral_begin_ut,
            penumbral_end: found.penumbral_end_ut,
            partial_begin: found.partial_begin_ut,
            partial_end: found.partial_end_ut,
            total_begin: found.total_begin_ut,
            total_end: found.total_end_ut,
            umbral_magnitude: found.umbral_magnitude,
            penumbral_magnitude: found.penumbral_magnitude,
        })
    }

    /// `swe_sol_eclipse_when_glob`-shaped: the first solar eclipse whose greatest eclipse
    /// is after `jd_ut`; all times UT. Geometric shadow on the WGS84 ellipsoid.
    #[napi]
    pub fn solar_eclipse_ut(&mut self, jd_ut: f64, backward: bool) -> Result<SolarEclipseResult> {
        let found =
            next_solar_eclipse(|ut| self.sun_moon_km(ut), jd_ut, 400.0, backward).map_err(|e| {
                match e {
                    EclipseError::Position(inner) => inner,
                    other => fail("E_CALC", other),
                }
            })?;
        let kind = match found.kind {
            SolarKind::Partial => 16,
            SolarKind::Annular => 8,
            SolarKind::Total => 4,
            SolarKind::Hybrid => 32,
        };
        Ok(SolarEclipseResult {
            kind: kind | if found.central { 1 } else { 2 },
            maximum: found.maximum_ut,
            partial_begin: found.partial_begin_ut,
            partial_end: found.partial_end_ut,
            total_begin: found.total_begin_ut,
            total_end: found.total_end_ut,
            center_begin: found.center_begin_ut,
            center_end: found.center_end_ut,
            gamma: found.gamma,
            noon: found.noon_ut.unwrap_or(0.0),
        })
    }

    /// Load and validate Hipparcos records for the given HIP numbers from a
    /// caller-supplied `hip2.dat`. `policy` is `"strict"` or `"traditional"`.
    /// A requested star that fails the policy or is absent is an error; the
    /// previous selection is kept untouched on failure. Returns the count.
    #[napi]
    pub fn load_hipparcos(
        &mut self,
        hip2_path: String,
        hips: Vec<u32>,
        policy: String,
    ) -> Result<u32> {
        let policy = match policy.as_str() {
            "strict" => Hip2QualityPolicy::STRICT,
            "traditional" => Hip2QualityPolicy::TRADITIONAL_STARS,
            other => return Err(fail("E_RANGE", format!("unknown policy {other:?}"))),
        };
        let file = File::open(&hip2_path).map_err(|e| fail("E_CATALOG_FILE", e))?;
        let records = read_hip2_selected(BufReader::new(file), &hips)
            .map_err(|e| fail("E_CATALOG_FILE", e))?;
        for record in &records {
            if !policy.accepts(record) {
                return Err(fail(
                    "E_STAR_QUALITY",
                    format!(
                        "HIP {} rejected by policy (Sn {}, Nc {})",
                        record.hip, record.solution_type, record.components
                    ),
                ));
            }
        }
        self.stars = records.into_iter().map(|r| (r.hip, r)).collect();
        Ok(self.stars.len() as u32)
    }

    /// Apparent position of a loaded Hipparcos star at a TDB epoch.
    /// `rv_km_s = null` means an **explicit assumption of 0 km/s**, reported in
    /// `notes`; it is never a silent default.
    #[napi]
    pub fn fixstar_tdb(
        &mut self,
        hip: u32,
        rv_km_s: Option<f64>,
        jd_tdb: f64,
        flags: i32,
    ) -> Result<CalcResult> {
        check_flags(flags)?;
        if flags & (FLAG_HELCTR | FLAG_EQUATORIAL) != 0 {
            return Err(fail(
                "E_UNSUPPORTED_FLAG",
                "fixed stars support neither SEFLG_HELCTR nor SEFLG_EQUATORIAL yet",
            ));
        }
        let record = self
            .stars
            .get(&hip)
            .copied()
            .ok_or_else(|| fail("E_STAR_NOT_LOADED", format!("HIP {hip}")))?;
        let (rv, mut notes) = match rv_km_s {
            Some(v) => (RadialVelocity::Measured(v), Vec::new()),
            None => (
                RadialVelocity::AssumedZero,
                vec!["radial velocity assumed to be 0 km/s (no measurement supplied)".to_string()],
            ),
        };
        let entry = record
            .to_catalog_entry(rv)
            .map_err(|e| fail("E_STAR_QUALITY", e))?;
        if !jd_tdb.is_finite() {
            return Err(fail("E_RANGE", "non-finite Julian date"));
        }
        let tt = tdb_to_tt_jd(jd_tdb).map_err(|e| fail("E_RANGE", e))?;
        let deflection = if flags & FLAG_NOGDEFL == 0 {
            StellarSolarDeflection::PointMass
        } else {
            StellarSolarDeflection::Omit
        };
        let p = calculate_star_with_solar_deflection(
            &mut self.ephemeris,
            entry,
            tt,
            JulianDateTdb(jd_tdb),
            deflection,
        )
        .map_err(|e| fail("E_CALC", e))?;
        notes.push(
            "Hipparcos 2007 astrometry, rectilinear motion, annual parallax and aberration".into(),
        );
        if deflection == StellarSolarDeflection::PointMass {
            notes.push("first-order solar light deflection".into());
        }
        Ok(CalcResult {
            xx: zero_speeds_unless_requested(
                [
                    p.longitude_deg,
                    p.latitude_deg,
                    p.distance_au,
                    p.longitude_speed_deg_per_tdb_day,
                    p.latitude_speed_deg_per_tdb_day,
                    p.distance_speed_au_per_tdb_day,
                ],
                flags,
            ),
            rflag: (flags & (FLAG_SPEED | FLAG_NOGDEFL)) | FLAG_JPLEPH,
            time_source: "TDB_INPUT".to_string(),
            notes,
        })
    }

    /// Fixed-star position at a UT epoch using the engine's current time policy.
    #[napi]
    pub fn fixstar_ut(
        &mut self,
        hip: u32,
        rv_km_s: Option<f64>,
        jd_ut: f64,
        flags: i32,
    ) -> Result<CalcResult> {
        let (tdb, _, source) = self.epoch_from_ut(jd_ut)?;
        let mut result = self.fixstar_tdb(hip, rv_km_s, tdb.0, flags)?;
        result.time_source = source;
        Ok(result)
    }
}
