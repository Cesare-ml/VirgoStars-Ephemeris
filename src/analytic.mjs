/**
 * Independent analytic Lite prototypes with no external ephemeris files.
 * Data/formula sources:
 * https://ssd.jpl.nasa.gov/planets/approx_pos.html (Table 1: 1800–2050)
 * https://aa.usno.navy.mil/faq/sun_approx (approximate apparent Sun)
 * These are approximate, NOT Moshier, NOT a replacement for JPL SPK precision.
 */
import { SpkError } from './spk.mjs';
const DEG = Math.PI / 180;
const norm = a => ((a % 360) + 360) % 360;
const rad = a => a * DEG;
// Each record: six elements a,e,I,L,peri,node followed by their rates per Julian century.
const JPL_ELEMENTS_1800_2050 = Object.freeze({
  Mercury: [[0.38709927,0.20563593,7.00497902,252.25032350,77.45779628,48.33076593],[0.00000037,0.00001906,-0.00594749,149472.67411175,0.16047689,-0.12534081]],
  Venus: [[0.72333566,0.00677672,3.39467605,181.97909950,131.60246718,76.67984255],[0.00000390,-0.00004107,-0.00078890,58517.81538729,0.00268329,-0.27769418]],
  EM_BARY: [[1.00000261,0.01671123,-0.00001531,100.46457166,102.93768193,0.0],[0.00000562,-0.00004392,-0.01294668,35999.37244981,0.32327364,0.0]],
  Mars: [[1.52371034,0.09339410,1.84969142,-4.55343205,-23.94362959,49.55953891],[0.00001847,0.00007882,-0.00813131,19140.30268499,0.44441088,-0.29257343]],
  Jupiter: [[5.20288700,0.04838624,1.30439695,34.39644051,14.72847983,100.47390909],[-0.00011607,-0.00013253,-0.00183714,3034.74612775,0.21252668,0.20469106]],
  Saturn: [[9.53667594,0.05386179,2.48599187,49.95424423,92.59887831,113.66242448],[-0.00125060,-0.00050991,0.00193609,1222.49362201,-0.41897216,-0.28867794]],
  Uranus: [[19.18916464,0.04725744,0.77263783,313.23810451,170.95427630,74.01692503],[-0.00196176,-0.00004397,-0.00242939,428.48202785,0.40805281,0.04240589]],
  Neptune: [[30.06992276,0.00859048,1.77004347,-55.12002969,44.96476227,131.78422574],[0.00026291,0.00005105,0.00035372,218.45945325,-0.32241464,-0.00508664]],
});

/** Lightweight heliocentric J2000 mean-ecliptic AU state; JD must be TDB. */
export function approximateHeliocentricEcliptic(body, jdTdb) {
  if (!Number.isFinite(jdTdb) || jdTdb < 2378496.5 || jdTdb >= 2470172.5)
    throw new SpkError('OUT_OF_ANALYTIC_RANGE', 'JPL Table 1 approximation valid from 1800-01-01 to 2050-12-31');
  const entry = JPL_ELEMENTS_1800_2050[body];
  if (!entry) throw new SpkError('UNSUPPORTED_BODY', `Analytic Lite does not support ${body}`);
  const t = (jdTdb - 2451545) / 36525;
  const [a,e,I,L,peri,node] = entry[0].map((x,i) => x + t * entry[1][i]);
  const meanAnomaly = rad(((L - peri + 180) % 360 + 360) % 360 - 180);
  let E = meanAnomaly;
  let converged = false;
  for (let i=0; i<20; i++) {
    const delta = (meanAnomaly - (E - e * Math.sin(E))) / (1 - e * Math.cos(E));
    E += delta;
    if (Math.abs(delta) < 1e-13) { converged = true; break; }
  }
  if (!converged) throw new SpkError('NUMERIC_NON_CONVERGENCE', 'Kepler solver did not converge');
  const x = a * (Math.cos(E) - e), y = a * Math.sqrt(1 - e * e) * Math.sin(E);
  const w = rad(peri - node), omega = rad(node), incl = rad(I);
  const cw = Math.cos(w), sw = Math.sin(w), co = Math.cos(omega), so = Math.sin(omega), ci = Math.cos(incl), si = Math.sin(incl);
  const positionAu = Object.freeze([
    (cw * co - sw * so * ci) * x + (-sw * co - cw * so * ci) * y,
    (cw * so + sw * co * ci) * x + (-sw * so + cw * co * ci) * y,
    (sw * si) * x + (cw * si) * y,
  ]);
  return Object.freeze({body, jdTdb, positionAu, frame:'MEAN_ECLIPTIC_J2000', center:'SUN',
    provider:'JPL_KEPLER_TABLE1_ANALYTIC_LITE', validity:'1800-01-01..2050-12-31',
    accuracy:'LOWER_PRECISION_NOT_VALIDATED_LOCALLY', corrections:'GEOMETRIC',
    source:'https://ssd.jpl.nasa.gov/planets/approx_pos.html'});
}

/** Relative to the Earth–Moon *barycenter*, NOT Earth's physical center. */
export function approximateRelativeToEmBary(body, jdTdb) {
  if (body === 'EM_BARY') throw new SpkError('INVALID_ARGUMENT', 'Request a planet or Sun relative to EMB');
  const earth = approximateHeliocentricEcliptic('EM_BARY', jdTdb);
  const planet = body === 'Sun' ? {positionAu:[0,0,0]} : approximateHeliocentricEcliptic(body, jdTdb);
  return Object.freeze({body, jdTdb,
    positionAu:Object.freeze(planet.positionAu.map((n,i)=>n-earth.positionAu[i])),
    center:'EARTH_MOON_BARYCENTER_NOT_EARTH', frame:'MEAN_ECLIPTIC_J2000',
    provider:'JPL_KEPLER_TABLE1_ANALYTIC_LITE', validity:'1800-01-01..2050-12-31',
    accuracy:'LOWER_PRECISION_NOT_VALIDATED_LOCALLY', corrections:'GEOMETRIC'});
}

/** Public US Naval Observatory formula; JD is UT, approximate apparent Sun. */
export function approximateApparentSunUsno(jdUt) {
  if (!Number.isFinite(jdUt) || jdUt < 2378496.5 || jdUt >= 2524958.5)
    throw new SpkError('OUT_OF_ANALYTIC_RANGE', 'USNO solar approximation limited to 1800-2200 in this implementation');
  const d = jdUt - 2451545;
  const g = rad(norm(357.529 + 0.98560028 * d));
  const q = norm(280.459 + 0.98564736 * d);
  const longitudeDeg = norm(q + 1.915 * Math.sin(g) + 0.020 * Math.sin(2*g));
  const distanceAu = 1.00014 - 0.01671 * Math.cos(g) - 0.00014 * Math.cos(2*g);
  return Object.freeze({body:'Sun', jdUt, longitudeDeg, latitudeDeg:0, distanceAu,
    center:'EARTH_APPROX', frame:'ECLIPTIC_OF_DATE_APPROX', corrections:'APPARENT_APPROX',
    provider:'USNO_ANALYTIC_SOLAR_LITE', validity:'1800..2200',
    publishedTypicalAccuracy:'about 1 arcminute within two centuries of 2000 (USNO)',
    accuracy:'NOT_VALIDATED_LOCALLY',
    source:'https://aa.usno.navy.mil/faq/sun_approx'});
}
