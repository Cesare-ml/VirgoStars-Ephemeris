// Original VirgoStars test source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {buildNativeIcrsCandidate} from '../scripts/export-native-icrs-candidate.mjs';
const EPOCHS=[2448349.0625,2451545,2469807.5];
const catalog={id:'synthetic-source-only',records:[{
  name:'Synthetic',epochTdbJulianDay:2448349.0625,rightAscensionDeg:10,
  declinationDeg:20,properMotionRaCosDecMasPerYear:2,
  properMotionDecMasPerYear:3,parallaxMas:20,radialVelocityKmPerSecond:12
}]};
const reference={referenceModel:'ERFA_PMSAFE',coordinateFrame:'ICRS_ASTROMETRIC',
  epochConvention:'TDB_JULIAN_DAY',catalogId:catalog.id,
  inputCatalogSha256:'a'.repeat(64),epochsTdbJulianDay:EPOCHS,
  stars:[{name:'Synthetic',hip:123,samples:EPOCHS.map(tdbJulianDay=>({
    tdbJulianDay,rightAscensionDeg:55,declinationDeg:-55
  }))}]};
test('candidate exporter calls independent native callback at each epoch, never copies ERFA positions',()=>{
  const called=[];
  const output=buildNativeIcrsCandidate(catalog,reference,(entry,jd,hip)=>{
    called.push([entry.name,jd,hip]);
    return {tdbJulianDay:jd,rightAscensionDeg:10,declinationDeg:20};
  });
  assert.equal(called.length,3);
  assert.deepEqual(called.map(x=>x[1]),EPOCHS);
  assert.equal(output.stars[0].samples[0].rightAscensionDeg,10);
  assert.notEqual(output.stars[0].samples[0].rightAscensionDeg,reference.stars[0].samples[0].rightAscensionDeg);
  assert.equal(output.coordinateFrame,'ICRS_ASTROMETRIC');
  assert.equal(output.scientificallyValidated,false);
});
test('candidate rejects missing or nonfinite astrometry, mismatch, bad epoch',()=>{
  const noop=(_e,jd)=>({tdbJulianDay:jd,rightAscensionDeg:10,declinationDeg:20});
  assert.throws(()=>buildNativeIcrsCandidate(catalog,{...reference,catalogId:'other'},noop),/source catalog/);
  assert.throws(()=>buildNativeIcrsCandidate({...catalog,records:[{...catalog.records[0],parallaxMas:NaN}]},reference,noop),/Invalid astrometric/);
  assert.throws(()=>buildNativeIcrsCandidate(catalog,reference,(_e,jd)=>({...noop(null,jd),tdbJulianDay:jd+1})),/missing\/invalid/);
  assert.throws(()=>buildNativeIcrsCandidate(catalog,reference,(_e,jd)=>({...noop(null,jd),rightAscensionDeg:360})),/missing\/invalid/);
});
