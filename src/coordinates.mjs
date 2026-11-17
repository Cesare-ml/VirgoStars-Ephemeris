/**
 * Original mathematical projection of a geometric SPK ICRF/J2000 state.
 * Ecliptic is a simple J2000 rotation using IAU 2006 mean obliquity;
 * frame bias, date precession, nutation and light-time are NOT applied.
 */
export const ARCSEC_TO_RAD = Math.PI / (180 * 3600);
export const MEAN_OBLIQUITY_J2000_RAD = 84381.406 * ARCSEC_TO_RAD;
const DEGREES = 180 / Math.PI;
const SECONDS_PER_DAY = 86400;

export class CoordinateError extends Error {
  constructor(code, message) { super(message); this.name='CoordinateError'; this.code=code; }
}
function validate(v) {
  return Array.isArray(v) && v.length===3 && v.every(x=>typeof x==='number' && Number.isFinite(x));
}
const norm = x => ((x % 360) + 360) % 360;
function spherical(position, velocity) {
  const [x,y,z]=position, [vx,vy,vz]=velocity;
  const rho2=x*x+y*y, rho=Math.sqrt(rho2);
  const r2=rho2+z*z, r=Math.sqrt(r2);
  if(!Number.isFinite(r) || r===0) throw new CoordinateError('SINGULAR_POSITION','Zero or invalid distance');
  if(!Number.isFinite(rho) || rho < r*1e-14)
    throw new CoordinateError('SINGULAR_LONGITUDE','Longitude is undefined near pole');
  const longitudeDeg=norm(Math.atan2(y,x)*DEGREES);
  const latitudeDeg=Math.atan2(z,rho)*DEGREES;
  const longitudeSpeedDegPerDay=((x*vy-y*vx)/rho2)*DEGREES*SECONDS_PER_DAY;
  const radialXYSpeed=(x*vx+y*vy)/rho;
  const latitudeSpeedDegPerDay=((rho*vz-z*radialXYSpeed)/r2)*DEGREES*SECONDS_PER_DAY;
  const distanceSpeedKmPerSec=(x*vx+y*vy+z*vz)/r;
  return Object.freeze({longitudeDeg,latitudeDeg,distanceKm:r,longitudeSpeedDegPerDay,
    latitudeSpeedDegPerDay,distanceSpeedKmPerSec});
}
/** Equatorial and approximate mean-ecliptic J2000 coordinates from a geometric J2000 vector. */
export function projectGeometricJ2000(state) {
  if (!state || state.frame!=='J2000' || state.corrections!=='GEOMETRIC' ||
      !validate(state.positionKm) || !validate(state.velocityKmPerSec))
    throw new CoordinateError('INCOMPATIBLE_STATE','Expected finite Cartesian geometric J2000 km / km/s state');
  const [x,y,z]=state.positionKm, [vx,vy,vz]=state.velocityKmPerSec;
  const c=Math.cos(MEAN_OBLIQUITY_J2000_RAD),s=Math.sin(MEAN_OBLIQUITY_J2000_RAD);
  const ecliptic=spherical([x,c*y+s*z,c*z-s*y],[vx,c*vy+s*vz,c*vz-s*vy]);
  const equatorial=spherical(state.positionKm,state.velocityKmPerSec);
  return Object.freeze({target:state.target,center:state.center,etTdbSeconds:state.etTdbSeconds,
    corrections:'GEOMETRIC',frame:'J2000',
    equatorialJ2000:Object.freeze({...equatorial,rightAscensionDeg:equatorial.longitudeDeg,declinationDeg:equatorial.latitudeDeg}),
    approximateMeanEclipticJ2000:ecliptic,
    warnings:Object.freeze(['ECLIPTIC_J2000_APPROX_FRAME_BIAS_NOT_APPLIED',
      'NO_PRECESSION_NUTATION','NO_LIGHT_TIME_OR_ABERRATION']),
    model:'IAU2006_MEAN_OBLIQUITY_J2000_SIMPLE_ROTATION'});
}
