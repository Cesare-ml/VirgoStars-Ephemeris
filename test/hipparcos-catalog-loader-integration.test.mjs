// Original VirgoStars test source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync,writeFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {buildHipparcosCatalog} from '../scripts/import-hipparcos-2007.mjs';
import {loadExperimentalStarCatalogFile} from '../src/experimental-star-catalog-file.mjs';
import {createExperimentalSwissSubset} from '../src/experimental-swiss-subset.mjs';

const digest=bytes=>createHash('sha256').update(bytes).digest('hex');
const fixedRow=()=>{
 const arr=Array(276).fill(' ');
 const fill=(offset,size,value)=>{
  const str=String(value).padStart(size);
  if(str.length>size)throw Error('Fixture too large');
  for(let i=0;i<size;i++)arr[offset+i]=str[i];
 };
 fill(0,6,123);fill(7,3,5);fill(13,1,1);fill(15,13,'1.3');
 fill(29,13,'0.5');fill(43,7,'10');fill(51,8,'2');fill(60,8,'3');
 return arr.join('')+'\n';
};
test('offline Hipparcos converter feeds pinned loader and magnitude API with the same alias',t=>{
 const hipBytes=Buffer.from(fixedRow());
 const supBytes=Buffer.from([
   'HIP','name','aliases','visualMagnitude','visualMagnitudeSource',
   'radialVelocityKmPerSecond','radialVelocitySource','radialVelocityReferenceFrame','nameSource'
 ].join('\t')+'\n'+[
   '123','Synthetic star','Demo name','2.45','https://example.org/photometry',
   '12.5','https://example.org/radial-velocity','BARYCENTRIC','https://example.org/identity'
 ].join('\t')+'\n');
 const {catalog,audit}=buildHipparcosCatalog(hipBytes,supBytes,{
   hip2Sha256:digest(hipBytes),supplementsSha256:digest(supBytes),requireFullCatalog:false,
 });
 assert.equal(audit.stars.length,1);
 const dir=mkdtempSync(join(tmpdir(),'hip2-pinned-'));t.after(()=>rmSync(dir,{recursive:true,force:true}));
 const filePath=join(dir,'converted.json');
 const bytes=Buffer.from(JSON.stringify(catalog)+'\n');writeFileSync(filePath,bytes);
 const loaded=loadExperimentalStarCatalogFile({
   filePath,sha256:digest(bytes),source:'Synthetic Hipparcos-format fixture only',license:'No external data: test-generated',
 });
 const swe=createExperimentalSwissSubset({
   executablePath:'not-called',kernelPath:'not-called',
   convertUt1ToTtTdb:jd=>({ttJulianDay:jd,tdbJulianDay:jd}),catalog:loaded.catalog,
 });
 assert.equal(swe.swe_fixstar_mag('Synthetic star').magnitude,2.45);
 assert.equal(swe.swe_fixstar_mag('HIP 123').magnitude,2.45);
 assert.equal(swe.swe_fixstar_mag('demo name').magnitude,2.45);
 assert.equal(swe.swe_fixstar_mag('HIP 123').catalogId,catalog.id);
 assert.throws(()=>loadExperimentalStarCatalogFile({...{
   filePath,sha256:'0'.repeat(64),source:'Synthetic fixture',license:'Test only',
 }}),/SHA-256 mismatch/);
});
