import test from 'node:test';
import assert from 'node:assert/strict';
import {projectGeometricJ2000,MEAN_OBLIQUITY_J2000_RAD} from '../src/coordinates.mjs';
import {solveOneWayLightTime,LIGHT_SPEED_KM_PER_SECOND} from '../src/light-time.mjs';
const state=(p,v=[0,0,0])=>({frame:'J2000',corrections:'GEOMETRIC',target:10,center:399,etTdbSeconds:0,positionKm:p,velocityKmPerSec:v});
test('ecliptic J2000 rotated from equatorial J2000; equator reference checks',()=>{
  const result=projectGeometricJ2000(state([1,0,0]));
  assert.equal(result.equatorialJ2000.rightAscensionDeg,0);
  assert.equal(result.approximateMeanEclipticJ2000.longitudeDeg,0);
  assert.equal(result.approximateMeanEclipticJ2000.latitudeDeg,0);
  const eps=MEAN_OBLIQUITY_J2000_RAD;
  const yAxis=projectGeometricJ2000(state([0,Math.cos(eps),Math.sin(eps)]));
  assert.ok(Math.abs(yAxis.approximateMeanEclipticJ2000.longitudeDeg-90)<1e-12);
  assert.ok(Math.abs(yAxis.approximateMeanEclipticJ2000.latitudeDeg)<1e-12);
});
test('J2000 velocity projects to ecliptic longitude rate and range speed',()=>{
  const r=projectGeometricJ2000(state([1000,0,0],[0,1,0])).approximateMeanEclipticJ2000;
  const expected=Math.cos(MEAN_OBLIQUITY_J2000_RAD)/1000*180/Math.PI*86400;
  assert.ok(Math.abs(r.longitudeSpeedDegPerDay-expected)<1e-10);
  assert.equal(r.distanceKm,1000);
  assert.equal(r.distanceSpeedKmPerSec,0);
});
test('J2000 projection rejects invalid/incompatible frame and singular poles',()=>{
  assert.throws(()=>projectGeometricJ2000({...state([1,0,0]),corrections:'APPARENT'}),e=>e.code==='INCOMPATIBLE_STATE');
  assert.throws(()=>projectGeometricJ2000(state([0,0,0])),e=>e.code==='SINGULAR_POSITION');
  assert.throws(()=>projectGeometricJ2000(state([0,0,1])),e=>e.code==='SINGULAR_LONGITUDE');
});
test('iterative light-time matches analytic moving target and uses observer reception epoch',()=>{
  const speed=12,C=LIGHT_SPEED_KM_PER_SECOND;
  const provider=(body,epoch)=>({frame:'J2000',corrections:'GEOMETRIC',center:0,positionKm:body===399?[0,0,0]:[C+speed*epoch,0,0]});
  const r=solveOneWayLightTime(provider,10,399,0);
  const expected=C/(C+speed);
  assert.ok(Math.abs(r.lightTimeSeconds-expected)<1e-10);
  assert.ok(Math.abs(r.positionKm[0]-C*expected)<1e-5);
  assert.ok(r.iterations>=2);
  assert.equal(r.corrections,'ONE_WAY_LIGHT_TIME_ONLY');
  assert.equal(r.velocityKmPerSec,null);
});
test('light-time enforces geometric barycentric frame and convergence bound',()=>{
  const provider=(body,et)=>({frame:'ECLIPTIC_OF_DATE',center:0,corrections:'GEOMETRIC',positionKm:[1,2,3]});
  assert.throws(()=>solveOneWayLightTime(provider,10,399,0),e=>e.code==='INCOMPATIBLE_FRAME');
  const valid=(body,et)=>({frame:'J2000',center:0,corrections:'GEOMETRIC',positionKm:[body===10?299792.458:0,0,0]});
  assert.throws(()=>solveOneWayLightTime(valid,10,399,0,{maxIterations:1}),e=>e.code==='NUMERIC_NON_CONVERGENCE');
});
