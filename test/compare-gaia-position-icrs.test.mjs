// Original VirgoStars test source, CPAL-1.0; see repository LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {parseGaiaPositionTsv,compareGaiaPosition,runGaiaPositionComparison} from '../scripts/compare-gaia-position-icrs.mjs';

const digest=x=>createHash('sha256').update(x).digest('hex');
const gaiaId='3796442680948579328';
const header='Source\tRA_ICRS\tDE_ICRS\te_RA_ICRS\te_DE_ICRS\tPlx\te_Plx\tpmRA\tpmDE\tRV\te_RV\tRUWE';
const gaia=Buffer.from(`#Name: I/355/gaiadr3\n#Coosys\tH_2016.000:\tICRS\n${header}\n ${gaiaId}\t123\t60\t0.2\t0.3\t50\t1\t0\t0\t20\t0.1\t1.1\n`);
const star={name:'Fictional',aliases:[],epochTdbJulianDay:2448349.0625,rightAscensionDeg:123,declinationDeg:60,properMotionRaCosDecMasPerYear:0,properMotionDecMasPerYear:0,parallaxMas:50,radialVelocityKmPerSecond:20,visualMagnitude:3};
const sourceHash='a'.repeat(64),suppHash='b'.repeat(64);
const catalog={id:`hipparcos-2007-curated-${sourceHash.slice(0,12)}-${suppHash.slice(0,12)}-standard`,records:[star]};
const audit={hipparcos:{sha256:sourceHash},supplements:{sha256:suppHash},complexSolutionsAllowed:false,stars:[{hip:123,name:'Fictional'}]};
const ids={status:'OPERATOR_SNAPSHOTS_NOT_AUTHENTICATED_OR_CERTIFIED',scientificallyValidated:false,stars:[{hip:'123',gaiaSourceId:gaiaId}]};

test('Gaia ICRS epoch, exact string identifier and coordinate quality are enforced',()=>{
  const p=parseGaiaPositionTsv(gaia.toString(),gaiaId);
  assert.equal(p.rightAscensionDeg,123);assert.equal(p.raErrorMas,.2);assert.equal(p.ruwe,1.1);
  assert.throws(()=>parseGaiaPositionTsv(gaia.toString(),Number(gaiaId)),/source ID/);
  assert.throws(()=>parseGaiaPositionTsv(gaia.toString(),'123'),/exactly one/);
  assert.throws(()=>parseGaiaPositionTsv(gaia.toString().replace('H_2016.000','H_2015.000'),gaiaId),/schema/);
  assert.throws(()=>compareGaiaPosition({epochPolicy:'automatic'}),/TCB-to-TDB policy/);
});

test('pinned synthetic catalog and observed source flow through the Rust candidate',{
  skip:!process.env.VIRGOSTARS_EPHEMERIS_CLI,
},t=>{
  const dir=mkdtempSync(join(tmpdir(),'vs-gaia-position-'));t.after(()=>rmSync(dir,{recursive:true,force:true}));
  const data=[Buffer.from(JSON.stringify(catalog)),Buffer.from(JSON.stringify(audit)),Buffer.from(JSON.stringify(ids)),gaia];
  const keys=['catalog','catalog-audit','identity-audit','gaia-observed'];
  const paths=keys.map((key,i)=>{const p=join(dir,key+'.json');writeFileSync(p,data[i]);return p;});
  const args=keys.flatMap((key,i)=>[`--${key}=${paths[i]}`,`--${key}-sha256=${digest(data[i])}`]);
  args.push('--hip=123',`--executable=${process.env.VIRGOSTARS_EPHEMERIS_CLI}`,'--epoch-policy=TCB_JD_AS_TDB_JD_APPROXIMATION',`--out=${join(dir,'out.json')}`);
  const value=runGaiaPositionComparison(args);
  assert.equal(value.scientificallyValidated,false);assert.ok(value.separationArcsec<1e-8);
  const report=JSON.parse(readFileSync(join(dir,'out.json'),'utf8'));
  assert.equal(report.gaiaSourceId,gaiaId);assert.equal(report.status,'OBSERVED_POSITION_DIAGNOSTIC_NOT_CERTIFIED');
  assert.equal(report.inputHashes.gaiaObserved,digest(gaia));
  assert.equal(report.tdbMinusTcbSeconds,null);
  const iauArgs=args.map(arg=>arg.startsWith('--epoch-policy=')?'--epoch-policy=IAU2006_B3_TCB_TO_TDB':arg.startsWith('--out=')?`--out=${join(dir,'iau.json')}`:arg);
  const iau=runGaiaPositionComparison(iauArgs);assert.ok(iau.separationArcsec<1e-8);
  const iauReport=JSON.parse(readFileSync(join(dir,'iau.json'),'utf8'));
  assert.ok(Math.abs(iauReport.tdbMinusTcbSeconds+19.082696314659184)<1e-9);
  assert.ok(iauReport.candidateEpochTdbJulianDay<2457389.0);
  assert.throws(()=>runGaiaPositionComparison(args),/EEXIST/);
  writeFileSync(paths[3],Buffer.concat([gaia,Buffer.from(' ')]));
  assert.throws(()=>runGaiaPositionComparison(args),/SHA-256 mismatch/);
});
