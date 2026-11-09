/**
 * Experimental modern-epoch UTC -> approximate TDB converter.
 * Leap second sources: NIST and IERS Bulletin C 72 (2026-07-06).
 * This is not a full SPICE LSK, IAU SOFA, or high-precision TDB model.
 * UTC leap second instants (:60) and historic pre-1972 UTC are unsupported.
 */
import { SpkError, stateAtTdbSeconds } from './spk.mjs';

export const TIME_MODEL_ID = 'UTC_TAI_TT_TDB_APPROX_IERS_C72_2026';
const DAY_MS = 86_400_000;
const ONE_DAY_SECONDS = 86_400;
const J2000_JD = 2451545.0;
const VALID_START = '1972-01-01T00:00:00Z';
const VALID_END_EXCLUSIVE = '2027-01-01T00:00:00Z';
const LEAP_OFFSETS = Object.freeze([
  ['1972-01-01', 10], ['1972-07-01', 11], ['1973-01-01', 12],
  ['1974-01-01', 13], ['1975-01-01', 14], ['1976-01-01', 15],
  ['1977-01-01', 16], ['1978-01-01', 17], ['1979-01-01', 18],
  ['1980-01-01', 19], ['1981-07-01', 20], ['1982-07-01', 21],
  ['1983-07-01', 22], ['1985-07-01', 23], ['1988-01-01', 24],
  ['1990-01-01', 25], ['1991-01-01', 26], ['1992-07-01', 27],
  ['1993-07-01', 28], ['1994-07-01', 29], ['1996-01-01', 30],
  ['1997-07-01', 31], ['1999-01-01', 32], ['2006-01-01', 33],
  ['2009-01-01', 34], ['2012-07-01', 35], ['2015-07-01', 36],
  ['2017-01-01', 37],
]);

function parseCanonicalUtc(isoUtc) {
  if (typeof isoUtc !== 'string' ||
      !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,3})?Z$/.test(isoUtc))
    throw new SpkError('INVALID_UTC', 'Use explicit ISO 8601 UTC timestamp ending in Z; leap second :60 unsupported');
  const milliseconds = Date.parse(isoUtc);
  if (!Number.isFinite(milliseconds))
    throw new SpkError('INVALID_UTC', 'UTC timestamp is not a valid calendar time');
  // Date.parse normalizes some nonexistent dates (e.g. February 30); reject them.
  const [whole, fraction = ''] = isoUtc.slice(0, -1).split('.');
  if (new Date(milliseconds).toISOString() !== `${whole}.${fraction.padEnd(3, '0')}Z`)
    throw new SpkError('INVALID_UTC', 'UTC timestamp is not a valid calendar time');
  if (milliseconds < Date.parse(VALID_START) || milliseconds >= Date.parse(VALID_END_EXCLUSIVE))
    throw new SpkError('UNSUPPORTED_TIME_RANGE', 'This table supports 1972 through 2026; refresh leap-second data before extending');
  return milliseconds;
}

/**
 * Millisecond-resolution civil UTC conversion; returns approximate TDB seconds
 * past 2000-01-01 12:00:00 TDB. We do not claim sub-ms accuracy.
 */
export function approximateTdbFromUtc(isoUtc) {
  const utcMilliseconds = parseCanonicalUtc(isoUtc);
  const dayUtc = isoUtc.slice(0, 10);
  let taiMinusUtcSeconds = 10;
  for (const [date, offset] of LEAP_OFFSETS) {
    if (dayUtc >= date) taiMinusUtcSeconds = offset;
    else break;
  }
  const ttMinusUtcSeconds = taiMinusUtcSeconds + 32.184;
  const jdUtc = utcMilliseconds / DAY_MS + 2440587.5;
  const jdTt = jdUtc + ttMinusUtcSeconds / ONE_DAY_SECONDS;
  // Meeus-style low-order periodic approximation; not full IAU ephemeris time.
  const meanAnomalyRad = (357.53 + 0.9856003 * (jdTt - J2000_JD)) * Math.PI / 180;
  const tdbMinusTtSeconds = 0.001658 * Math.sin(meanAnomalyRad) +
    0.000014 * Math.sin(2 * meanAnomalyRad);
  return Object.freeze({
    isoUtc, jdUtc, jdTt, jdTdbApprox: jdTt + tdbMinusTtSeconds / ONE_DAY_SECONDS,
    tdbSecondsFromJ2000Approx: (jdTt - J2000_JD) * ONE_DAY_SECONDS + tdbMinusTtSeconds,
    taiMinusUtcSeconds, ttMinusUtcSeconds, tdbMinusTtSeconds,
    model: TIME_MODEL_ID,
    warnings: Object.freeze(['APPROX_TDB_CONVERSION','UTC_LEAP_SECOND_INSTANTS_NOT_SUPPORTED']),
  });
}

/** Geometric J2000 SPK with an explicit approximate civil-time conversion. */
export function approximateGeometricStateFromUtc(buffer, target, center, isoUtc) {
  const time = approximateTdbFromUtc(isoUtc);
  const state = stateAtTdbSeconds(buffer,target,center,time.tdbSecondsFromJ2000Approx);
  return Object.freeze({ ...state, time, warnings:time.warnings });
}
