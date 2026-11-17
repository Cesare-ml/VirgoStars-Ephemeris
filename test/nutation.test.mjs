import test from 'node:test';
import assert from 'node:assert/strict';
import {
  iau2000bNutation,iau2000bNutationMatrix,iau2006With2000bTrueMatrix,
  projectGeometricTrueOfDate,NUTATION_MODEL_ID
} from '../src/nutation.mjs';
import {IAU2000B_TERMS} from '../src/data/iau2000b-terms.mjs';

const jd=2400000.5+53736;
test('exactly 77 full-series nutation terms, no approximated subset',()=>{
  assert.equal(IAU2000B_TERMS.length,77);
  assert.ok(IAU2000B_TERMS.every(r=>r.length===11&&r.every(Number.isFinite)));
});
test('IAU 2000B nutation matches published ERFA dPsi and dEpsilon reference (1e-13 rad)',()=>{
  const result=iau2000bNutation(jd);
  assert.ok(Math.abs(result.dPsiRad-(-0.9632552291148362783e-5))<1e-13);
  assert.ok(Math.abs(result.dEpsilonRad-0.4063197106621159367e-4)<1e-13);
  assert.equal(result.terms,77);
});
test('complete 2000B nutation matrix matches independent ERFA num00b 3x3',()=>{
  // Reference IAU 2000 mean obliquity from ERFA pn00b for this TT epoch.
  const matrix=iau2000bNutationMatrix(jd,0.4090791789404229916);
  const reference=[
    [0.9999999999536069682,0.8837746144871248011e-5,0.3831488838252202945e-5],
    [-0.8837590456632304720e-5,0.9999999991354692733,-0.4063198798559591654e-4],
    [-0.3831847930134941271e-5,0.4063195412258168380e-4,0.9999999991671806225]
  ];
  for(let i=0;i<3;i++)for(let j=0;j<3;j++)
    assert.ok(Math.abs(matrix[i][j]-reference[i][j])<1e-13);
});
test('true-of-date axis transform remains GEOMETRIC and rejects invented velocity/apparent',()=>{
  const state={target:10,center:399,frame:'J2000',corrections:'GEOMETRIC',positionKm:[1e6,2e5,3e5]};
  const result=projectGeometricTrueOfDate(state,jd);
  assert.equal(result.trueEclipticOfDate.frame,'TRUE_ECLIPTIC_OF_DATE');
  assert.equal(result.model,NUTATION_MODEL_ID);
  assert.equal(result.corrections,'GEOMETRIC');
  assert.equal(result.velocityKmPerSec,null);
  assert.ok(result.warnings.includes('NOT_APPARENT'));
  assert.ok(Math.abs(result.trueEclipticOfDate.distanceKm-Math.hypot(...state.positionKm))<1e-6);
  const matrix=iau2006With2000bTrueMatrix(jd);
  for(let i=0;i<3;i++) for(let j=0;j<3;j++){
    const dot=matrix[i].reduce((s,x,k)=>s+x*matrix[j][k],0);
    assert.ok(Math.abs(dot-(i===j?1:0))<1e-12);
  }
});
test('invalid TT, unsupported historic epochs, incompatible frame, pole and missing positions fail closed',()=>{
  assert.throws(()=>iau2000bNutation(Number.NaN),e=>e.code==='INVALID_JD_TT');
  assert.throws(()=>iau2000bNutation(2200000),e=>e.code==='UNSUPPORTED_EPOCH');
  assert.throws(()=>iau2000bNutationMatrix(jd,NaN),e=>e.code==='INVALID_OBLIQUITY');
  assert.throws(()=>projectGeometricTrueOfDate({frame:'J2000',corrections:'APPARENT',positionKm:[1,0,0]},jd),
    e=>e.code==='INCOMPATIBLE_STATE');
  assert.throws(()=>projectGeometricTrueOfDate({frame:'J2000',corrections:'GEOMETRIC',positionKm:[0,0,0]},jd),
    e=>e.code==='SINGULAR_POSITION');
  assert.throws(()=>projectGeometricTrueOfDate({frame:'J2000',corrections:'GEOMETRIC',positionKm:null},jd),
    e=>e.code==='INVALID_VECTOR');
});
