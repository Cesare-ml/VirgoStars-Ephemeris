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

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    applied_corrections, geocentric_apparent_six_fields_many_with_options, tdb_to_tt_jd,
    ApparentCenter, ApparentFrame, ApparentOptions,
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
use virgostars_ephemeris_core::fixed_star::{
    calculate_star_with_solar_deflection, StellarSolarDeflection,
};
use virgostars_ephemeris_core::hipparcos::{
    read_hip2_selected, Hip2QualityPolicy, Hip2Record, RadialVelocity,
};
use virgostars_ephemeris_core::house_angles::house_angles;

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
}
