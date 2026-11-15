// Original VirgoStars test source, CPAL-1.0; see repository LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {auditHipparcosTargets,parseHip2Uncertainties} from '../scripts/audit-hipparcos-2007.mjs';

const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
function line(hip, {sn=5,nc=1,plx=10,plxError=1}={}) {
 const bytes=Array(276).fill(' ');
 const put=(start,size,v)=>{if(v===null)return;const str=String(v).padStart(size);assert.ok(str.length<=size);for(let i=0;i<size;i++)bytes[start+i]=str[i];};
 put(0,6,hip);put(7,3,sn);put(13,1,nc);put(15,13,'1.3');put(29,13,'0.5');
 put(43,7,plx);put(51,8,'2');put(60,8,'-3');
 put(69,6,'0.7');put(76,6,'0.8');put(83,6,plxError);put(90,6,'1.2');put(97,6,'1.5');
 return bytes.join('');
}
const manifest=(ids=[123])=>Buffer.from(JSON.stringify({schemaVersion:1,stars:ids.map((hip,i)=>({
  name:'Target'+i,hip,simbadUrl:'https://simbad.cds.unistra.fr/simbad/sim-id?Ident=HIP+'+hip,
}))}));
const audit=(raw,targets=manifest(),extra={})=>{
 const bytes=Buffer.from(raw);
 return auditHipparcosTargets(bytes,targets,{hip2Sha256:hash(bytes),requireFullCatalog:false,...extra});
};
test('original Hipparcos uncertainty column offsets and units parsed without substitution',()=>{
 const row=line(123,{plxError:2});
 assert.deepEqual(parseHip2Uncertainties(row),{
   raMas:.7,decMas:.8,parallaxMas:2,properMotionRaMasPerYear:1.2,properMotionDecMasPerYear:1.5,
 });
 const report=audit(row+'\n');
 assert.equal(report.source.rawRows,1);
 assert.equal(report.stars[0].status,'ASTROMETRY_PRECHECK_PASSED');
 assert.equal(report.stars[0].parallaxSignalToNoise,5);
 assert.equal(report.stars[0].catalogReady,false);
 assert.ok(report.stars[0].blockers.some(x=>x.includes('barycentric radial velocity')));
 assert.equal(report.stars[0].parallaxMas,10);
});
test('weak, missing or negative parallax and complex astrometry are transparently flagged',()=>{
 assert.equal(audit(line(123,{plx:2,plxError:2})+'\n').stars[0].status,'LOW_PARALLAX_SIGNAL_TO_NOISE');
 assert.equal(audit(line(123,{plx:10,plxError:null})+'\n').stars[0].status,'PARALLAX_UNCERTAINTY_UNAVAILABLE');
 assert.equal(audit(line(123,{plx:-2,plxError:3})+'\n').stars[0].status,'NONPOSITIVE_PARALLAX');
 assert.equal(audit(line(123,{sn:15,nc:2})+'\n').stars[0].status,'COMPLEX_ASTROMETRY_REVIEW');
});
test('audit flags missing non-parallax formal uncertainties consistently with catalog export',()=>{
 const full=line(123,{plx:10,plxError:1});
 const missingRaError=full.slice(0,69)+' '.repeat(6)+full.slice(75);
 const report=audit(missingRaError+'\n');
 assert.equal(report.stars[0].status,'OTHER_FORMAL_UNCERTAINTIES_UNAVAILABLE');
 assert.equal(report.stars[0].uncertainties.raMas,null);
 assert.equal(report.stars[0].catalogReady,false);
 const weak=audit(line(123,{plx:4,plxError:1})+'\n');
 assert.equal(weak.stars[0].status,'LOW_PARALLAX_SIGNAL_TO_NOISE');
});

test('source integrity, duplicate identifiers and target coverage are auditable',()=>{
 const raw=Buffer.from(line(123)+'\n');
 const other=manifest([123,124]);
 const report=audit(raw,other);
 assert.deepEqual(report.counts,{ASTROMETRY_PRECHECK_PASSED:1,HIP_NOT_FOUND:1});
 assert.equal(report.stars[1].catalogReady,false);
 assert.throws(()=>auditHipparcosTargets(raw,other,{hip2Sha256:'0'.repeat(64),requireFullCatalog:false}),/SHA-256 mismatch/);
 assert.throws(()=>audit(line(123)+'\n'+line(123)+'\n'),/duplicate HIP/);
 assert.throws(()=>audit(line(123)+'\n',manifest(),{requireFullCatalog:true}),/117955/);
 assert.throws(()=>audit('bad\n'),/276 characters/);
});
const realPath=process.env.VIRGOSTARS_HIP2_PATH,realDigest=process.env.VIRGOSTARS_HIP2_SHA256;
test('optional manual audit against authentic external I/311 hip2.dat', {
 skip:!realPath || !realDigest ? 'Set VIRGOSTARS_HIP2_PATH and VIRGOSTARS_HIP2_SHA256 to use genuine local CDS data' : false,
},()=>{
 const source=readFileSync(realPath);
 const targets=readFileSync(new URL('../docs/data/fixed-stars-target-crossmatches.json',import.meta.url));
 const report=auditHipparcosTargets(source,targets,{hip2Sha256:realDigest});
 assert.equal(report.source.rawRows,117955);
 assert.equal(report.stars.length,81);
 assert.ok(report.stars.every(s=>s.status!=='HIP_NOT_FOUND'));
});
