// Original VirgoStars test source, CPAL-1.0; see repository LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {buildHipparcosCatalog,parseHip2Line,parseSupplementTsv,HIP_EPOCH_TDB_JD} from '../scripts/import-hipparcos-2007.mjs';
const hash=x=>createHash('sha256').update(x).digest('hex');
const line=(hip,ra,dec,plx,pmra,pmdec,sn=5,components=1)=>{
  const s=Array(276).fill(' ');
  const put=(start,length,value)=>{const str=String(value).padStart(length);assert.ok(str.length<=length);for(let i=0;i<length;i++)s[start+i]=str[i];};
  put(0,6,hip);put(7,3,sn);put(13,1,components);put(15,13,ra);put(29,13,dec);put(43,7,plx);put(51,8,pmra);put(60,8,pmdec);put(69,6,'0.7');put(76,6,'0.8');put(83,6,'1.0');put(90,6,'1.2');put(97,6,'1.5');
  return s.join('');
};
const header='HIP\tname\taliases\tvisualMagnitude\tvisualMagnitudeSource\tradialVelocityKmPerSecond\tradialVelocitySource\tradialVelocityReferenceFrame\tnameSource';
const urls='https://example.org/mag\t12.5\thttps://example.org/rv\tBARYCENTRIC\thttps://example.org/names';
const input=(row, supplement=`${header}\n123\tTestStar\tOther Name\t2.45\t${urls}\n`)=>{
 const h=Buffer.from(row+'\n'),s=Buffer.from(supplement);
 return [h,s,{hip2Sha256:hash(h),supplementsSha256:hash(s),requireFullCatalog:false}];
};
test('Hipparcos 2007 fixed-width radians and angular proper-motion columns are correctly read',()=>{
 const row=line(123,'3.1415926536','0.5235987756','12.5','-10.25','20.75');
 const parsed=parseHip2Line(row);
 assert.equal(parsed.pmRa,-10.25);assert.equal(parsed.pmDec,20.75);
 const {catalog,audit}=buildHipparcosCatalog(...input(row));
 const star=catalog.records[0];
 assert.equal(star.epochTdbJulianDay,HIP_EPOCH_TDB_JD);
 assert.ok(Math.abs(star.rightAscensionDeg-180)<1e-8);
 assert.ok(Math.abs(star.declinationDeg-30)<1e-8);
 assert.equal(star.properMotionRaCosDecMasPerYear,-10.25);
 assert.equal(star.radialVelocityKmPerSecond,12.5);
 assert.equal(star.visualMagnitude,2.45);
 assert.deepEqual(star.aliases,['Other Name','HIP 123']);
 assert.equal(audit.stars[0].hip,123);
});
test('missing radial velocity, V magnitude, names, and scientific provenance fail closed',()=>{
 const row=line(123,'1.3','0.5','10.0','2.0','3.0');
 assert.throws(()=>parseSupplementTsv(`${header}\n123\tStar\t\t\thttps://example.org/mag\t5\thttps://example.org/rv\tBARYCENTRIC\thttps://example.org/names`),/visual V magnitude/);
 assert.throws(()=>buildHipparcosCatalog(...input(row,`${header}\n123\tStar\t\t1\thttps://example.org/mag\t\thttps://example.org/rv\tBARYCENTRIC\thttps://example.org/names`)),/radial velocity/);
 assert.throws(()=>buildHipparcosCatalog(...input(row,`${header}\n123\tStar\t\t1\tunknown\t5\thttps://example.org/rv\tBARYCENTRIC\thttps://example.org/names`)),/HTTPS provenance/);
 assert.throws(()=>buildHipparcosCatalog(...input(line(123,'1.3','0.5','-2.0','2.0','3.0'))),/parallax/);
});
test('checks source hashes, source row counts, HIP uniqueness and alias collisions',()=>{
 const row=line(123,'1.3','0.5','10','2','3');
 const [h,s,opts]=input(row);
 assert.throws(()=>buildHipparcosCatalog(h,s,{...opts,hip2Sha256:'0'.repeat(64)}),/SHA-256 mismatch/);
 assert.throws(()=>buildHipparcosCatalog(h,s,{...opts,requireFullCatalog:true}),/Incomplete hip2.dat/);
 assert.throws(()=>buildHipparcosCatalog(...input(row+'\n'+row)),/duplicate HIP/);
 const dup=`${header}\n123\tStar\tAlias\t1\t${urls}\n124\tOther\tAlias\t1\t${urls}\n`;
 assert.throws(()=>parseSupplementTsv(dup),/Duplicate star name/);
});

test('complex Hipparcos astrometry is excluded unless explicitly opted in and marked uncertified',()=>{
 const row=line(123,'1.3','0.5','10','2','3',15,2);
 const [h,s,opts]=input(row);
 assert.throws(()=>buildHipparcosCatalog(h,s,opts),/non-standard/);
 const {audit}=buildHipparcosCatalog(h,s,{...opts,allowComplexSolutions:true});
 assert.equal(audit.stars[0].solutionCode,15);
 assert.equal(audit.stars[0].components,2);
 assert.equal(audit.stars[0].astrometryReviewStatus,'EXPERIMENTAL_COMPLEX_SOLUTION_NOT_CERTIFIED');
});
test('heliocentric radial velocities cannot be silently treated as barycentric',()=>{
 const row=line(123,'1.3','0.5','10','2','3');
 const supplement=`${header}\n123\tTestStar\tOther Name\t2.45\thttps://example.org/mag\t12.5\thttps://example.org/rv\tHELIOCENTRIC\thttps://example.org/names\n`;
 assert.throws(()=>buildHipparcosCatalog(...input(row,supplement)),/BARYCENTRIC/);
});

test('import gate rejects missing or unreliable formal uncertainties by default',()=>{
 const base=line(123,'1.3','0.5','10','2','3');
 const replace=(start,end,field)=>base.slice(0,start)+field.padStart(end-start)+base.slice(end);
 assert.throws(()=>buildHipparcosCatalog(...input(replace(83,89,'5.0'))),/low parallax signal-to-noise/);
 assert.throws(()=>buildHipparcosCatalog(...input(replace(83,89,''))),/missing formal astrometric uncertainty/);
 assert.throws(()=>buildHipparcosCatalog(...input(replace(69,75,''))),/missing formal astrometric uncertainty/);
 assert.throws(()=>buildHipparcosCatalog(...input(replace(83,89,'0.0'))),/strictly positive/);
 const [h,extra,opts]=input(replace(83,89,'5.0'));
 const {audit}=buildHipparcosCatalog(h,extra,{...opts,minParallaxSignalToNoise:1});
 assert.equal(audit.qualityPolicy.minParallaxSignalToNoise,1);
 assert.equal(audit.stars[0].formalUncertainties.parallaxMas,5);
 assert.equal(audit.stars[0].parallaxSignalToNoise,2);
 assert.throws(()=>buildHipparcosCatalog(h,extra,{...opts,minParallaxSignalToNoise:NaN}),/SNR threshold/);
});
