// Original VirgoStars test source, CPAL-1.0; see repository LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {runIcrsCandidate,exportHipparcosIcrsCandidate} from '../scripts/export-hipparcos-icrs-candidate.mjs';
import {compareIcrsStarEpochs} from '../scripts/compare-star-epoch-icrs.mjs';

const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const record={name:'Synthetic',aliases:[],epochTdbJulianDay:2448349.0625,rightAscensionDeg:123,declinationDeg:60,properMotionRaCosDecMasPerYear:10,properMotionDecMasPerYear:-20,parallaxMas:10,radialVelocityKmPerSecond:20,visualMagnitude:3};
const epochs=[2448349.0625,2451545,2469807.5];
const testCli=process.env.VIRGOSTARS_EPHEMERIS_CLI;

test('candidate rejects unpinned or mismatched catalog identities before executing',()=>{
  const catalog={id:'synthetic',records:[record]};
  const good={catalog,catalogSha256:'a'.repeat(64),audit:{stars:[{name:'Synthetic',hip:123}]},auditSha256:'b'.repeat(64),executablePath:join(tmpdir(),'not-executed')};
  assert.throws(()=>exportHipparcosIcrsCandidate({...good,catalogSha256:'bad'}),/Pinned/);
  assert.throws(()=>exportHipparcosIcrsCandidate({...good,audit:{stars:[{name:'Other',hip:123}]}}),/identities/);
  assert.throws(()=>exportHipparcosIcrsCandidate({...good,catalog:{...catalog,records:[{...record,epochTdbJulianDay:2451545}]}}),/epoch/);
});

test('pinned synthetic catalog flows through Rust and the independent ICRS comparator',{
  skip:!testCli,
},t=>{
  const dir=mkdtempSync(join(tmpdir(),'vs-icrs-'));t.after(()=>rmSync(dir,{recursive:true,force:true}));
  const catalogPath=join(dir,'catalog.json'),auditPath=join(dir,'audit.json'),outputPath=join(dir,'candidate.json');
  const catalogBytes=Buffer.from(JSON.stringify({id:'synthetic',records:[record]}));
  const auditBytes=Buffer.from(JSON.stringify({stars:[{name:'Synthetic',hip:123}]}));
  writeFileSync(catalogPath,catalogBytes);writeFileSync(auditPath,auditBytes);
  const args=[`--catalog=${catalogPath}`,`--catalog-sha256=${sha(catalogBytes)}`,`--audit=${auditPath}`,`--audit-sha256=${sha(auditBytes)}`,`--executable=${testCli}`,`--out=${outputPath}`];
  const summary=runIcrsCandidate(args);assert.equal(summary.stars,1);assert.equal(summary.scientificallyValidated,false);
  const candidate=JSON.parse(readFileSync(outputPath,'utf8'));
  assert.equal(candidate.stars[0].hip,123);assert.deepEqual(candidate.epochsTdbJulianDay,epochs);
  assert.ok(candidate.limitations.some(item=>item.includes('light-time')));
  // Generated independently with pyerfa 2.0.1.5 pmsafe. Synthetic data only;
  // pmsafe and the classical candidate use different light-time assumptions.
  const directions=[[123,60],[123.00004861095266,59.999951388966956],[123.0003263817466,59.999673614630574]];
  const reference={schemaVersion:1,status:'INDEPENDENT_ALGORITHM_REFERENCE_NOT_ENGINE_PARITY',referenceModel:'ERFA_PMSAFE',coordinateFrame:'ICRS_ASTROMETRIC',epochConvention:'TDB_JULIAN_DAY',catalogId:'synthetic',inputCatalogSha256:sha(catalogBytes),inputAuditSha256:sha(auditBytes),epochsTdbJulianDay:epochs,scientificallyValidated:false,stars:[{name:'Synthetic',hip:123,samples:epochs.map((jd,i)=>({tdbJulianDay:jd,rightAscensionDeg:directions[i][0],declinationDeg:directions[i][1]}))}]};
  const comparison=compareIcrsStarEpochs(reference,candidate,{maxArcsec:0.01});
  assert.equal(comparison.failures,0);assert.equal(comparison.comparisons,3);assert.equal(comparison.scientificallyValidated,false);
  assert.throws(()=>runIcrsCandidate(args),/EEXIST/);
  writeFileSync(auditPath,Buffer.concat([auditBytes,Buffer.from(' ')]));
  assert.throws(()=>runIcrsCandidate(args),/SHA-256 mismatch/);
});
