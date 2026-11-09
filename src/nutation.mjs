/**
 * Original JavaScript implementation of the published 77-term IAU 2000B
 * nutation model, with ERFA scientific coefficients under BSD-3 notice.
 * IAU 2006 precession is combined with 2000B nutation as an explicit
 * APPROXIMATE 2006/2000B model (distinct from rigorously matched 06A).
 *
 * Independent numerical reference: ERFA nut00b and num00b test vectors.
 * Source tables under third_party/ERFA_LICENSE.txt; no ERFA C functions
 * have been translated or copied into this implementation.
 */
import { IAU2000B_TERMS } from './data/iau2000b-terms.mjs';
import { iau2006MeanObliquity, iau2006BiasPrecessionMatrix } from './precession.mjs';

const ARCSEC_TO_RAD=Math.PI/(180*3600);
const UNIT_TO_RAD=ARCSEC_TO_RAD/1e7;
const MILLIARCSEC_TO_RAD=ARCSEC_TO_RAD/1000;
const TURN_ARCSEC=1296000;
const TWO_PI=2*Math.PI;
const JD_TT_1900=2415020.0;
const JD_TT_2100=2488070.0;
export const NUTATION_MODEL_ID='IAU2000B_77_TERMS_IAU2006_PRECESSION_APPROX_COMBINATION';
export class NutationError extends Error {
  constructor(code,message){super(message);this.name='NutationError';this.code=code;}
}
function centuries(jdTt){
  if(typeof jdTt!=='number'||!Number.isFinite(jdTt))
    throw new NutationError('INVALID_JD_TT','Explicit finite Julian Date in TT required');
  if(jdTt<JD_TT_1900||jdTt>=JD_TT_2100)
    throw new NutationError('UNSUPPORTED_EPOCH','Experimental 2000B support limited to 1900-2100 TT');
  return (jdTt-2451545)/36525;
}
function angles(t){
  // Published linearized Simon et al. (1994) Delaunay arguments for 2000B.
  const initial=[485868.249036,1287104.79305,335779.526232,1072260.70369,450160.398036];
  const rates=[1717915923.2178,129596581.0481,1739527262.8478,1602961601.2090,-6962890.5431];
  return initial.map((x,i)=>((x+rates[i]*t)%TURN_ARCSEC)*ARCSEC_TO_RAD);
}
/** Nutation in ecliptic longitude dPsi and obliquity dEpsilon, TT. */
export function iau2000bNutation(jdTt){
  const t=centuries(jdTt);
  const a=angles(t);
  let longitude=0,obliquity=0;
  for(let i=IAU2000B_TERMS.length-1;i>=0;i--){
    const row=IAU2000B_TERMS[i];
    let angle=0;
    for(let k=0;k<5;k++)angle+=row[k]*a[k];
    angle%=TWO_PI;
    const sn=Math.sin(angle),cs=Math.cos(angle);
    longitude+=(row[5]+row[6]*t)*sn+row[7]*cs;
    obliquity+=(row[8]+row[9]*t)*cs+row[10]*sn;
  }
  const dPsiRad=longitude*UNIT_TO_RAD-0.135*MILLIARCSEC_TO_RAD;
  const dEpsilonRad=obliquity*UNIT_TO_RAD+0.388*MILLIARCSEC_TO_RAD;
  return Object.freeze({jdTt,model:'IAU_2000B_77_TERMS',terms:IAU2000B_TERMS.length,
    dPsiRad,dEpsilonRad,dPsiArcsec:dPsiRad/ARCSEC_TO_RAD,
    dEpsilonArcsec:dEpsilonRad/ARCSEC_TO_RAD,
    warnings:Object.freeze(['IAU2000B_APPROX_1MAS_ONLY_NEAR_MODERN_EPOCHS','FREE_CORE_NUTATION_NOT_INCLUDED'])});
}
const r1 = a => [[1,0,0],[0,Math.cos(a),Math.sin(a)],[0,-Math.sin(a),Math.cos(a)]];
const r3 = a => [[Math.cos(a),Math.sin(a),0],[-Math.sin(a),Math.cos(a),0],[0,0,1]];
const matMul=(a,b)=>a.map(r=>b[0].map((_,j)=>r.reduce((sum,v,k)=>sum+v*b[k][j],0)));
const freezeMatrix=m=>Object.freeze(m.map(r=>Object.freeze(r)));
const rotate=(m,v)=>m.map(r=>r.reduce((sum,x,j)=>sum+x*v[j],0));
function vector(v){
  if(!Array.isArray(v)||v.length!==3||v.some(n=>typeof n!=='number'||!Number.isFinite(n)))
    throw new NutationError('INVALID_VECTOR','Expected finite 3D Cartesian positionKm');
  return v;
}
function polar(v){
  const distanceKm=Math.hypot(...v),rxy=Math.hypot(v[0],v[1]);
  if(distanceKm===0)throw new NutationError('SINGULAR_POSITION','Cannot use zero position vector');
  if(rxy<distanceKm*1e-14)throw new NutationError('SINGULAR_LONGITUDE','Longitude undefined near pole');
  return Object.freeze({longitudeDeg:((Math.atan2(v[1],v[0])*180/Math.PI)%360+360)%360,
    latitudeDeg:Math.atan2(v[2],rxy)*180/Math.PI,distanceKm});
}
/**
 * Rotation from mean equator of date to true equator of date.
 * If meanObliquityRad is provided, the matrix can be compared with
 * IAU 2000-era numeric ERFA oracles; default uses IAU 2006 epsilon.
 */
export function iau2000bNutationMatrix(jdTt,meanObliquityRad=iau2006MeanObliquity(jdTt)){
  const nut=iau2000bNutation(jdTt);
  if(!Number.isFinite(meanObliquityRad))
    throw new NutationError('INVALID_OBLIQUITY','Finite mean obliquity in radians required');
  return freezeMatrix(matMul(matMul(r1(-meanObliquityRad-nut.dEpsilonRad),
    r3(-nut.dPsiRad)),r1(meanObliquityRad)));
}
/**
 * Original matrix composition: IAU2000B nutation * IAU2006 bias/precession.
 * Explicitly named APPROX due to 2000B/2006 cross-model conventions.
 */
export function iau2006With2000bTrueMatrix(jdTt){
  const matrix=matMul(iau2000bNutationMatrix(jdTt),iau2006BiasPrecessionMatrix(jdTt));
  return freezeMatrix(matrix);
}
/**
 * Geometric position expressed in the true equatorial/ecliptic axes of date.
 * A rotated GEOMETRIC vector is NOT an apparent planetary position.
 * Velocity is omitted, since date-frame derivatives were not implemented.
 */
export function projectGeometricTrueOfDate(state,jdTt){
  if(!state||state.frame!=='J2000'||state.corrections!=='GEOMETRIC')
    throw new NutationError('INCOMPATIBLE_STATE','Expected GEOMETRIC J2000 input');
  const input=vector(state.positionKm);
  const n=iau2000bNutation(jdTt);
  const m=iau2006With2000bTrueMatrix(jdTt);
  const equatorial=rotate(m,input);
  const eps=iau2006MeanObliquity(jdTt)+n.dEpsilonRad;
  const ecliptic=rotate(r1(eps),equatorial);
  return Object.freeze({target:state.target,center:state.center,jdTt,timeScale:'TT',
    corrections:'GEOMETRIC',model:NUTATION_MODEL_ID,
    trueEquatorialOfDate:Object.freeze({...polar(equatorial),
      frame:'TRUE_EQUATORIAL_OF_DATE',positionKm:Object.freeze(equatorial)}),
    trueEclipticOfDate:Object.freeze({...polar(ecliptic),
      frame:'TRUE_ECLIPTIC_OF_DATE',positionKm:Object.freeze(ecliptic)}),
    nutation:n,velocityKmPerSec:null,
    warnings:Object.freeze(['NOT_APPARENT','NO_LIGHT_TIME','NO_ABERRATION',
      'NO_GRAVITATIONAL_DEFLECTION','DATE_FRAME_VELOCITY_NOT_IMPLEMENTED',
      'IAU2000B_2006_APPROX_COMBINATION'])});
}
