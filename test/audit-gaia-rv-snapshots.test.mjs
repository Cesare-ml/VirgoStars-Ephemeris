// Original VirgoStars test source, CPAL-1.0; see repository LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {parseSimbadHipGaiaTsv,parseGaiaRvTsv,auditGaiaSnapshots,runGaiaSnapshotAudit} from '../scripts/audit-gaia-rv-snapshots.mjs';

const digest=bytes=>createHash('sha256').update(bytes).digest('hex');
const matches=Buffer.from('hip_id\tgaia_id\n"HIP 123"\t"Gaia DR3 3796442680948579328"\n"HIP 124"\t"Gaia DR3 804753180515722624"\n');
const fixture=(id,rv,err,ruwe)=>Buffer.from(`#Name: I/355/gaiadr3\nSource\tRV\te_RV\tRUWE\n \tkm/s\tkm/s\t\n---\t---\t---\t---\n ${id}\t${rv}\t${err}\t${ruwe}\n`);

test('identities preserve 64-bit Gaia IDs and reject ambiguity',()=>{
  const values=parseSimbadHipGaiaTsv(matches.toString());
  assert.equal(values[0].gaiaSourceId,'3796442680948579328');assert.equal(typeof values[0].gaiaSourceId,'string');
  assert.throws(()=>parseSimbadHipGaiaTsv(matches.toString()+'"HIP 125"\t"Gaia DR3 3796442680948579328"\n'),/duplicate/);
  assert.throws(()=>parseSimbadHipGaiaTsv('hip_id\tgaia_id\n"HIP 123"\t"Gaia DR3 123"'),/Invalid/);
});

test('padded IDs, unknown RV, malformed or mismatched source fail safely',()=>{
  assert.deepEqual(parseGaiaRvTsv(fixture('804753180515722624','','','3.335').toString(),'804753180515722624'),{rvKmPerSecond:null,rvErrorKmPerSecond:null,ruwe:3.335,status:'RV_MISSING'});
  assert.throws(()=>parseGaiaRvTsv(fixture('123','1','-1','1').toString(),'123'),/uncertainty/);
  assert.throws(()=>parseGaiaRvTsv(fixture('123','1','1','1').toString(),'124'),/matching/);
  assert.throws(()=>parseGaiaRvTsv('server returned an error','123'),/schema/);
});

test('local audit pins identities and records hashes without certifying values',t=>{
  const dir=mkdtempSync(join(tmpdir(),'vs-gaia-audit-'));t.after(()=>rmSync(dir,{recursive:true,force:true}));
  const cross=join(dir,'identities.tsv'),out=join(dir,'report.json');
  const row=fixture('3796442680948579328','4.38','0.30','2.811');
  writeFileSync(cross,matches);writeFileSync(join(dir,'123.tsv'),row);
  const args=[`--crossmatch=${cross}`,`--crossmatch-sha256=${digest(matches)}`,`--gaia-dir=${dir}`,`--out=${out}`];
  const result=runGaiaSnapshotAudit(args);
  assert.deepEqual(result.counts,{RV_PRESENT_REVIEW_REQUIRED:1,SNAPSHOT_NOT_SUPPLIED:1});
  const report=JSON.parse(readFileSync(out,'utf8'));
  assert.equal(report.scientificallyValidated,false);assert.equal(report.stars[0].snapshotSha256,digest(row));
  assert.equal(report.stars[0].ruwe,2.811);assert.equal(report.stars[1].snapshotSha256,null);
  assert.throws(()=>runGaiaSnapshotAudit(args),/EEXIST/);
  assert.throws(()=>auditGaiaSnapshots({crossmatchBytes:matches,crossmatchSha256:'0'.repeat(64),readSnapshot:()=>null}),/Pinned/);
  const repo=join(dir,'other-repository');mkdirSync(repo);mkdirSync(join(repo,'.git'));
  assert.throws(()=>runGaiaSnapshotAudit([...args.slice(0,-1),`--out=${join(repo,'report.json')}`]),/outside any Git worktree/);
});
