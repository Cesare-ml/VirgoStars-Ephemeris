// Types for the experimental VirgoStars-Ephemeris Node binding.
// Modification (2026-11-18): declare opt-in analytic deflection diagnostic.
// Modification (2026-11-18): declare opt-in historical Delta T scenarios.

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

/** Geometric ICRF Earth-centered state; no apparent or civil-time corrections. */
export interface GeometricStateResult {
  jdTdb: number;
  target: string;
  center: 'Earth';
  frame: 'ICRF';
  positionKm: number[];
  velocityKmPerDay: number[];
}

export interface LongTermExperimentalResult {
  /** Longitude, latitude, distance, and rates in degrees/AU per TDB day. */
  xx: number[];
  target: string;
  timeSource: 'TDB_INPUT';
  notes: string[];
}

export interface ExperimentalLongitudeRateResult {
  base: LongTermExperimentalResult;
  correctionDegPerTdbDay: number;
  candidateDegPerTdbDay: number;
  notes: string[];
}

export interface ExperimentalLatitudeRateResult {
  base: LongTermExperimentalResult;
  correctionDegPerTdbDay: number;
  candidateDegPerTdbDay: number;
  notes: string[];
}

export interface ExperimentalRadialRateResult {
  base: LongTermExperimentalResult;
  correctionAuPerTdbDay: number;
  candidateAuPerTdbDay: number;
  correctionApplied: boolean;
  notes: string[];
}

export interface ExperimentalCombinedRateResult {
  base: LongTermExperimentalResult;
  /** Original positions and separately labeled experimental rates. */
  candidateXx: number[];
  radialCorrectionApplied: boolean;
  notes: string[];
}

/** Explicit caller-provided UT1 and Delta T; uncertainty is not a position bound. */
export interface HistoricalThreeRateResult {
  baseXx: number[];
  candidateXx: number[];
  target: string;
  jdUt1: number;
  jdTt: number;
  jdTdb: number;
  deltaTSeconds: number;
  deltaTUncertaintySeconds: number;
  deltaTSource: string;
  radialCorrectionApplied: boolean;
  notes: string[];
}

/** Apparent geocentric ICRF vector, light-time range in km and derivative in km/TDB day. */
export interface ApparentIcrfResult extends GeometricStateResult {
  notes: string[];
}

export interface LongTermUt1Result {
  xx: number[];
  target: string;
  jdUt1: number;
  jdTt: number;
  jdTdb: number;
  deltaTSeconds: number;
  deltaTUncertaintySeconds: number;
  deltaTSource: string;
  notes: string[];
}

export interface DeltaTScenarioResult {
  deltaTSeconds: number;
  jdTt: number;
  jdTdb: number;
  /** Longitude and latitude in degrees, distance in AU, then rates per TDB day. */
  xx: number[];
}

/** Three explicit samples, not a position bound or a confidence interval. */
export interface HistoricalDeltaTScenariosResult {
  jdUt1: number;
  target: string;
  deltaTSeconds: number;
  deltaTUncertaintySeconds: number;
  deltaTSource: string;
  frame: 'POLE_BASED' | 'MIXED_DIRECT_OBLIQUITY';
  lower: DeltaTScenarioResult;
  nominal: DeltaTScenarioResult;
  upper: DeltaTScenarioResult;
  notes: string[];
}

export interface LongRangeDeltaTEstimate {
  decimalYear: number;
  seconds: number;
  model: 'ESPENAK_MEEUS_2006_UNCORRECTED';
  notes: string[];
}

/** Historical 2006 model estimate for astronomical decimal years 0 through 3000; uncertainty not supplied. */
export function experimentalEspenakMeeus2006DeltaT(decimalYear: number): LongRangeDeltaTEstimate;

export class Engine {
  constructor(de441Path: string);
  calcTdb(jdTdb: number, ipl: number, flags?: number): CalcResult;
  geometricStateTdb(jdTdb: number, ipl: number): GeometricStateResult;
  /** Experimental airless apparent vector with light-time, solar deflection and aberration. */
  experimentalApparentIcrfTdb(jdTdb: number, ipl: number): ApparentIcrfResult;
  /** Experimental analytic deflection velocity, unrefined emission time; floor guard can raise E_DEFLECTION_BOUNDARY. */
  experimentalAnalyticDeflectionIcrfTdb(jdTdb: number, ipl: number): ApparentIcrfResult;
  /** Experimental mean ecliptic of date; nutation is not included. */
  experimentalLongTermMeanEclipticTdb(jdTdb: number, ipl: number): LongTermExperimentalResult;
  /** Experimental apparent position; extrapolated nutation has no established long-range accuracy. */
  experimentalLongTermApparentTdb(jdTdb: number, ipl: number): LongTermExperimentalResult;
  /** Experimental mixed frame with directly fitted obliquity; no accuracy guarantee. */
  experimentalLongTermDirectObliquityTdb(jdTdb: number, ipl: number): LongTermExperimentalResult;
  /** Diagnostic only: candidate is not the derivative of the reported position. */
  experimentalLongitudeRateCandidateTdb(jdTdb: number, ipl: number): ExperimentalLongitudeRateResult;
  /** Diagnostic only: candidate is not the derivative of the reported position. */
  experimentalRadialRateCandidateTdb(jdTdb: number, ipl: number): ExperimentalRadialRateResult;
  /** Diagnostic only: candidate rates are not position derivatives. */
  experimentalCombinedRateCandidateTdb(jdTdb: number, ipl: number): ExperimentalCombinedRateResult;
  /** Diagnostic only: candidate is not the derivative of the reported position. */
  experimentalLatitudeRateCandidateTdb(jdTdb: number, ipl: number): ExperimentalLatitudeRateResult;
  /** Three candidate rates; base.xx retains the original position derivatives. */
  experimentalThreeRateCandidateTdb(jdTdb: number, ipl: number): ExperimentalCombinedRateResult;
  /** UT1 input and caller-supplied Delta T, uncertainty and source. */
  experimentalThreeRateCandidateUt1(jdUt1: number, ipl: number, deltaTSeconds: number, uncertaintySeconds: number, source: string): HistoricalThreeRateResult;
  /** UT1 and caller-asserted Delta T seconds, uncertainty seconds and source required. */
  experimentalLongTermApparentUt1(jdUt1: number, ipl: number, deltaTSeconds: number, uncertaintySeconds: number, source: string): LongTermUt1Result;
  /** Experimental mixed frame; UT1 and caller-asserted Delta T, uncertainty and source required. */
  experimentalLongTermDirectObliquityUt1(jdUt1: number, ipl: number, deltaTSeconds: number, uncertaintySeconds: number, source: string): LongTermUt1Result;
  /** Three fixed-UT1 samples at Delta T minus/plus the caller's perturbation. */
  experimentalLongTermApparentUt1Scenarios(jdUt1: number, ipl: number, deltaTSeconds: number, uncertaintySeconds: number, source: string): HistoricalDeltaTScenariosResult;
  /** Three samples using the experimental mixed-obliquity frame. */
  experimentalLongTermDirectObliquityUt1Scenarios(jdUt1: number, ipl: number, deltaTSeconds: number, uncertaintySeconds: number, source: string): HistoricalDeltaTScenariosResult;
  calcUt(jdUt: number, ipl: number, flags?: number): CalcResult;
  loadDeltaT(deltatDataPath: string, historicDeltatPath: string, deltatPredsPath?: string | null): void;
  loadChiron(spkPath: string): void;
  loadHipparcos(hip2Path: string, hips: number[], policy: 'strict' | 'traditional' | 'experimental'): number;
  /** House astronomy from a UT Julian date under this engine's time policy (measured DeltaT if loaded). */
  houseAnglesUt(jdUt: number, latitudeDeg: number, longitudeDegEast: number): HouseAnglesResult;
  /** First lunar eclipse with its maximum after (`backward`: before) `jdUt`; times are UT Julian dates. */
  lunarEclipseUt(jdUt: number, backward?: boolean): LunarEclipseResult;
  /** First solar eclipse (global circumstances); kind bits 4/8/16/32 plus 1 (central) or 2. */
  solarEclipseUt(jdUt: number, backward?: boolean): SolarEclipseResult;
  fixstarTdb(hip: number, rvKmS: number | null, jdTdb: number, flags?: number): CalcResult;
  fixstarUt(hip: number, rvKmS: number | null, jdUt: number, flags?: number): CalcResult;
  fixstarExplicitTdb(astrometry: ExplicitStarAstrometry, rvKmS: number | null, jdTdb: number, flags?: number): CalcResult;
  fixstarExplicitUt(astrometry: ExplicitStarAstrometry, rvKmS: number | null, jdUt: number, flags?: number): CalcResult;
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

export interface LunarEclipseResult {
  kind: number;
  maximum: number;
  penumbralBegin: number;
  penumbralEnd: number;
  partialBegin?: number | null;
  partialEnd?: number | null;
  totalBegin?: number | null;
  totalEnd?: number | null;
  umbralMagnitude: number;
  penumbralMagnitude: number;
}

export interface SolarEclipseResult {
  kind: number;
  maximum: number;
  partialBegin: number;
  partialEnd: number;
  totalBegin?: number | null;
  totalEnd?: number | null;
  centerBegin?: number | null;
  centerEnd?: number | null;
  /** Closest approach of the shadow axis to the geocentre, equatorial Earth radii. */
  gamma: number;
  /** Local apparent noon at the ground point under the axis; 0 if there is none. */
  noon: number;
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
  vertexDeg: number;
  equatorialAscendantDeg: number;
  kochCoAscendantDeg: number;
  munkaseyCoAscendantDeg: number;
  munkaseyPolarAscendantDeg: number;
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
/** Experimental long-term orientation. The caller supplies UT1 and TT. */
export function createExperimentalHistoricalHouseAngleProvider(): Readonly<{
  calculateHouseAngles(input: {ut1JulianDay: number; ttJulianDay: number;
    latitudeDeg: number; longitudeDegEast: number}): Readonly<{
      model: 'EXPERIMENTAL_LONG_TERM_SIDEREAL_TIME'; ut1JulianDay: number;
      ttJulianDay: number; latitudeDeg: number; longitudeDegEast: number;
      armcDeg: number; ascendantDeg: number; midheavenDeg: number; trueObliquityDeg: number;
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

/** Independently sourced, caller-owned metadata. Null RV explicitly assumes zero; null magnitude is unavailable. */
export interface ExplicitStarAstrometry {
  epochTdbJd: number;
  rightAscensionDeg: number;
  declinationDeg: number;
  properMotionRaCosDecMasPerYear: number;
  properMotionDecMasPerYear: number;
  parallaxMas: number;
}

export interface StarCatalogBase {
  name: string;
  /** Optional caller-owned result label; lookup still uses name and aliases. */
  displayName?: string;
  aliases: string[];
  /** Caller-asserted numerical selection tier, not independent scientific approval. */
  qualityTier?: 'strict' | 'traditional' | 'experimental';
  /** Caller-asserted differences for individual lookup names. */
  lookupWarnings?: StarLookupWarning[];
  radialVelocityKmS: number | null;
  magnitude: number | null;
  /** Optional caller-asserted provenance for an approximate integrated magnitude. */
  magnitudeInfo?: StarMagnitudeInfo;
}

export interface StarMagnitudeInfo {
  kind: 'INTEGRATED_APPARENT';
  band: 'VISUAL' | 'UNSPECIFIED';
  quality: 'APPROXIMATE';
  aperture: 'UNSPECIFIED';
  source: string;
  status?: 'CALLER_ASSERTED';
}

export type StarCatalogEntry = StarCatalogBase & (
  | { hip: number; sourceId?: never; astrometry?: never }
  | { hip?: never; sourceId: string; astrometry: ExplicitStarAstrometry }
);

export interface StarLookupWarning {
  name: string;
  code: 'SWISS_POSITION_MISMATCH' | 'SWISS_MAGNITUDE_MISMATCH' | 'SWISS_LOOKUP_UNAVAILABLE' | 'SWISS_RETURNED_DIFFERENT_NAME';
  detail: string;
  status?: 'CALLER_ASSERTED';
}

export type ReviewedStarCatalogEntry = StarCatalogEntry & {
  identitySource: string;
  displayNameSource?: string;
  qualityDecision: string;
  magnitudeSource?: string;
  magnitudeMissingReason?: string;
  radialVelocitySource?: string;
  radialVelocityMissingReason?: string;
  astrometrySource?: string;
};

export interface StarCatalogManifest {
  schema: 'virgostars-star-manifest/1' | 'virgostars-star-manifest/2';
  reviewer: string;
  astrometrySource: string;
  dataUseDecision: string;
  qualityPolicy: 'strict' | 'traditional' | 'experimental';
  stars: ReviewedStarCatalogEntry[];
}

/** Numerical policy eligibility only, not scientific or data-use approval. */
export interface HipparcosQualityResult {
  hip: number;
  solutionType: number;
  components: number;
  strictEligible: boolean;
  traditionalEligible: boolean;
  experimentalEligible: boolean;
}

export function inspectHipparcosQuality(hip2Path: string, hips: number[]): HipparcosQualityResult[];

/** Validates caller assertions; does not itself verify scientific identity or data rights. */
export function prepareStarCatalog(manifest: StarCatalogManifest): {
  stars: StarCatalogEntry[]; hips: number[]; qualityPolicy: 'strict' | 'traditional' | 'experimental';
  reviewer: string; astrometrySource: string; dataUseDecision: string;
};

/** Lookup coverage of caller assertions only; no HIP2, identity, rights or numerical approval. */
export function auditStarCatalogCoverage(manifest: StarCatalogManifest, requestedNames: string[]): {
  schema: 'virgostars-star-coverage/1'; status: 'REVIEW_REQUIRED';
  catalogStarCount: number; requestedCount: number; matchedCount: number; unmatchedCount: number;
  uniqueMatchedStarCount: number; missingMagnitudeCount: number; assumedZeroRadialVelocityCount: number;
  qualityPolicy: 'strict' | 'traditional' | 'experimental'; hip2Eligibility: 'NOT_CHECKED';
  sourceRights: 'NOT_VERIFIED'; scientificIdentity: 'NOT_VERIFIED';
  results: Array<
    | { requestedName: string; status: 'NOT_IN_MANIFEST' }
    | { requestedName: string; status: 'MATCHED_CALLER_ASSERTION'; canonicalName: string; hip: number;
        magnitude: 'MISSING' | 'VALUE_SUPPLIED'; radialVelocity: 'ASSUMED_ZERO' | 'VALUE_SUPPLIED' }
  >;
};

export interface HouseDiagnosticEvent {
  operation: 'swe_houses' | 'swe_houses_pos';
  code: string;
  system: 'V' | 'P' | 'K' | 'R' | 'O' | 'ALL' | 'OTHER';
  count: number;
}

/** Opt-in synchronous scope; diagnostics contain no dates, coordinates or messages. */
export function createHouseDiagnosticSession<TFacade extends Readonly<Record<string, unknown>>>(facade: TFacade): {
  facade: TFacade;
  run<T>(calculate: () => T): { result: T; diagnostics: {
    schema: 'virgostars-house-diagnostics/1'; events: HouseDiagnosticEvent[];
  } };
};

export function createSwissShapedFacade(
  engine: Engine | null,
  options?: {
    acceptSwiephFlag?: boolean;
    provenance?: boolean;
    /** Functions of astrological-functions that supply swe_houses / swe_houses_pos. */
    houseMethods?: { calculateHouseSetWithWarnings?: Function; calculatePlacidusHousePosition?: Function };
    starCatalog?: StarCatalogEntry[];
    /** Explicitly accepted legacy path; does not change the configured Engine data. */
    legacyEphePath?: string | null;
  },
): Readonly<Record<string, unknown>>;
export function loadNative(): unknown;
