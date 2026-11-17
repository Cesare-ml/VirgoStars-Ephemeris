import test from 'node:test';
import assert from 'node:assert/strict';
import {
  iau2006MeanObliquity, iau2006BiasPrecessionMatrix,
  iau2006FukushimaWilliamsAngles, projectGeometricMeanOfDate
} from '../src/precession.mjs';

// Independent published ERFA conformance reference values from t_erfa_c.c.
test('IAU 2006 mean obliquity matches independent ERFA obl06 reference to 1e-14 rad', () => {
  const eps=iau2006MeanObliquity(2400000.5+54388.0);
  assert.ok(Math.abs(eps-0.4090749229387258204)<1e-14, String(eps));
});

test('IAU 2006 complete bias+precession matrix matches ERFA pmat06 9-element reference', () => {
  const expected=[
    [0.9999995505176007047, 0.8695404617348208406e-3, 0.3779735201865589104e-3],
    [-0.8695404723772031414e-3, 0.9999996219496027161, -0.1361752497080270143e-6],
    [-0.3779734957034089490e-3, -0.1924880847894457113e-6, 0.9999999285679971958]
  ];
  const actual=iau2006BiasPrecessionMatrix(2400000.5+50123.9999);
  for (let i=0;i<3;i++) for (let j=0;j<3;j++)
    assert.ok(Math.abs(actual[i][j]-expected[i][j])<1e-14, 'matrix[' + i + '][' + j + ']');
});

test('bias+precession matrix is orthogonal with determinant +1', () => {
  for(const jd of [2451545, 2415020.0, 2488070.0]) {
    const m=iau2006BiasPrecessionMatrix(jd);
    for(let i=0;i<3;i++) for(let j=0;j<3;j++) {
      const dot=m[i].reduce((sum,x,k)=>sum+x*m[j][k],0);
      assert.ok(Math.abs(dot-(i===j?1:0))<1e-13);
    }
    const det=m[0][0]*(m[1][1]*m[2][2]-m[1][2]*m[2][1])
      -m[0][1]*(m[1][0]*m[2][2]-m[1][2]*m[2][0])
      +m[0][2]*(m[1][0]*m[2][1]-m[1][1]*m[2][0]);
    assert.ok(Math.abs(det-1)<1e-13);
  }
});

test('a J2000 source vector changes in the mean-of-date frame in later year',()=> {
  const input={frame:'J2000',corrections:'GEOMETRIC',center:399,target:10,positionKm:[1000,200,400]};
  const now=projectGeometricMeanOfDate(input,2451545);
  const later=projectGeometricMeanOfDate(input,2451545+36525);
  assert.equal(now.meanEclipticOfDate.frame,'MEAN_ECLIPTIC_OF_DATE');
  assert.ok(Math.abs(now.meanEclipticOfDate.distanceKm-Math.hypot(...input.positionKm))<1e-9);
  assert.ok(Math.abs(later.meanEclipticOfDate.longitudeDeg-now.meanEclipticOfDate.longitudeDeg)>1);
  assert.equal(later.velocityKmPerSec,null);
  assert.ok(later.warnings.includes('NOT_APPARENT'));
});

test('unsupported times, frames, zero radius, and poles are rejected',()=>{
  assert.throws(()=>iau2006FukushimaWilliamsAngles(NaN),e=>e.code==='INVALID_JD_TT');
  assert.throws(()=>iau2006BiasPrecessionMatrix(2200000),e=>e.code==='UNSUPPORTED_EPOCH');
  assert.throws(()=>projectGeometricMeanOfDate({frame:'J2000',corrections:'APPARENT',positionKm:[1,2,3]},2451545),e=>e.code==='INCOMPATIBLE_STATE');
  assert.throws(()=>projectGeometricMeanOfDate({frame:'J2000',corrections:'GEOMETRIC',positionKm:[0,0,0]},2451545),e=>e.code==='SINGULAR_POSITION');
  const matrix=iau2006BiasPrecessionMatrix(2451545);
  const exactlyMeanPole=matrix.map((_,i)=>matrix[2][i]); // transpose(M) * [0,0,1]
  assert.throws(()=>projectGeometricMeanOfDate({frame:'J2000',corrections:'GEOMETRIC',positionKm:exactlyMeanPole},2451545),e=>e.code==='SINGULAR_LONGITUDE');
});
