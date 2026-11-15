// Original VirgoStars test source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {checkFixedStarCoverage} from '../scripts/audit-fixed-star-coverage.mjs';

const load=path=>JSON.parse(readFileSync(new URL(path,import.meta.url),'utf8'));
const review=load('../docs/data/fixed-star-identity-review.json');
const stars=load('../docs/data/fixed-stars-target-crossmatches.json');
const extended=load('../docs/data/extended-fixed-star-targets.json');

test('all manual-review aliases remain disjoint from actual stellar/extended catalogs',()=>{
  assert.equal(review.schemaVersion,1);
  assert.equal(review.reviewStatus,'NO_AUTOMATIC_RESOLUTION');
  assert.equal(review.scientificallyValidated,0);
  assert.equal(review.names.length,10);
  assert.equal(stars.stars.length,81);
  const names=new Set([...stars.stars,...extended.objects].map(x=>x.name.toLowerCase()));
  const reviewed=new Set();
  for(const item of review.names){
    assert.equal(item.status,'REQUIRES_LEGACY_SEMANTICS_REVIEW');
    assert.equal(item.catalogReady,false);
    assert.ok(item.reason.length>20);
    assert.ok(!names.has(item.name.toLowerCase()),'Reviewed labels cannot silently become active stars');
    assert.ok(!reviewed.has(item.name.toLowerCase()),'A review label must not be duplicated');
    reviewed.add(item.name.toLowerCase());
    assert.ok(Array.isArray(item.options)&&item.options.length>0);
    for(const suggestion of item.options){
      assert.ok(suggestion.hip===null || Number.isSafeInteger(suggestion.hip)&&suggestion.hip>0);
      assert.equal(new URL(suggestion.sourceUrl).protocol,'https:');
      assert.equal(typeof suggestion.designation,'string');
    }
  }
});
test('Tureis and Rigil record multiple distinct HIP candidates; Foramen has no asserted HIP',()=>{
 const byName=Object.fromEntries(review.names.map(x=>[x.name,x]));
 assert.deepEqual(byName.Tureis.options.map(x=>x.hip),[39757,45556]);
 assert.deepEqual(byName.Rigil.options.map(x=>x.hip),[71683,71681]);
 assert.deepEqual(byName.Foramen.options.map(x=>x.hip),[null]);
 assert.ok(byName['Gienah cygni'].options[0].hip!==byName['Gienah Ghurab'].options[0].hip);
});
test('the coverage auditor keeps reviewed but unresolved names outside the accepted catalog',()=>{
 const source=`const STARS_ASTRODIENST_TO_COW_ID = {"Mirach":8,"Tureis":127,"Praesepe":96};
var STARS_ASTRODIENST = [
'Mirach',
'Tureis',
'Praesepe',
];`;
 const result=checkFixedStarCoverage(source,stars,extended,review);
 assert.equal(result.candidatesIdentified,1);
 assert.equal(result.nonstellarIdentified,1);
 assert.equal(result.pendingIdentityReview,1);
 assert.equal(result.unclassifiedCount,0);
 assert.deepEqual(result.pendingIdentityReviewNames,['Tureis']);
 assert.equal(result.scientificallyValidated,0);
 const malformed=structuredClone(review);
 malformed.names[0].status='READY';
 assert.throws(()=>checkFixedStarCoverage(source,stars,extended,malformed),/manual-review identity|non-ready|review entries/i);
 const overlap=structuredClone(review);
 overlap.names[0].name='Mirach';
 assert.throws(()=>checkFixedStarCoverage(source,stars,extended,overlap),/overlapping manual-review target/);
});
