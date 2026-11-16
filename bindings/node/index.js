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
  SE_GREG_CAL: 1, SE_JUL_CAL: 0,
  SEFLG_JPLEPH: 1, SEFLG_SWIEPH: 2, SEFLG_MOSEPH: 4, SEFLG_SPEED: 256, SEFLG_NOGDEFL: 512,
});

const BODY_NAMES = ['Sun', 'Moon', 'Mercury', 'Venus', 'Mars', 'Jupiter', 'Saturn', 'Uranus', 'Neptune', 'Pluto'];

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

/**
 * Swiss-shaped synchronous facade over an Engine, for the call shapes the
 * VirgoStars backend uses: `swe_julday`, `swe_revjul`, `swe_calc_ut`, `swe_calc`.
 *
 * Differences that are deliberate and visible:
 *  - `SEFLG_SWIEPH` (the flag the backend passes) is REJECTED unless the facade
 *    is created with `{ acceptSwiephFlag: true }`; then it is accepted as "use the
 *    configured DE441 file" and `rflag` never carries the SWIEPH bit.
 *  - `SEFLG_MOSEPH`, helio/equatorial/topocentric/sidereal flags and bodies
 *    outside 0..9 return an `error` object; nothing is approximated.
 *  - Mars..Pluto are system barycenters (as in Swiss JPL mode); see `notes`.
 *  - Failures return `{ error, code }` like the Swiss binding; they do not throw.
 *  - A trailing callback receives the same object synchronously.
 *
 * Whether these shapes match the installed `swisseph` 0.5.17 binding exactly
 * has NOT been verified against that package.
 */
function createSwissShapedFacade(engine, { acceptSwiephFlag = false } = {}) {
  function withCallback(args, compute) {
    const callback = typeof args[args.length - 1] === 'function' ? args[args.length - 1] : null;
    const result = compute();
    if (callback) {
      callback(result);
      return undefined;
    }
    return result;
  }

  function shape(r) {
    return {
      longitude: r.xx[0], latitude: r.xx[1], distance: r.xx[2],
      longitudeSpeed: r.xx[3], latitudeSpeed: r.xx[4], distanceSpeed: r.xx[5],
      rflag: r.rflag, timeSource: r.timeSource, notes: r.notes,
    };
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
      return { error: `${code}: ${message}`, code };
    }
  }

  return Object.freeze({
    ...constants,
    swe_julday: (year, month, day, hour, gregflag = constants.SE_GREG_CAL) =>
      julday(year, month, day, hour, gregflag === constants.SE_GREG_CAL),
    swe_revjul: (jd, gregflag = constants.SE_GREG_CAL) => revjul(jd, gregflag === constants.SE_GREG_CAL),
    swe_calc_ut: (...args) =>
      withCallback(args, () => guarded(() => shape(engine.calcUt(args[0], args[1], normalizeFlags(args[2] ?? 0))))),
    swe_calc: (...args) =>
      withCallback(args, () => guarded(() => shape(engine.calcTdb(args[0], args[1], normalizeFlags(args[2] ?? 0))))),
    swe_get_planet_name: (ipl) => ({ name: BODY_NAMES[ipl] ?? '' }),
  });
}

module.exports = { Engine, julday, revjul, constants, createSwissShapedFacade, loadNative };
