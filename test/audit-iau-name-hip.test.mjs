// Original VirgoStars test source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {parseIauNameHipCsv,compareIauNamedStarIdentities,KNOWN_IAU_SOURCE_BLOB_SHA} from '../scripts/audit-iau-name-hip.mjs';

const makeBytes=()=>Buffer.from([
 'Proper Names,NEC+,Designation,HIP,Bayer ID,Simbad spelling,Constellation',
 'Mirach,365,HR 337,5447,β And,Mirach,And',
 'Elnath,"1,826",HR 1791,25428,β Tau,Elnath,Tau',
 'Nashira,"8,256",HR 8278,106985,γ Cap,Nashira,Cap',
 'No-HIP,12,HR 1,,β CMa,No-HIP,CMa',
 ''].join('\n'),'utf8');
const gitSha=bytes=>createHash('sha1').update('blob '+bytes.length+'\0').update(bytes).digest('hex');

test('parse name/HIP fields only; quoted NEC identifiers do not shift columns',()=>{
 const b=makeBytes(), m=parseIauNameHipCsv(b);
 assert.equal(m.get('mirach'),5447);
 assert.equal(m.get('elnath'),25428);
 assert.equal(m.get('nashira'),106985);
 assert.equal(m.has('no-hip'),false);
});
test('exact IAU names and documented legacy aliases are compared without assuming scientific readiness',()=>{
 const bytes=makeBytes(),stars=[
 {name:'Mirach',hip:5447},
 {name:'El Nath',hip:25428,iauApprovedName:'Elnath',legacyNameReviewStatus:'SPELLING_OR_HISTORICAL_ALIAS_NEEDS_CONFIRMATION'},
 ];
 const r=compareIauNamedStarIdentities(stars,bytes,{expectedGitBlobSha:gitSha(bytes)});
 assert.equal(r.status,'IAU_NAME_IDENTITY_AGREEMENT_ONLY');
 assert.equal(r.matching,2);
 assert.equal(r.legacyAliases,1);
 assert.equal(r.scientificallyValidated,0);
 assert.throws(()=>compareIauNamedStarIdentities(stars,bytes,{expectedGitBlobSha:KNOWN_IAU_SOURCE_BLOB_SHA}),/SHA/);
});
test('mismatched and missing IAU entries are flagged, not promoted to valid 6D star measurements',()=>{
 const bytes=makeBytes();
 const r=compareIauNamedStarIdentities([{name:'Mirach',hip:999},{name:'No IAU record',hip:111}],bytes);
 assert.equal(r.status,'NEEDS_REVIEW');
 assert.deepEqual(r.nonMatching.map(x=>x.iauHip),[5447]);
 assert.equal(r.missing.length,1);
 assert.equal(r.scientificallyValidated,0);
 assert.throws(()=>parseIauNameHipCsv(Buffer.from('invalid')),/Expected locally supplied/);
});

const sourcePath=process.env.VIRGOSTARS_IAU_NAMES_CSV;
test('optional exact IAU mirror snapshot validates all 81 current backend star name/HIP pairs',{
 skip:sourcePath?false:'Set VIRGOSTARS_IAU_NAMES_CSV to the independently checked IAU-CSN CSV snapshot',
},()=>{
 const manifest=JSON.parse(readFileSync(new URL('../docs/data/fixed-stars-target-crossmatches.json',import.meta.url),'utf8'));
 const report=compareIauNamedStarIdentities(manifest.stars,readFileSync(sourcePath),{
   expectedGitBlobSha:KNOWN_IAU_SOURCE_BLOB_SHA,
 });
 assert.equal(report.count,81);
 assert.equal(report.matching,81);
 assert.equal(report.legacyAliases,5);
 assert.deepEqual(report.missing,[]);
 assert.deepEqual(report.nonMatching,[]);
});
