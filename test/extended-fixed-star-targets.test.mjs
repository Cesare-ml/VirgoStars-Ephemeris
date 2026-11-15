// Original VirgoStars test source, CPAL-1.0; see repository LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const extended=JSON.parse(readFileSync(new URL('../docs/data/extended-fixed-star-targets.json',import.meta.url),'utf8'));
const stars=JSON.parse(readFileSync(new URL('../docs/data/fixed-stars-target-crossmatches.json',import.meta.url),'utf8'));

test('extended legacy labels are source backed and never given fabricated HIP or stellar states',()=>{
 assert.equal(extended.schemaVersion,1);
 assert.equal(extended.catalogReady,false);
 assert.deepEqual(extended.objects.map(x=>x.name).sort(),['Aculeus','Acumen','Facies','Praesepe','Spiculum'].sort());
 const hipNames=new Set(stars.stars.map(s=>s.name.toLowerCase()));
 for(const obj of extended.objects){
   assert.ok(!hipNames.has(obj.name.toLowerCase()),'Extended object should not also be a star');
   assert.ok(!Object.hasOwn(obj,'hip'),'Do not fabricate Hipparcos identifier');
   assert.ok(!Object.hasOwn(obj,'radialVelocityKmPerSecond'));
   assert.ok(!Object.hasOwn(obj,'parallaxMas'));
   assert.ok(!Object.hasOwn(obj,'rightAscensionDeg'));
   assert.ok(obj.status!=='READY');
   const uri=new URL(obj.sourceUrl);
   assert.equal(uri.protocol,'https:');
   assert.ok(['simbad.cds.unistra.fr','science.nasa.gov'].includes(uri.hostname));
 }
});
test('known clusters and the ambiguous Spiculum complex retain distinct object policies',()=>{
 const ids=Object.fromEntries(extended.objects.map(x=>[x.name,x]));
 assert.equal(ids.Aculeus.messier,'M6');
 assert.equal(ids.Acumen.messier,'M7');
 assert.equal(ids.Facies.messier,'M22');
 assert.equal(ids.Praesepe.messier,'M44');
 assert.equal(ids.Spiculum.astronomicalType,'AMBIGUOUS_NEBULAR_REGION');
 assert.match(ids.Spiculum.review,/M8, M20 and M21/);
 assert.equal(ids.Spiculum.status,'TRADITIONAL_LABEL_REQUIRES_SELECTION');
});
