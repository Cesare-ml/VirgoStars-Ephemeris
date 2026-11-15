// Original VirgoStars test source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync,writeFileSync,readFileSync,existsSync,rmSync,truncateSync,symlinkSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {gzipSync} from 'node:zlib';
import {OFFICIAL_HIP2_GZIP_URL,acquireHipparcos2007,decodeHipparcos2007Archive,readBoundedArchiveResponse,runHipparcosDownload,writeHipparcosSourcePair} from '../scripts/fetch-hipparcos-2007.mjs';

const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const row=(id)=>String(id).padStart(6)+Array(270).fill(' ').join('');
const mockArchive=(bytes,{reportedLength=bytes.length,chunkSize=7}={})=>({
  ok:true,status:200,
  headers:new Headers(reportedLength===null?{}:{'content-length':String(reportedLength)}),
  body:new ReadableStream({
    start(controller){
      for(let i=0;i<bytes.length;i+=chunkSize)
        controller.enqueue(Uint8Array.from(bytes.subarray(i,i+chunkSize)));
      controller.close();
    },
  }),
  async arrayBuffer(){throw Error('unbounded arrayBuffer must never be called');}
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

test('download caps streaming bytes even if Content-Length is omitted or forged',async()=>{
  const extra=Buffer.alloc(27,0x47);
  await assert.rejects(
    ()=>readBoundedArchiveResponse(mockArchive(extra,{reportedLength:null,chunkSize:4}),{maxBytes:16}),
    /compressed archive exceeds/,
  );
  await assert.rejects(
    ()=>readBoundedArchiveResponse(mockArchive(extra,{reportedLength:14,chunkSize:5}),{maxBytes:16}),
    /compressed archive exceeds/,
  );
  await assert.rejects(
    ()=>readBoundedArchiveResponse(mockArchive(extra,{reportedLength:300}),{maxBytes:16}),
    /compressed archive exceeds/,
  );
});

test('stream reader cancels on overflow rather than accepting further archive chunks',async()=>{
  let canceled=false,readCount=0;
  const body=new ReadableStream({
    pull(controller){
      readCount++;
      controller.enqueue(new Uint8Array(6));
    },
    cancel(){canceled=true;}
  },{highWaterMark:0});
  await assert.rejects(
    ()=>readBoundedArchiveResponse({headers:new Headers(),body},{maxBytes:8}),
    /compressed archive exceeds/,
  );
  assert.equal(canceled,true);
  assert.ok(readCount<=3);
});

test('stream content length must be well formed and match the received archive bytes',async()=>{
  const bytes=Buffer.from('0123456789');
  await assert.rejects(()=>readBoundedArchiveResponse(mockArchive(bytes,{reportedLength:9})),/Content-Length does not match/);
  await assert.rejects(()=>readBoundedArchiveResponse(mockArchive(bytes,{reportedLength:'x'})),/Invalid Hipparcos Content-Length/);
  await assert.rejects(()=>readBoundedArchiveResponse({headers:new Headers(),body:null}),/streaming body/);
  await assert.rejects(()=>readBoundedArchiveResponse(mockArchive(Buffer.from('x'))),/empty or truncated/);
});

test('bounded streamed archive preserves byte order when Content-Length is absent',async()=>{
  const raw=Buffer.from(row(17)+'\n');
  const archive=gzipSync(raw);
  const out=await readBoundedArchiveResponse(mockArchive(archive,{reportedLength:null,chunkSize:1}));
  assert.deepEqual(out,archive);
  const got=await acquireHipparcos2007({
    requireFullCatalog:false,
    fetchImpl:async()=>mockArchive(archive,{reportedLength:null,chunkSize:3}),
  });
  assert.equal(got.provenance.rows,1);
  assert.equal(got.provenance.sourceArchiveSha256,sha(archive));
});

test('offline archive loader refuses oversized local source before allocating the whole file',async t=>{
  const dir=mkdtempSync(join(tmpdir(),'hip2-gzip-cap-'));
  t.after(()=>rmSync(dir,{recursive:true,force:true}));
  const archive=join(dir,'oversized.gz'),out=join(dir,'catalog.dat');
  writeFileSync(archive,Buffer.from([0x1f,0x8b]));
  truncateSync(archive,20*1024*1024+1); // sparse source, no external data required
  await assert.rejects(()=>runHipparcosDownload(['--archive='+archive,'--out='+out]),/at most 20 MiB/);
  assert.equal(existsSync(out),false);
  assert.equal(existsSync(out+'.source.json'),false);
});

test('raw Hipparcos file and provenance manifest are written as a protected pair',t=>{
  const dir=mkdtempSync(join(tmpdir(),'hip2-source-pair-'));
  t.after(()=>rmSync(dir,{recursive:true,force:true}));
  const output=join(dir,'hip2.dat'),manifest=output+'.source.json';
  const bytes=Buffer.from('Synthetic bytes - not actual Hipparcos');
  const provenance={sourceUrl:null,rawSha256:sha(bytes)};
  const result=writeHipparcosSourcePair(output,manifest,bytes,provenance);
  assert.equal(result.rawFile,output);
  assert.deepEqual(readFileSync(output),bytes);
  assert.deepEqual(JSON.parse(readFileSync(manifest,'utf8')),provenance);
  assert.throws(()=>writeHipparcosSourcePair(output,manifest,bytes,provenance),/already exists/);
});

test('provenance output failure removes only files newly created by this invocation',t=>{
  const dir=mkdtempSync(join(tmpdir(),'hip2-rollback-'));
  t.after(()=>rmSync(dir,{recursive:true,force:true}));
  const rawFile=join(dir,'hip2.dat');
  const nonexistentManifest=join(dir,'not-a-real-dir','manifest.json');
  const raw=Buffer.from('Synthetic only');
  assert.throws(()=>writeHipparcosSourcePair(rawFile,nonexistentManifest,raw,{}),/ENOENT/);
  assert.equal(existsSync(rawFile),false);
  assert.equal(existsSync(nonexistentManifest),false);
  const existingManifest=join(dir,'existing-source.json');
  writeFileSync(existingManifest,'retained content');
  assert.throws(()=>writeHipparcosSourcePair(rawFile,existingManifest,raw,{}),/already exists/);
  assert.equal(existsSync(rawFile),false);
  assert.equal(readFileSync(existingManifest,'utf8'),'retained content');
});

test('download source cannot write through an external symlink into repository',{
  skip:process.platform==='win32'?'Windows directory symlinks may need elevated privileges':false,
},t=>{
  const dir=mkdtempSync(join(tmpdir(),'hip2-symlink-'));
  t.after(()=>rmSync(dir,{recursive:true,force:true}));
  const alias=join(dir,'repo-alias');
  const root=fileURLToPath(new URL('../',import.meta.url));
  symlinkSync(root,alias,'dir');
  const data=join(alias,'hip2-should-not-be-written.dat');
  assert.throws(()=>writeHipparcosSourcePair(data,join(dir,'outside.manifest.json'),Buffer.from('Test'),{}),/outside the Git repository/);
  assert.equal(existsSync(data),false);
});
