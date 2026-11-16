// Types for the experimental VirgoStars-Ephemeris Node binding.

export interface CalcResult {
  /** [lon deg, lat deg, dist AU, dlon deg/day, dlat deg/day, ddist AU/day]; speeds are 0 unless SEFLG_SPEED. */
  xx: number[];
  /** Flags used; always includes SEFLG_JPLEPH (1). */
  rflag: number;
  /** TDB_INPUT | IERS_LEAP_SECONDS | DELTA_T_POLYNOMIAL:<model>:<seconds> | DELTA_T_TABLE:MEASURED|PREDICTED:<seconds>:<1-sigma|unknown> */
  timeSource: string;
  notes: string[];
}

export interface RevJul {
  year: number;
  month: number;
  day: number;
  hour: number;
}

export class Engine {
  constructor(de441Path: string);
  calcTdb(jdTdb: number, ipl: number, flags?: number): CalcResult;
  calcUt(jdUt: number, ipl: number, flags?: number): CalcResult;
  loadDeltaT(deltatDataPath: string, historicDeltatPath: string, deltatPredsPath?: string | null): void;
  loadChiron(spkPath: string): void;
  loadHipparcos(hip2Path: string, hips: number[], policy: 'strict' | 'traditional'): number;
  fixstarTdb(hip: number, rvKmS: number | null, jdTdb: number, flags?: number): CalcResult;
  /**
   * Horizon coordinates of an ecliptic (`equatorialInput` false) or equatorial-of-date
   * direction. Azimuth is from the SOUTH toward the west. Pressure 0 is estimated from `geoaltM`.
   */
  azaltUt(
    jdUt: number, equatorialInput: boolean, lonDegEast: number, latDeg: number, geoaltM: number,
    pressureHpa: number, temperatureC: number, x0: number, x1: number,
  ): AzaltResult;
  /** Inverse of `azaltUt` for a TRUE altitude: ecliptic lon/lat or RA/Dec of date. */
  azaltRevUt(
    jdUt: number, equatorialOutput: boolean, lonDegEast: number, latDeg: number,
    azimuthSouthDeg: number, trueAltitudeDeg: number,
  ): { x0Deg: number; x1Deg: number; timeSource: string };
  /**
   * First rise/set/transit after `jdUt` for bodies 0..9 and Chiron (when loaded); `rsmi` carries the
   * Swiss event and option bits. UT Julian date, or `null` when none within `searchDays` (default 2, max 31).
   */
  riseTransUt(
    jdUt: number, ipl: number, rsmi: number, lonDegEast: number, latDeg: number, geoaltM: number,
    pressureHpa: number, temperatureC: number, searchDays?: number,
  ): number | null;
}

export interface AzaltResult {
  azimuthSouthDeg: number;
  trueAltitudeDeg: number;
  apparentAltitudeDeg: number;
  timeSource: string;
}

export interface ExtendedRefractionResult {
  trueAltitudeDeg: number;
  apparentAltitudeDeg: number;
  /** Horizon dip of the observer's height, degrees (0 or negative). */
  dipDeg: number;
}

/** Plain-model refraction (`swe_refrac`): the altitude after refraction, degrees. */
export function refrac(altitudeDeg: number, pressureHpa: number, temperatureC: number, toApparent?: boolean): number;
/** Extended-model refraction (`swe_refrac_extended`, the model of azalt and rise/set). */
export function refracExtended(
  altitudeDeg: number, heightM: number, pressureHpa: number, temperatureC: number, lapseRate: number,
  toApparent?: boolean,
): ExtendedRefractionResult;

export function julday(year: number, month: number, day: number, hour: number, gregorian?: boolean): number;
export function revjul(jd: number, gregorian?: boolean): RevJul;
export interface HouseAnglesResult {
  ut1JulianDay: number;
  ttJulianDay: number;
  latitudeDeg: number;
  longitudeDegEast: number;
  armcDeg: number;
  trueObliquityDeg: number;
  ascendantDeg: number;
  midheavenDeg: number;
  timeSource: string;
}
export function houseAnglesUt(jdUt: number, latitudeDeg: number, longitudeDegEast: number): HouseAnglesResult;
export function createHouseAngleProvider(): Readonly<{
  calculateHouseAngles(input: {
    ut1JulianDay: number;
    ttJulianDay: number;
    latitudeDeg: number;
    longitudeDegEast: number;
  }): Readonly<{
    model: 'IAU2006_2000B_APPROXIMATE_SIDEREAL_TIME';
    ut1JulianDay: number;
    ttJulianDay: number;
    latitudeDeg: number;
    longitudeDegEast: number;
    armcDeg: number;
    ascendantDeg: number;
    midheavenDeg: number;
    trueObliquityDeg: number;
  }>;
}>;
export const constants: Readonly<Record<string, number>>;

export interface SwissShapedResult {
  longitude: number;
  latitude: number;
  distance: number;
  longitudeSpeed: number;
  latitudeSpeed: number;
  distanceSpeed: number;
  rflag: number;
  timeSource: string;
  notes: string[];
}

export interface SwissShapedError {
  error: string;
  code: string;
}

export function createSwissShapedFacade(
  engine: Engine | null,
  options?: { acceptSwiephFlag?: boolean },
): Readonly<Record<string, unknown>>;
export function loadNative(): unknown;
