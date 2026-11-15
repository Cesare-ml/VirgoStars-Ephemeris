// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync,writeFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {loadExperimentalStarCatalogFile} from '../src/experimental-star-catalog-file.mjs';
import {createExperimentalSwissSubset} from '../src/experimental-swiss-subset.mjs';

const record={name:'Fictional',aliases:['Demo'],epochTdbJulianDay:2451545,rightAscensionDeg:90,declinationDeg:20,properMotionRaCosDecMasPerYear:2,properMotionDecMasPerYear:3,parallaxMas:10,radialVelocityKmPerSecond:5,visualMagnitude:4};
const digest=bytes=>createHash('sha256').update(bytes).digest('hex');
test('pinned catalog file is usable by the subset and identifies the exact bytes',t=>{
  const dir=mkdtempSync(join(tmpdir(),'vs-catalog-'));t.after(()=>rmSync(dir,{recursive:true,force:true}));
  const filePath=join(dir,'stars.json'),bytes=Buffer.from(JSON.stringify({id:'fictional-v1',records:[record]}));
  writeFileSync(filePath,bytes);
  const options={filePath,sha256:digest(bytes),source:'fictional test',license:'test only'};
  const loaded=loadExperimentalStarCatalogFile(options);
  assert.deepEqual(loaded.provenance,{sha256:options.sha256,source:options.source,license:options.license});
  const engine=createExperimentalSwissSubset({executablePath:'unused',kernelPath:'unused',convertUt1ToTtTdb:jd=>({ttJulianDay:jd,tdbJulianDay:jd}),catalog:loaded.catalog});
  assert.equal(engine.swe_fixstar_mag('demo').magnitude,4);
  assert.throws(()=>{loaded.catalog.records[0].visualMagnitude=9;},TypeError);
  assert.equal(engine.swe_fixstar_mag('demo').magnitude,4);
  writeFileSync(filePath,Buffer.concat([bytes,Buffer.from(' ')]));
  assert.throws(()=>loadExperimentalStarCatalogFile(options),/SHA-256 mismatch/);
  assert.throws(()=>loadExperimentalStarCatalogFile({...options,filePath:'relative.json'}),/absolute/);
  assert.throws(()=>loadExperimentalStarCatalogFile({...options,sha256:'BAD'}),/SHA-256/);
  assert.throws(()=>loadExperimentalStarCatalogFile({...options,license:''}),/source and license/);
});

test('invalid UTF-8, JSON, incomplete astrometry and duplicate aliases fail closed',t=>{
  const dir=mkdtempSync(join(tmpdir(),'vs-catalog-'));t.after(()=>rmSync(dir,{recursive:true,force:true}));
  const filePath=join(dir,'stars.json');
  const load=bytes=>{writeFileSync(filePath,bytes);return loadExperimentalStarCatalogFile({filePath,sha256:digest(bytes),source:'test',license:'test'});};
  assert.throws(()=>load(Buffer.from([0xff])),/encoded data|encoding/i);
  assert.throws(()=>load(Buffer.from('{')),SyntaxError);
  assert.throws(()=>load(Buffer.from(JSON.stringify({id:'x',records:[{...record,radialVelocityKmPerSecond:null}]}))),/Invalid or incomplete/);
  assert.throws(()=>load(Buffer.from(JSON.stringify({id:'x',records:[record,{...record,name:'Other'}]}))),/Duplicate/);
});
