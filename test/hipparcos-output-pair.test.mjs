// Original VirgoStars test source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,readFileSync,writeFileSync,existsSync,rmSync,statSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {writeHipparcosOutputPair} from '../scripts/import-hipparcos-2007.mjs';

const fixture=()=>({
  catalog:{id:'synthetic-file-transaction-test',records:[{name:'Synthetic'}]},
  audit:{status:'TEST_ONLY_NOT_OBSERVATIONAL',stars:[{name:'Synthetic'}]},
});
const temp=t=>{
  const dir=mkdtempSync(join(tmpdir(),'hip2-export-pair-'));
  t.after(()=>rmSync(dir,{recursive:true,force:true}));
  return dir;
};

test('catalog and provenance audit are both created without overwriting files',t=>{
  const dir=temp(t),catalogPath=join(dir,'catalog.json'),auditPath=join(dir,'audit.json');
  const {catalog,audit}=fixture();
  const result=writeHipparcosOutputPair(catalogPath,auditPath,catalog,audit);
  assert.equal(result.catalogPath,catalogPath);
  assert.equal(result.auditPath,auditPath);
  assert.deepEqual(JSON.parse(readFileSync(catalogPath,'utf8')),catalog);
  assert.deepEqual(JSON.parse(readFileSync(auditPath,'utf8')),audit);
  assert.throws(()=>writeHipparcosOutputPair(catalogPath,auditPath,catalog,audit),/already exists/);
});

test('pre-existing audit cannot be overwritten and no catalog is left behind',t=>{
  const dir=temp(t),catalogPath=join(dir,'catalog.json'),auditPath=join(dir,'audit.json');
  writeFileSync(auditPath,'preserve pre-existing evidence');
  const {catalog,audit}=fixture();
  assert.throws(()=>writeHipparcosOutputPair(catalogPath,auditPath,catalog,audit),/already exists/);
  assert.equal(existsSync(catalogPath),false);
  assert.equal(readFileSync(auditPath,'utf8'),'preserve pre-existing evidence');
});

test('failed audit creation rolls back only the newly created catalog',t=>{
  const dir=temp(t),catalogPath=join(dir,'catalog.json');
  const auditPath=join(dir,'missing-audit-dir','audit.json');
  const {catalog,audit}=fixture();
  assert.throws(()=>writeHipparcosOutputPair(catalogPath,auditPath,catalog,audit),/ENOENT/);
  assert.equal(existsSync(catalogPath),false);
  assert.equal(existsSync(auditPath),false);
});

test('normalized alias paths and repository-local outputs fail closed',t=>{
  const dir=temp(t),{catalog,audit}=fixture();
  const path=join(dir,'same.json');
  assert.throws(()=>writeHipparcosOutputPair(path,join(dir,'.','same.json'),catalog,audit),/same file/);
  assert.equal(existsSync(path),false);
  const insideRepo=fileURLToPath(new URL('../test/should-not-be-committed.json',import.meta.url));
  assert.throws(()=>writeHipparcosOutputPair(insideRepo,join(dir,'audit.json'),catalog,audit),/outside the Git repository/);
  assert.equal(existsSync(insideRepo),false);
});
