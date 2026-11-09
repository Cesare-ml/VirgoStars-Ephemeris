/**
 * Independent mathematical implementation of the published IAU 2006 P03
 * Fukushima-Williams bias+precession angles for a TT epoch.
 *
 * Scientific references:
 *   Capitaine & Wallace (2006), A&A 450, 855
 *   Wallace & Capitaine (2006), A&A 459, 981
 *   Hilton et al. (2006), Celestial Mechanics 94, 351 (angle polynomials)
 * Test oracle: ERFA public conformance examples for pmat06 and obl06.
 * No SOFA/ERFA source code is copied into this module.
 *
 * Deliberately omits nutation, apparent corrections, polar motion and dynamic
 * frame-rate terms. Neither this module nor its output is a natal-chart API.
 */
export const IAU2006_FRAME_MODEL = 'IAU_2006_P03_BIAS_PRECESSION_MEAN_OF_DATE';
const ARCSEC_TO_RAD = Math.PI / (180 * 3600);
const RAD_TO_DEGREES = 180 / Math.PI;
const JD_J2000 = 2451545;
const JULIAN_DAYS_PER_CENTURY = 36525;
const JD_TT_MIN = 2378496.5; // approximately 1800-01-01
const JD_TT_MAX = 2524593.5; // approximately 2200-01-01

export class PrecessionError extends Error {
  constructor(code, message) { super(message); this.name = 'PrecessionError'; this.code = code; }
}
function centuries(jdTt) {
  if (typeof jdTt !== 'number' || !Number.isFinite(jdTt))
    throw new PrecessionError('INVALID_JD_TT', 'A finite Julian date in Terrestrial Time (TT) is required');
  if (jdTt < JD_TT_MIN || jdTt >= JD_TT_MAX)
    throw new PrecessionError('UNSUPPORTED_EPOCH', 'Experimental IAU 2006 precession API is restricted to 1800–2200 TT until extended validation');
  return (jdTt - JD_J2000) / JULIAN_DAYS_PER_CENTURY;
}
function polynomial(t, coefficients) {
  let value = 0;
  for (let i = coefficients.length - 1; i >= 0; i--) value = value * t + coefficients[i];
  return value * ARCSEC_TO_RAD;
}
const GAMMA = [-0.052928, 10.556378, 0.4932044, -0.00031238, -0.000002788, 0.0000000260];
const PHI = [84381.412819, -46.811016, 0.0511268, 0.00053289, -0.000000440, -0.0000000176];
const PSI = [-0.041775, 5038.481484, 1.5584175, -0.00018522, -0.000026452, -0.0000000148];
const EPS = [84381.406, -46.836769, -0.0001831, 0.00200340, -0.000000576, -0.0000000434];

/** The IAU 2006 mean obliquity epsilon_A (radians), evaluated in TT. */
export function iau2006MeanObliquity(jdTt) {
  return polynomial(centuries(jdTt), EPS);
}
/** Four F-W angles in radians for GCRS/ICRF bias+precession. */
export function iau2006FukushimaWilliamsAngles(jdTt) {
  const t = centuries(jdTt);
  return Object.freeze({gammaBarRad: polynomial(t, GAMMA), phiBarRad: polynomial(t, PHI),
    psiBarRad: polynomial(t, PSI), meanObliquityRad: polynomial(t, EPS)});
}
// We use passive coordinate-rotation matrices (change of basis):
// R1(a): y'=cos(a)*y+sin(a)*z, z'=-sin(a)*y+cos(a)*z.
// R3(a): x'=cos(a)*x+sin(a)*y, y'=-sin(a)*x+cos(a)*y.
const r1 = a => [[1, 0, 0], [0, Math.cos(a), Math.sin(a)], [0, -Math.sin(a), Math.cos(a)]];
const r3 = a => [[Math.cos(a), Math.sin(a), 0], [-Math.sin(a), Math.cos(a), 0], [0, 0, 1]];
function multiply(a,b) {
  return a.map((row,i) => row.map((_,j) => row.reduce((s,_,k) => s + a[i][k] * b[k][j], 0)));
}
const freezeMatrix = m => Object.freeze(m.map(row => Object.freeze(row)));

/**
 * Bias+precession matrix mapping GCRS/ICRF coordinates to the mean equator
 * and equinox of date. This includes the IAU 2006 frame bias, and is NOT
 * simply identity at J2000.0.
 */
export function iau2006BiasPrecessionMatrix(jdTt) {
  const {gammaBarRad, phiBarRad, psiBarRad, meanObliquityRad} = iau2006FukushimaWilliamsAngles(jdTt);
  // Published Fukushima-Williams change-of-basis composition:
  // R1(-epsA) · R3(-psiBar) · R1(phiBar) · R3(gammaBar)
  return freezeMatrix(multiply(multiply(multiply(r1(-meanObliquityRad), r3(-psiBarRad)), r1(phiBarRad)), r3(gammaBarRad)));
}
function vector(v) {
  if (!Array.isArray(v) || v.length !== 3 || v.some(n => typeof n !== 'number' || !Number.isFinite(n)))
    throw new PrecessionError('INVALID_GEOMETRIC_STATE', 'Expected finite 3D Cartesian positionKm vector');
  return v;
}
const rotate = (m, v) => m.map(row => row.reduce((s, c, j) => s + c*v[j], 0));
function spherical(v) {
  const r = Math.hypot(...v), rho = Math.hypot(v[0], v[1]);
  if (!(r > 0)) throw new PrecessionError('SINGULAR_POSITION','Zero position vector');
  if (rho < 1e-14*r) throw new PrecessionError('SINGULAR_LONGITUDE','Longitude undefined at pole');
  const longitudeDeg = ((Math.atan2(v[1],v[0])*RAD_TO_DEGREES)%360+360)%360;
  return Object.freeze({longitudeDeg, latitudeDeg: Math.atan2(v[2],rho)*RAD_TO_DEGREES, distanceKm:r});
}
/**
 * Project geometric ICRF/J2000 position to mean equatorial and mean ecliptic
 * of a TT date. A pure spatial rotation of position is not an apparent
 * position and does not include velocity in the *moving* date frame.
 */
export function projectGeometricMeanOfDate(state, jdTt) {
  if (!state || state.frame !== 'J2000' || state.corrections !== 'GEOMETRIC')
    throw new PrecessionError('INCOMPATIBLE_STATE', 'Requires a geometric J2000 position state');
  const source = vector(state.positionKm);
  const matrix = iau2006BiasPrecessionMatrix(jdTt);
  const eq = rotate(matrix,source);
  const epsilon = iau2006MeanObliquity(jdTt);
  const ecl = rotate(r1(epsilon),eq);
  return Object.freeze({jdTt, timeScale:'TT', model:IAU2006_FRAME_MODEL,
    target:state.target, center:state.center, corrections:'GEOMETRIC',
    meanEquatorialOfDate:Object.freeze({...spherical(eq), frame:'MEAN_EQUATORIAL_OF_DATE', positionKm:Object.freeze(eq)}),
    meanEclipticOfDate:Object.freeze({...spherical(ecl), frame:'MEAN_ECLIPTIC_OF_DATE', positionKm:Object.freeze(ecl)}),
    velocityKmPerSec:null,
    warnings:Object.freeze(['NO_NUTATION','NO_LIGHT_TIME','NO_ABERRATION','NOT_APPARENT','DATE_FRAME_VELOCITY_NOT_IMPLEMENTED'])});
}
