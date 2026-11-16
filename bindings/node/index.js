'use strict';
// VirgoStars-Ephemeris Node binding: loader, constants and a Swiss-shaped facade.
// Experimental. No astronomy is computed here; see src/lib.rs and the Rust core.
// Licensed CPAL-1.0 (see LICENSE); "Powered by VirgoStars" attribution applies.

const path = require('node:path');

let native;
function loadNative() {
  if (!native) {
    try {
      native = require(path.join(__dirname, 'native', 'virgostars_ephemeris.node'));
    } catch (error) {
      const wrapped = new Error(`E_NATIVE_MISSING: build the addon first (npm run build): ${error.message}`);
      wrapped.code = 'E_NATIVE_MISSING';
      throw wrapped;
    }
  }
  return native;
}

// Public Swiss-style constant values (the C API's documented numbers).
const constants = Object.freeze({
  SE_SUN: 0, SE_MOON: 1, SE_MERCURY: 2, SE_VENUS: 3, SE_MARS: 4,
  SE_JUPITER: 5, SE_SATURN: 6, SE_URANUS: 7, SE_NEPTUNE: 8, SE_PLUTO: 9,
  SE_MEAN_NODE: 10, SE_TRUE_NODE: 11, SE_MEAN_APOG: 12, SE_OSCU_APOG: 13, SE_CHIRON: 15,
  SE_GREG_CAL: 1, SE_JUL_CAL: 0,
  SE_EARTH: 14,
  SEFLG_JPLEPH: 1, SEFLG_SWIEPH: 2, SEFLG_MOSEPH: 4, SEFLG_HELCTR: 8, SEFLG_SPEED: 256,
  SEFLG_NOGDEFL: 512, SEFLG_NOABERR: 1024, SEFLG_EQUATORIAL: 2048,
  SE_ECL2HOR: 0, SE_EQU2HOR: 1, SE_HOR2ECL: 0, SE_HOR2EQU: 1,
  SE_TRUE_TO_APP: 0, SE_APP_TO_TRUE: 1,
  SE_CALC_RISE: 1, SE_CALC_SET: 2, SE_CALC_MTRANSIT: 4, SE_CALC_ITRANSIT: 8,
  SE_BIT_DISC_CENTER: 256, SE_BIT_NO_REFRACTION: 512, SE_BIT_CIVIL_TWILIGHT: 1024,
  SE_BIT_NAUTIC_TWILIGHT: 2048, SE_BIT_ASTRO_TWILIGHT: 4096, SE_BIT_DISC_BOTTOM: 8192,
  SE_BIT_FIXED_DISC_SIZE: 16384, SE_BIT_FORCE_SLOW_METHOD: 32768,
});

const BODY_NAMES = ['Sun', 'Moon', 'Mercury', 'Venus', 'Mars', 'Jupiter', 'Saturn', 'Uranus', 'Neptune', 'Pluto'];
BODY_NAMES[10] = 'mean Node';
BODY_NAMES[11] = 'true Node';
BODY_NAMES[12] = 'mean Apogee';
BODY_NAMES[13] = 'osc. Apogee';
BODY_NAMES[14] = 'Earth';
BODY_NAMES[15] = 'Chiron';

function parseError(error) {
  const text = String(error && error.message ? error.message : error);
  const match = /^(E_[A-Z_]+): ?(.*)$/s.exec(text);
  return match ? { code: match[1], message: match[2] } : { code: 'E_UNKNOWN', message: text };
}

/**
 * One open DE441 file. Engines share no mutable state with each other.
 * Methods are synchronous and throw Errors whose `code` is a stable E_* string.
 */
class Engine {
  constructor(de441Path) {
    if (typeof de441Path !== 'string' || de441Path.length === 0) {
      const e = new Error('E_RANGE: a DE441 file path is required');
      e.code = 'E_RANGE';
      throw e;
    }
    try {
      this._native = new (loadNative().Engine)(de441Path);
    } catch (error) {
      throw Engine._wrap(error);
    }
  }

  static _wrap(error) {
    const { code, message } = parseError(error);
    const wrapped = new Error(`${code}: ${message}`);
    wrapped.code = code;
    return wrapped;
  }

  _call(fn) {
    try {
      return fn();
    } catch (error) {
      throw Engine._wrap(error);
    }
  }

  /** TDB Julian date input. Returns `{ xx, rflag, timeSource, notes }`. */
  calcTdb(jdTdb, ipl, flags = 0) {
    const r = this._call(() => this._native.calcTdb(jdTdb, ipl, flags));
    return { xx: r.xx, rflag: r.rflag, timeSource: r.timeSource, notes: r.notes };
  }

  /** UT Julian date input, converted under the declared time policy. */
  calcUt(jdUt, ipl, flags = 0) {
    const r = this._call(() => this._native.calcUt(jdUt, ipl, flags));
    return { xx: r.xx, rflag: r.rflag, timeSource: r.timeSource, notes: r.notes };
  }

  /**
   * Load measured/predicted DeltaT from caller-supplied USNO files (deltat.data,
   * historic_deltat.data, optionally deltat.preds). Afterwards `calcUt` uses it and
   * `timeSource` reads DELTA_T_TABLE:MEASURED|PREDICTED:<seconds>:<1-sigma>.
   */
  loadDeltaT(deltatDataPath, historicDeltatPath, deltatPredsPath = null) {
    return this._call(() => this._native.loadDeltaT(deltatDataPath, historicDeltatPath, deltatPredsPath));
  }

  /**
   * Load a JPL small-body SPK (type 21, Sun-centered) for 2060 Chiron from a caller
   * supplied file; afterwards body 15 (SE_CHIRON) is available like the planets.
   */
  loadChiron(spkPath) {
    return this._call(() => this._native.loadChiron(spkPath));
  }

  /**
   * Horizon coordinates of an ecliptic (`equatorialInput` false) or equatorial-of-date
   * direction. Azimuth is measured from the SOUTH toward the west. Pressure 0 is
   * estimated from `geoaltM`.
   */
  azaltUt(jdUt, equatorialInput, lonDegEast, latDeg, geoaltM, pressureHpa, temperatureC, x0, x1) {
    const r = this._call(() => this._native.azaltUt(
      jdUt, Boolean(equatorialInput), lonDegEast, latDeg, geoaltM, pressureHpa, temperatureC, x0, x1));
    return {
      azimuthSouthDeg: r.azimuthSouthDeg, trueAltitudeDeg: r.trueAltitudeDeg,
      apparentAltitudeDeg: r.apparentAltitudeDeg, timeSource: r.timeSource,
    };
  }

  /** Inverse of `azaltUt` for a TRUE altitude: ecliptic lon/lat or RA/Dec of date. */
  azaltRevUt(jdUt, equatorialOutput, lonDegEast, latDeg, azimuthSouthDeg, trueAltitudeDeg) {
    const r = this._call(() => this._native.azaltRevUt(
      jdUt, Boolean(equatorialOutput), lonDegEast, latDeg, azimuthSouthDeg, trueAltitudeDeg));
    return { x0Deg: r.x0Deg, x1Deg: r.x1Deg, timeSource: r.timeSource };
  }

  /**
   * First rise/set/transit after `jdUt` for bodies 0..9 and Chiron (when loaded).
   * `rsmi` carries the Swiss event and option bits. Returns the UT Julian date, or
   * `null` when there is no event within `searchDays` days (default 2; at most 31).
   */
  riseTransUt(jdUt, ipl, rsmi, lonDegEast, latDeg, geoaltM, pressureHpa, temperatureC, searchDays = 2) {
    const r = this._call(() => this._native.riseTransUt(
      jdUt, ipl, rsmi, lonDegEast, latDeg, geoaltM, pressureHpa, temperatureC, searchDays));
    return r === undefined ? null : r;
  }

  /** policy: 'strict' | 'traditional'. Returns the number of stars loaded. */
  loadHipparcos(hip2Path, hips, policy) {
    return this._call(() => this._native.loadHipparcos(hip2Path, hips, policy));
  }

  /** `rvKmS = null` is an explicit assumption of 0 km/s, reported in `notes`. */
  fixstarTdb(hip, rvKmS, jdTdb, flags = 0) {
    const r = this._call(() => this._native.fixstarTdb(hip, rvKmS, jdTdb, flags));
    return { xx: r.xx, rflag: r.rflag, timeSource: r.timeSource, notes: r.notes };
  }
}

function julday(year, month, day, hour, gregorian = true) {
  try {
    return loadNative().julday(year, month, day, hour, Boolean(gregorian));
  } catch (error) {
    throw Engine._wrap(error);
  }
}

function revjul(jd, gregorian = true) {
  try {
    return loadNative().revjul(jd, Boolean(gregorian));
  } catch (error) {
    throw Engine._wrap(error);
  }
}

/** House astronomy (ARMC, true obliquity, ASC, MC) from a UT Julian date. */
function houseAnglesUt(jdUt, latitudeDeg, longitudeDegEast) {
  try {
    return loadNative().houseAnglesUt(jdUt, latitudeDeg, longitudeDegEast);
  } catch (error) {
    throw Engine._wrap(error);
  }
}

/**
 * Provider for `astrological-functions` (`calculateHouseAngles`): the caller
 * supplies UT1 and TT explicitly, so no time conversion happens here.
 */
function createHouseAngleProvider() {
  return Object.freeze({
    calculateHouseAngles({ ut1JulianDay, ttJulianDay, latitudeDeg, longitudeDegEast }) {
      let r;
      try {
        r = loadNative().houseAnglesExplicit(ut1JulianDay, ttJulianDay, latitudeDeg, longitudeDegEast);
      } catch (error) {
        throw Engine._wrap(error);
      }
      return Object.freeze({
        model: 'IAU2006_2000B_APPROXIMATE_SIDEREAL_TIME',
        ut1JulianDay: r.ut1JulianDay,
        ttJulianDay: r.ttJulianDay,
        latitudeDeg: r.latitudeDeg,
        longitudeDegEast: r.longitudeDegEast,
        armcDeg: r.armcDeg,
        ascendantDeg: r.ascendantDeg,
        midheavenDeg: r.midheavenDeg,
        trueObliquityDeg: r.trueObliquityDeg,
      });
    },
  });
}

/**
 * Lenient `swe_julday`: month, day and hour overflow arithmetically (month 13 is
 * January of the next year, day 0 is the day before, hour 25 is 1h of the next
 * day), year/month/day are truncated to integers and non-finite input gives NaN,
 * which is what the installed `swisseph` 0.5.17 does (observed, see
 * docs/MVP1_SWISSEPH_SHAPE_2026-11-16.md). The strict `julday` stays available.
 */
function lenientJulday(year, month, day, hour, gregorian) {
  const nums = [year, month, day, hour];
  if (!nums.every((v) => typeof v === 'number' && Number.isFinite(v))) return NaN;
  const y0 = Math.trunc(year);
  const m0 = Math.trunc(month) - 1;
  const y = y0 + Math.floor(m0 / 12);
  const m = ((m0 % 12) + 12) % 12 + 1;
  const firstOfMonth = julday(y, m, 1, 0, gregorian);
  return firstOfMonth + (Math.trunc(day) - 1) + hour / 24;
}

// The reference reports -2 ("no event") when the first event is more than about
// 26 hours after the start (measured: found at 1.0811 d, missed at 1.0855 d).
const SWISS_EVENT_WINDOW_DAYS = 26 / 24;

function coded(code, message) {
  const e = new Error(`${code}: ${message}`);
  e.code = code;
  return e;
}

/** Names of the bodies this engine computes, as the Swiss binding prints them. */
const BODY_NAMES_BY_NUMBER = BODY_NAMES;

/**
 * Swiss-shaped synchronous facade over an Engine, for the call shapes the
 * VirgoStars backend uses: `swe_julday`, `swe_revjul`, `swe_calc_ut`, `swe_calc`
 * (including `SE_ECL_NUT`) and `swe_get_planet_name`.
 *
 * Shape facts verified against the installed `swisseph` 0.5.17 (local probe):
 * result keys `longitude, latitude, distance, longitudeSpeed, latitudeSpeed,
 * distanceSpeed, rflag` and nothing else; speeds are 0 without `SEFLG_SPEED`;
 * failures return an object whose only enumerable key is `error`; a trailing
 * callback is invoked synchronously, before the call returns, and the call also
 * returns the result.
 *
 * Differences that are deliberate and visible:
 *  - `SEFLG_SWIEPH` (the flag the backend passes) is REJECTED unless the facade
 *    is created with `{ acceptSwiephFlag: true }`; then it is accepted as "use the
 *    configured DE441 file" and `rflag` never carries the SWIEPH bit.
 *  - Provenance (`timeSource`, `notes`) is NOT added to results unless the facade
 *    is created with `{ provenance: true }`, so that results have exactly the
 *    Swiss keys; the `Engine` methods always return it.
 *  - `SEFLG_MOSEPH`, helio/equatorial/topocentric/sidereal flags and bodies
 *    outside 0..9 return an `error` object; nothing is approximated.
 *  - The error object also carries a non-enumerable `code`.
 *  - Mars..Pluto are system barycenters (as in Swiss JPL mode).
 */
function createSwissShapedFacade(engine, { acceptSwiephFlag = false, provenance = false } = {}) {
  function deliver(args, result) {
    const callback = typeof args[args.length - 1] === 'function' ? args[args.length - 1] : null;
    if (callback) callback(result);
    return result;
  }

  function shape(r, flags = 0) {
    const equatorial = (flags & constants.SEFLG_EQUATORIAL) !== 0;
    const result = equatorial
      ? {
        rectAscension: r.xx[0], declination: r.xx[1], distance: r.xx[2],
        rectAscensionSpeed: r.xx[3], declinationSpeed: r.xx[4], distanceSpeed: r.xx[5],
        rflag: r.rflag,
      }
      : {
        longitude: r.xx[0], latitude: r.xx[1], distance: r.xx[2],
        longitudeSpeed: r.xx[3], latitudeSpeed: r.xx[4], distanceSpeed: r.xx[5],
        rflag: r.rflag,
      };
    if (provenance) {
      result.timeSource = r.timeSource;
      result.notes = r.notes;
    }
    return result;
  }

  function normalizeFlags(flags) {
    const f = flags | 0;
    if (f & constants.SEFLG_SWIEPH) {
      if (!acceptSwiephFlag) {
        const e = new Error('E_UNSUPPORTED_FLAG: SEFLG_SWIEPH asserts Swiss data files; this engine reads a DE441 file. Create the facade with acceptSwiephFlag to map it explicitly.');
        e.code = 'E_UNSUPPORTED_FLAG';
        throw e;
      }
      return f & ~constants.SEFLG_SWIEPH;
    }
    return f;
  }

  function guarded(fn) {
    try {
      return fn();
    } catch (error) {
      const { code, message } = parseError(error);
      const failure = { error: `${code}: ${message}` };
      Object.defineProperty(failure, 'code', { value: code, enumerable: false });
      return failure;
    }
  }

  function eclNut(tjdEt) {
    const n = loadNative().eclipticNutationTt(tjdEt);
    // Same slots as the Swiss binding: true obliquity, mean obliquity, nutation in
    // longitude, nutation in obliquity (all degrees); the other two are zero.
    return {
      longitude: n.trueObliquityDeg, latitude: n.meanObliquityDeg, distance: n.nutationLongitudeDeg,
      longitudeSpeed: n.nutationObliquityDeg, latitudeSpeed: 0, distanceSpeed: 0, rflag: 0,
    };
  }

  return Object.freeze({
    ...constants,
    SE_ECL_NUT: -1,
    swe_julday: (year, month, day, hour, gregflag = constants.SE_GREG_CAL) =>
      lenientJulday(year, month, day, hour, gregflag === constants.SE_GREG_CAL),
    swe_revjul: (jd, gregflag = constants.SE_GREG_CAL) => revjul(jd, gregflag === constants.SE_GREG_CAL),
    swe_calc_ut: (...args) =>
      deliver(args, guarded(() => {
        const flags = normalizeFlags(args[2] ?? 0);
        return shape(engine.calcUt(args[0], args[1], flags), flags);
      })),
    swe_calc: (...args) =>
      deliver(args, guarded(() => {
        if (args[1] === -1) return eclNut(args[0]);
        const flags = normalizeFlags(args[2] ?? 0);
        return shape(engine.calcTdb(args[0], args[1], flags), flags);
      })),
    swe_get_planet_name: (ipl) => ({ name: BODY_NAMES_BY_NUMBER[ipl] ?? String(ipl) }),
    // swe_azalt(tjd_ut, calc_flag, geolon, geolat, geoalt, atpress, attemp, x0, x1, x2[, cb])
    swe_azalt: (...args) =>
      deliver(args, guarded(() => {
        const [jd, flag, lon, lat, alt, pressure, temperature, x0, x1] = args;
        if (flag !== constants.SE_ECL2HOR && flag !== constants.SE_EQU2HOR) {
          throw coded('E_RANGE', `calc_flag must be SE_ECL2HOR (0) or SE_EQU2HOR (1), got ${flag}`);
        }
        const r = engine.azaltUt(jd, flag === constants.SE_EQU2HOR, lon, lat, alt ?? 0, pressure ?? 0, temperature ?? 0, x0, x1);
        const result = {
          azimuth: r.azimuthSouthDeg, trueAltitude: r.trueAltitudeDeg, apparentAltitude: r.apparentAltitudeDeg,
        };
        if (provenance) result.timeSource = r.timeSource;
        return result;
      })),
    // swe_azalt_rev(tjd_ut, calc_flag, geolon, geolat, geoalt, azimuth(south), trueAltitude[, cb])
    swe_azalt_rev: (...args) =>
      deliver(args, guarded(() => {
        const [jd, flag, lon, lat, , azimuth, altitude] = args;
        if (flag !== constants.SE_HOR2ECL && flag !== constants.SE_HOR2EQU) {
          throw coded('E_RANGE', `calc_flag must be SE_HOR2ECL (0) or SE_HOR2EQU (1), got ${flag}`);
        }
        const r = engine.azaltRevUt(jd, flag === constants.SE_HOR2EQU, lon, lat, azimuth, altitude);
        // The reference binding names its two outputs azimuth/trueAltitude even for ecliptic/equatorial results.
        const result = { azimuth: r.x0Deg, trueAltitude: r.x1Deg };
        if (provenance) result.timeSource = r.timeSource;
        return result;
      })),
    swe_refrac: (...args) =>
      deliver(args, guarded(() => {
        const [altitude, pressure, temperature, flag] = args;
        return { refraction: refrac(altitude, pressure, temperature, flag !== constants.SE_APP_TO_TRUE) };
      })),
    swe_refrac_extended: (...args) =>
      deliver(args, guarded(() => {
        const [altitude, geoalt, pressure, temperature, lapse, flag] = args;
        const toApparent = flag !== constants.SE_APP_TO_TRUE;
        const r = refracExtended(altitude, geoalt, pressure, temperature, lapse, toApparent);
        return {
          refraction: toApparent ? r.apparentAltitudeDeg : r.trueAltitudeDeg,
          trueAltitude: r.trueAltitudeDeg, apparentAltitude: r.apparentAltitudeDeg, horizonDip: r.dipDeg,
        };
      })),
    // swe_rise_trans(tjd_ut, ipl, starname, epheflag, rsmi, geolon, geolat, geoalt, atpress, attemp[, cb])
    swe_rise_trans: (...args) =>
      deliver(args, guarded(() => {
        const [jd, ipl, starname, epheflag, rsmi, lon, lat, alt, pressure, temperature] = args;
        if (typeof starname !== 'string' || typeof epheflag !== 'number' || typeof rsmi !== 'number') {
          throw coded('E_RANGE', 'swe_rise_trans expects (tjd_ut, ipl, starname, epheflag, rsmi, geolon, geolat, geoalt, atpress, attemp); this call has a different shape');
        }
        if (starname !== '') {
          throw coded('E_UNSUPPORTED_BODY', 'rise/set of fixed stars is not implemented');
        }
        const eflag = normalizeFlags(epheflag);
        if ((eflag & ~constants.SEFLG_JPLEPH) !== 0) {
          throw coded('E_UNSUPPORTED_FLAG', `ephemeris flags ${eflag} are not supported by swe_rise_trans`);
        }
        const t = engine.riseTransUt(jd, ipl, rsmi, lon, lat, alt ?? 0, pressure ?? 0, temperature ?? 0, SWISS_EVENT_WINDOW_DAYS);
        return { name: '', transitTime: t === null ? -2 : t };
      })),
  });
}

/** Plain-model refraction (`swe_refrac`): altitude after refraction, degrees. */
function refrac(altitudeDeg, pressureHpa, temperatureC, toApparent = true) {
  try {
    return loadNative().refrac(altitudeDeg, pressureHpa, temperatureC, Boolean(toApparent));
  } catch (error) {
    throw Engine._wrap(error);
  }
}

/** Extended-model refraction (`swe_refrac_extended`, used by azalt and rise/set). */
function refracExtended(altitudeDeg, heightM, pressureHpa, temperatureC, lapseRate, toApparent = true) {
  try {
    const r = loadNative().refracExtended(
      altitudeDeg, heightM, pressureHpa, temperatureC, lapseRate, Boolean(toApparent));
    return { trueAltitudeDeg: r.trueAltitudeDeg, apparentAltitudeDeg: r.apparentAltitudeDeg, dipDeg: r.dipDeg };
  } catch (error) {
    throw Engine._wrap(error);
  }
}

module.exports = {
  Engine,
  refrac,
  refracExtended,
  julday,
  lenientJulday,
  revjul,
  houseAnglesUt,
  createHouseAngleProvider,
  constants,
  createSwissShapedFacade,
  loadNative,
};
