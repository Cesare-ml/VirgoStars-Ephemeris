// Original VirgoStars test source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {readLegacyStarNames,checkFixedStarCoverage} from '../scripts/audit-fixed-star-coverage.mjs';

const legacy=`const STARS_ASTRODIENST_TO_COW_ID = {
  "Spica": 737,
  "Sirius": 102,
  "Regulus": 402
};
var STARS_ASTRODIENST = [
  'Spica',
  'Sirius',
  'Regulus',
  // 'Unused', historical comment
  'Not mapped',
];`;

test('counts only actually mapped backend stars and keeps unmapped ones separate',()=>{
  const result=readLegacyStarNames(legacy);
  assert.equal(result.declared,4);
  assert.deepEqual(result.mapped,['Spica','Sirius','Regulus']);
  assert.deepEqual(result.unmapped,['Not mapped']);
});

test('compares caller-provided crossmatches without claiming numeric parity',()=>{
  const manifest={schemaVersion:1,stars:[
    {name:'Spica',hip:65474},{name:'Sirius',hip:32349},
    {name:'Not used',hip:999},
  ]};
  const report=checkFixedStarCoverage(legacy,manifest);
  assert.equal(report.legacyUsed,3);
  assert.equal(report.candidatesIdentified,2);
  assert.equal(report.missingCandidates,1);
  assert.deepEqual(report.missingNames,['Regulus']);
  assert.deepEqual(report.candidateNamesNotUsed,['Not used']);
  assert.equal(report.scientificallyValidated,0);
  assert.match(report.status,/NAMING_COVERAGE_ONLY/);
});

test('malformed or changed legacy source and duplicate candidates fail closed',()=>{
  assert.throws(()=>readLegacyStarNames('module.exports={};'),/contract changed/);
  assert.throws(()=>readLegacyStarNames(legacy.replace("'Spica',","'Spica',\n  'Spica',")),/Duplicate backend star/);
  assert.throws(()=>checkFixedStarCoverage(legacy,{schemaVersion:1,stars:[{name:'Spica'},{name:'spica'}]}),/Duplicate candidate/);
  assert.throws(()=>checkFixedStarCoverage(legacy,{schemaVersion:1,stars:[{name:'Spica',catalogReady:true}]}),/scientific acceptance/);
});

test('current curated crossmatch manifest stays explicitly non-calculable',()=>{
  const candidates=JSON.parse(readFileSync(new URL('../docs/data/fixed-stars-target-crossmatches.json',import.meta.url),'utf8'));
  const report=checkFixedStarCoverage(legacy,candidates);
  assert.equal(report.scientificallyValidated,0);
  assert.equal(report.candidatesIdentified,3);
  assert.ok(candidates.stars.every(s=>s.astrometryStatus==='NOT_YET_REVIEWED'));
});
