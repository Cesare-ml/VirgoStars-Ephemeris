// Original VirgoStars test source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

const manifest=JSON.parse(readFileSync(new URL('../docs/data/fixed-stars-target-crossmatches.json',import.meta.url),'utf8'));

test('fixed-star target crossmatches are distinct sourced HIP IDs, never simulated measurements',()=>{
  assert.equal(manifest.schemaVersion,1);
  assert.equal(manifest.stars.length,81);
  const ids=new Set(),names=new Set();
  for(const item of manifest.stars){
    assert.ok(Number.isSafeInteger(item.hip)&&item.hip>0);
    assert.ok(!ids.has(item.hip),`duplicate HIP ${item.hip}`);ids.add(item.hip);
    assert.equal(typeof item.name,'string');
    assert.ok(!names.has(item.name.toLowerCase()));names.add(item.name.toLowerCase());
    const u=new URL(item.simbadUrl);
    assert.equal(u.protocol,'https:');
    assert.ok(u.hostname.startsWith('simbad.'));
    assert.equal(item.crossmatchStatus,'HIP_NAME_MATCH_REFERENCED');
    assert.equal(item.astrometryStatus,'NOT_YET_REVIEWED');
    assert.equal(item.visualMagnitude,null);
    assert.equal(item.barycentricRadialVelocityKmPerSecond,null);
    assert.ok(item.review.length>10);
  }
  assert.equal(manifest.stars.find(x=>x.name==='Sirius')?.hip,32349);
  assert.equal(manifest.stars.find(x=>x.name==='Spica')?.hip,65474);
  assert.equal(manifest.stars.find(x=>x.name==='Regulus')?.hip,49669);
  assert.equal(manifest.stars.find(x=>x.name==='Altair')?.hip,97649);
  assert.equal(manifest.stars.find(x=>x.name==='Deneb')?.hip,102098);
  assert.equal(manifest.stars.find(x=>x.name==='Castor')?.hip,36850);
  assert.equal(manifest.stars.find(x=>x.name==='Achernar')?.hip,7588);
  assert.equal(manifest.stars.find(x=>x.name==='Alpheratz')?.hip,677);
  assert.equal(manifest.stars.find(x=>x.name==='El Nath')?.hip,25428);
  assert.equal(manifest.stars.find(x=>x.name==='Mizar')?.hip,65378);
  assert.equal(manifest.stars.find(x=>x.name==='Acrux')?.hip,60718);
  assert.equal(manifest.stars.find(x=>x.name==='Alnair')?.hip,109268);
  assert.equal(manifest.stars.find(x=>x.name==='Mira')?.hip,10826);
  assert.equal(manifest.stars.find(x=>x.name==='Tania Australis')?.hip,50801);
  assert.equal(manifest.stars.find(x=>x.name==='Nashira')?.hip,106985);
  assert.equal(manifest.stars.find(x=>x.name==='Wezen')?.hip,34444);
  const aliases=manifest.stars.filter(x=>x.legacyNameReviewStatus==='SPELLING_OR_HISTORICAL_ALIAS_NEEDS_CONFIRMATION');
  assert.deepEqual(aliases.map(x=>x.name).sort(),['Adara','Agena','Etamin','Hasseleh'].sort());
});
