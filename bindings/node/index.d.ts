// Types for the experimental VirgoStars-Ephemeris Node binding.

export interface CalcResult {
  /** [lon deg, lat deg, dist AU, dlon deg/day, dlat deg/day, ddist AU/day]; speeds are 0 unless SEFLG_SPEED. */
  xx: number[];
  /** Flags used; always includes SEFLG_JPLEPH (1). */
  rflag: number;
  /** TDB_INPUT | IERS_LEAP_SECONDS | DELTA_T_POLYNOMIAL:<model>:<seconds> */
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
  loadHipparcos(hip2Path: string, hips: number[], policy: 'strict' | 'traditional'): number;
  fixstarTdb(hip: number, rvKmS: number | null, jdTdb: number, flags?: number): CalcResult;
}

export function julday(year: number, month: number, day: number, hour: number, gregorian?: boolean): number;
export function revjul(jd: number, gregorian?: boolean): RevJul;
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
