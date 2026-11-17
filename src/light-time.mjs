/** Geometric one-way light-time, independently implemented; no aberration or relativity. */
export const LIGHT_SPEED_KM_PER_SECOND = 299792.458;
export class LightTimeError extends Error {
  constructor(code,message) { super(message); this.name='LightTimeError'; this.code=code; }
}
const finiteVector=position=>Array.isArray(position) && position.length===3 &&
  position.every(x=>Number.isFinite(x));
function stateFromProvider(provider,body,epoch) {
  const state=provider(body,epoch);
  if (!state || !finiteVector(state.positionKm))
    throw new LightTimeError('INVALID_GEOMETRIC_STATE','Provider must return finite J2000 barycentric positionKm vector');
  if (state.frame!=='J2000'||state.corrections!=='GEOMETRIC'||state.center!==0)
    throw new LightTimeError('INCOMPATIBLE_FRAME','Provider must return geometric J2000 position relative to SSB (0)');
  return state.positionKm;
}
/**
 * Retarded target at emission time minus observer position at reception time.
 * provider(bodyNAIF, etTdbSeconds) MUST return geometric J2000 state w.r.t SSB.
 */
export function solveOneWayLightTime(provider,target,observer,etTdbSeconds,
  {maxIterations=12,toleranceSeconds=1e-6}={}) {
  if(typeof provider!=='function'||!Number.isInteger(target)||!Number.isInteger(observer)||
    target===observer||!Number.isFinite(etTdbSeconds)||
    !Number.isInteger(maxIterations)||maxIterations<1||maxIterations>100||
    !Number.isFinite(toleranceSeconds)||toleranceSeconds<=0)
    throw new LightTimeError('INVALID_ARGUMENT','Invalid provider, body, time or solver bounds');
  const origin=stateFromProvider(provider,observer,etTdbSeconds);
  let lightTime=0,position,iterations=0,converged=false;
  for(let i=0;i<maxIterations;i++) {
    const targetPosition=stateFromProvider(provider,target,etTdbSeconds-lightTime);
    position=targetPosition.map((v,j)=>v-origin[j]);
    const next=Math.hypot(...position)/LIGHT_SPEED_KM_PER_SECOND;
    if(!Number.isFinite(next)) throw new LightTimeError('INVALID_GEOMETRIC_STATE','Invalid light-time distance');
    iterations=i+1;
    if(Math.abs(next-lightTime)<=toleranceSeconds) { lightTime=next; converged=true; break; }
    lightTime=next;
  }
  if(!converged) throw new LightTimeError('NUMERIC_NON_CONVERGENCE','One-way light-time failed to converge');
  return Object.freeze({target,observer,frame:'J2000',corrections:'ONE_WAY_LIGHT_TIME_ONLY',
    positionKm:Object.freeze(position),velocityKmPerSec:null,
    receptionEtTdbSeconds:etTdbSeconds,emissionEtTdbSeconds:etTdbSeconds-lightTime,
    lightTimeSeconds:lightTime,iterations,
    warnings:Object.freeze(['NO_STELLAR_ABERRATION','NO_GRAVITATIONAL_DEFLECTION','NOT_APPARENT_ECLIPTIC_OF_DATE'])});
}
