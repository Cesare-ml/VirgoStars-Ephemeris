// Original VirgoStars test source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {angularSeparationArcsec,compareIcrsStarEpochs,runIcrsEpochAudit} from '../scripts/compare-star-epoch-icrs.mjs';

const EPOCHS=[2448349.0625,2451545,2469807.5];
const makeReference=()=>({
  schemaVersion:1,status:'INDEPENDENT_ALGORITHM_REFERENCE_NOT_ENGINE_PARITY',
  coordinateFrame:'ICRS_ASTROMETRIC',epochConvention:'TDB_JULIAN_DAY',
  referenceModel:'ERFA_PMSAFE',catalogId:'synthetic-test-only',
  inputCatalogSha256:'a'.repeat(64),inputAuditSha256:'b'.repeat(64),scientificallyValidated:false,
  epochsTdbJulianDay:EPOCHS,
  stars:[{hip:123,name:'Synthetic',samples:EPOCHS.map(jd=>({
    tdbJulianDay:jd,rightAscensionDeg:359.99995,declinationDeg:12
  }))}],
});
const makeCandidate=(reference=makeReference())=>({
  schemaVersion:1,coordinateFrame:'ICRS_ASTROMETRIC',epochConvention:'TDB_JULIAN_DAY',
  catalogId:reference.catalogId,inputCatalogSha256:reference.inputCatalogSha256,
  candidateModel:'VIRGOSTARS_ICRS_ASTROMETRIC_EXPERIMENTAL',
  epochsTdbJulianDay:[...reference.epochsTdbJulianDay],
  stars:structuredClone(reference.stars),
});

test('great-circle angular separation handles RA crossing 360 degrees',()=>{
  const angle=angularSeparationArcsec(359.99995,12,0.00005,12);
  assert.ok(angle>0.35&&angle<0.36);
  assert.equal(angularSeparationArcsec(0,10,0,10),0);
  assert.throws(()=>angularSeparationArcsec(360,0,0,0),/ICRS RA/);
});

test('three synthetic epochs pass/fail an explicitly chosen angular tolerance',()=>{
  const ref=makeReference(),candidate=makeCandidate(ref);
  candidate.stars[0].samples[2].rightAscensionDeg=0.00005;
  const good=compareIcrsStarEpochs(ref,candidate,{maxArcsec:0.5});
  assert.equal(good.status,'NUMERIC_ICRS_PARITY_PASSED_NOT_PRODUCTION_CERTIFICATION');
  assert.equal(good.comparisons,3);
  assert.equal(good.failures,0);
  assert.equal(good.scientificallyValidated,false);
  const bad=compareIcrsStarEpochs(ref,candidate,{maxArcsec:0.1});
  assert.equal(bad.status,'NUMERIC_PARITY_FAILED');
  assert.equal(bad.failures,1);
});

test('reject apparent ecliptic bridge outputs or same reference engine',()=>{
  const ref=makeReference(),candidate=makeCandidate(ref);
  assert.throws(()=>compareIcrsStarEpochs(ref,{...candidate,coordinateFrame:'ECLIPTIC_APPARENT'},{maxArcsec:1}),/ICRS astrometric frame/);
  assert.throws(()=>compareIcrsStarEpochs(ref,{...candidate,candidateModel:'ERFA_PMSAFE'},{maxArcsec:1}),/distinct ICRS engine/);
  assert.throws(()=>compareIcrsStarEpochs(ref,{...candidate,candidateModel:'EXPERIMENTAL_ICRS_LINEAR_STAR'},{maxArcsec:1}),/apparent\/ecliptic bridge/);
  assert.throws(()=>compareIcrsStarEpochs(ref,candidate),/Explicit ICRS angular tolerance/);
  assert.throws(()=>compareIcrsStarEpochs(ref,{...candidate,inputCatalogSha256:'0'.repeat(64)},{maxArcsec:1}),/Catalog\/hash\/epochs differ/);
});

test('duplicate HIP IDs, missing sample epochs, short baselines are not accepted',()=>{
  const ref=makeReference(),candidate=makeCandidate(ref);
  const duplicate=structuredClone(candidate);duplicate.stars.push(structuredClone(candidate.stars[0]));
  assert.throws(()=>compareIcrsStarEpochs(ref,duplicate,{maxArcsec:1}),/duplicate\/invalid HIP/);
  const missing=structuredClone(candidate);
  missing.stars[0].samples[2].tdbJulianDay=missing.stars[0].samples[1].tdbJulianDay;
  assert.throws(()=>compareIcrsStarEpochs(ref,missing,{maxArcsec:1}),/invalid\/duplicate sample epoch/);
  const tooNear=structuredClone(ref);
  tooNear.epochsTdbJulianDay=[2448349.0625,2448350,2450000];
  assert.throws(()=>compareIcrsStarEpochs(tooNear,candidate,{maxArcsec:1}),/10\+ year baseline/);
});

test('CLI verifies both SHA-256 files and writes a non-certified report exclusively',t=>{
  const dir=mkdtempSync(join(tmpdir(),'virgostars-epoch-check-'));
  t.after(()=>rmSync(dir,{recursive:true,force:true}));
  const reference=makeReference(),candidate=makeCandidate(reference);
  const refPath=join(dir,'ref.json'),canPath=join(dir,'candidate.json'),out=join(dir,'report.json');
  writeFileSync(refPath,JSON.stringify(reference));writeFileSync(canPath,JSON.stringify(candidate));
  const digest=path=>createHash('sha256').update(readFileSync(path)).digest('hex');
  const args=['--reference='+refPath,'--reference-sha256='+digest(refPath),
    '--candidate='+canPath,'--candidate-sha256='+digest(canPath),
    '--max-arcsec=0.1','--out='+out];
  assert.throws(()=>runIcrsEpochAudit(args.map(x=>x.startsWith('--candidate-sha256=')?'--candidate-sha256='+'0'.repeat(64):x)),/SHA-256 mismatch/);
  const result=runIcrsEpochAudit(args);
  assert.equal(result.failures,0);
  assert.equal(JSON.parse(readFileSync(out,'utf8')).scientificallyValidated,false);
  assert.throws(()=>runIcrsEpochAudit(args),/EEXIST/);
});
