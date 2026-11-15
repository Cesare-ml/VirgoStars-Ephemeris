// Original VirgoStars test source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync,writeFileSync,existsSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {gzipSync} from 'node:zlib';
import {OFFICIAL_HIP2_GZIP_URL,acquireHipparcos2007,decodeHipparcos2007Archive,runHipparcosDownload} from '../scripts/fetch-hipparcos-2007.mjs';

const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const row=(id)=>String(id).padStart(6)+Array(270).fill(' ').join('');
const mockArchive=(bytes)=>({
  ok:true,status:200,headers:new Headers({'content-length':String(bytes.length)}),
  async arrayBuffer(){return bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength);}
});
test('explicit source acquisition verifies gzip, SHA, full-width data and provenance without real network',async()=>{
  const raw=Buffer.from(row(17)+'\n'+row(18)+'\n');
  const archive=gzipSync(raw),requests=[];
  const got=await acquireHipparcos2007({requireFullCatalog:false,expectedArchiveSha256:sha(archive),fetchImpl:async(url,options)=>{
    requests.push({url,options});return mockArchive(archive);
  }});
  assert.equal(got.provenance.rows,2);
  assert.equal(got.provenance.rowLength,276);
  assert.equal(got.provenance.rawSha256,sha(raw));
  assert.equal(got.provenance.sourceArchiveSha256IndependentPin,true);
  assert.equal(requests.length,1);
  assert.equal(requests[0].url,OFFICIAL_HIP2_GZIP_URL);
  assert.equal(requests[0].options.redirect,'error');
});
test('offline acquisition refuses bad digest, unexpected content and short source',async()=>{
  const raw=Buffer.from(row(17)+'\n'),gz=gzipSync(raw);
  const fake=async()=>mockArchive(gz);
  await assert.rejects(()=>acquireHipparcos2007({fetchImpl:fake,requireFullCatalog:false,expectedArchiveSha256:'0'.repeat(64)}),/SHA-256/);
  await assert.rejects(()=>acquireHipparcos2007({fetchImpl:fake}),/expected 117955 rows/);
  await assert.rejects(()=>acquireHipparcos2007({fetchImpl:async()=>mockArchive(Buffer.from('notgzip')),requireFullCatalog:false}),/gzip archive/);
  await assert.rejects(()=>acquireHipparcos2007({fetchImpl:async()=>({ok:false,status:404}),requireFullCatalog:false}),/HTTP 404/);
});
test('acquisition rejects malformed 276-column records even with a valid gzip hash',async()=>{
  const archive=gzipSync(Buffer.from(row(17)+'\ninvalid\n'));
  await assert.rejects(()=>acquireHipparcos2007({requireFullCatalog:false,fetchImpl:async()=>mockArchive(archive)}),/expected 276 characters/);
});

test('CLI forbids raw third-party catalog output inside the source repository before network access',async()=>{
  await assert.rejects(
    ()=>runHipparcosDownload(['--out='+fileURLToPath(new URL('../data-local-hip2.dat',import.meta.url))]),
    /outside the Git repository/,
  );
});

test('offline gzip decoder preserves provenance and does not pretend a local source is an official HTTPS fetch',()=>{
  const raw=Buffer.from(row(17)+'\n'+row(18)+'\n');
  const archive=gzipSync(raw);
  const result=decodeHipparcos2007Archive(archive,{
    expectedArchiveSha256:sha(archive),requireFullCatalog:false,
  });
  assert.equal(result.provenance.acquisitionMode,'OPERATOR_SUPPLIED_LOCAL_ARCHIVE');
  assert.equal(result.provenance.sourceUrl,null);
  assert.equal(result.provenance.sourceArchiveSha256,sha(archive));
  assert.equal(result.provenance.rawSha256,sha(raw));
  assert.equal(result.provenance.rows,2);
  assert.throws(()=>decodeHipparcos2007Archive(archive,{expectedArchiveSha256:'0'.repeat(64),requireFullCatalog:false}),/SHA-256/);
});

test('offline CLI reads a real local gzip file, rejects incomplete data and does not contact the mirror',async t=>{
  const dir=mkdtempSync(join(tmpdir(),'hipparcos-offline-'));t.after(()=>rmSync(dir,{recursive:true,force:true}));
  const archive=join(dir,'hip2.dat.gz'),out=join(dir,'hip2.dat');
  writeFileSync(archive,gzipSync(Buffer.from(row(17)+'\n')));
  await assert.rejects(()=>runHipparcosDownload(['--archive='+archive,'--out='+out]),/expected 117955 rows/);
  assert.equal(existsSync(out),false);
  assert.equal(existsSync(out+'.source.json'),false);
});
