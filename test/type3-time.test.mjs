import test from 'node:test';
import assert from 'node:assert/strict';
import {
  inspectSpk, evaluateType3, stateAtTdbSeconds, approximateTdbFromUtc,
  approximateGeometricStateFromUtc,
} from '../src/index.mjs';

function type3Kernel() {
  const b=Buffer.alloc(4096);
  const i32=(v,p)=>b.writeInt32LE(v,p), f64=(v,p)=>b.writeDoubleLE(v,p);
  b.write('DAF/SPK ',0);i32(2,8);i32(6,12);i32(2,76);i32(2,80);i32(600,84);
  b.write('LTL-IEEE',88);
  f64(0,1024);f64(0,1032);f64(1,1040);
  f64(-10,1048);f64(10,1056);
  [10,0,1,3,385,402].forEach((v,j)=>i32(v,1064+j*4));
  b.write('TYPE3-SUN',2048);
  // MID,RADIUS, 6 series of degree 1, INIT,INTLEN,RSIZE,N
  [0,10,10,2,20,0,30,0,0.2,0,0.3,0,0.4,0,-10,20,14,1]
   .forEach((v,j)=>f64(v,(385+j-1)*8));
  return b;
}

test('type-3 SPK explicitly stores independent velocity coefficients',()=>{
  const b=type3Kernel(),seg=inspectSpk(b).segments[0];
  assert.equal(seg.type,3);
  const state=evaluateType3(b,seg,5,'LTL-IEEE');
  assert.deepEqual(state.positionKm,[11,20,30]);
  assert.deepEqual(state.velocityKmPerSec,[0.2,0.3,0.4]);
  assert.deepEqual(stateAtTdbSeconds(b,10,0,5).positionKm,[11,20,30]);
  assert.deepEqual(stateAtTdbSeconds(b,10,0,5).velocityKmPerSec,[0.2,0.3,0.4]);
});

test('type-3 segment rejects malformed record directory',()=>{
  const b=type3Kernel(),seg=inspectSpk(b).segments[0];
  b.writeDoubleLE(15,(401-1)*8);
  assert.throws(()=>evaluateType3(b,seg,0,'LTL-IEEE'),e=>e.code==='INVALID_SPK');
});

test('UTC->TT->approx TDB has explicit 1972/2017 leap-second offset',()=>{
  const before=approximateTdbFromUtc('2016-12-31T23:59:59Z');
  const after=approximateTdbFromUtc('2017-01-01T00:00:00Z');
  assert.equal(before.taiMinusUtcSeconds,36);
  assert.equal(after.taiMinusUtcSeconds,37);
  assert.equal(after.ttMinusUtcSeconds,69.184);
  assert.equal(approximateTdbFromUtc('1972-01-01T00:00:00Z').taiMinusUtcSeconds,10);
  assert.ok(after.tdbSecondsFromJ2000Approx>before.tdbSecondsFromJ2000Approx);
  assert.ok(Math.abs(after.tdbMinusTtSeconds)<0.002);
});

test('J2000 TT start yields near-zero TDB seconds',()=>{
  const x=approximateTdbFromUtc('2000-01-01T11:58:55.816Z');
  assert.ok(Math.abs(x.tdbSecondsFromJ2000Approx)<0.002);
});

test('UTC validation never guesses leap seconds outside table scope',()=>{
  for(const value of ['1970-01-01T00:00:00Z','2027-01-01T00:00:00Z','2026-02-30T01:00:00Z','2016-12-31T23:59:60Z','2026-11-09T20:00:00+01:00']){
    assert.throws(()=>approximateTdbFromUtc(value),e=>['INVALID_UTC','UNSUPPORTED_TIME_RANGE'].includes(e.code));
  }
});

test('converted geometric SPK state includes provenance and warning',()=>{
  const b=type3Kernel();
  // At epoch J2000-TDB source segment is valid only in [-10,+10] seconds.
  const s=approximateGeometricStateFromUtc(b,10,0,'2000-01-01T11:58:55.816Z');
  assert.equal(s.frame,'J2000');
  assert.equal(s.corrections,'GEOMETRIC');
  assert.ok(s.warnings.includes('APPROX_TDB_CONVERSION'));
});
