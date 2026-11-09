import test from 'node:test';
import assert from 'node:assert/strict';
import { horizonsVectorUrl, parseHorizonsVector, compareGeometricVectors } from '../tools/horizons.mjs';

test('Horizons reference request uses geometric ICRF FRAME TDB in km/s',()=>{
  const x=horizonsVectorUrl({target:10,center:399,etTdbSeconds:0});
  const u=new URL(x.url),p=u.searchParams;
  assert.equal(p.get('EPHEM_TYPE'),'VECTORS');
  assert.equal(p.get('VEC_CORR'),'NONE');
  assert.equal(p.get('REF_PLANE'),'FRAME');
  assert.equal(p.get('REF_SYSTEM'),'ICRF');
  assert.equal(p.get('TIME_TYPE'),'TDB');
  assert.equal(p.get('CENTER'),"'500@399'");
  assert.equal(p.get('TLIST'),"'2451545.000000000000'");
  assert.equal(p.get('OUT_UNITS'),'KM-S');
});

test('parse Horizons synthetic one-row vector and reject no-ephemeris errors',()=>{
  const sample='Header\n$$SOE\n2451545.0, A.D. 2000-Jan-01 12:00, 1.0, -2.0, 3.0, 0.1, -0.2, 0.3,\n$$EOE\n';
  const result=parseHorizonsVector(sample);
  assert.deepEqual(result.positionKm,[1,-2,3]);
  assert.deepEqual(result.velocityKmPerSec,[0.1,-0.2,0.3]);
  assert.throws(()=>parseHorizonsVector('No data'),e=>e.code==='INVALID_HORIZONS');
});

test('residuals are signed vectors plus Euclidean norm and max component',()=>{
  const actual={frame:'J2000',corrections:'GEOMETRIC',positionKm:[3,5,0],velocityKmPerSec:[1,2,3]};
  const reference={positionKm:[0,1,0],velocityKmPerSec:[0,2,3]};
  const diff=compareGeometricVectors(actual,reference);
  assert.deepEqual(diff.positionErrorKm.signed,[3,4,0]);
  assert.equal(diff.positionErrorKm.norm,5);
  assert.equal(diff.velocityErrorKmPerSec.norm,1);
  assert.throws(()=>compareGeometricVectors({...actual,frame:'ECLIPTIC_OF_DATE'},reference));
});
