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
}

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
