// Original VirgoStars source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
// 2026-11-15: explicitly invoked, offline-output-only Hipparcos archive acquisition.
// No imports/calculations fetch external data; this CLI runs only when requested.
import {createHash} from 'node:crypto';
import {existsSync,readFileSync,writeFileSync,unlinkSync,statSync,realpathSync,openSync,closeSync} from 'node:fs';
import {isAbsolute,resolve,relative,dirname,basename,sep} from 'node:path';
import {gunzipSync} from 'node:zlib';
import {pathToFileURL,fileURLToPath} from 'node:url';
import {readHip2Rows,HIP2_MAX_BYTES} from './hipparcos-source-rows.mjs';

export const OFFICIAL_HIP2_GZIP_URL='https://vizier.cfa.harvard.edu/ftp/cats/I/311/hip2.dat.gz';
const MAX_GZIP_BYTES=20*1024*1024;
const SHA_PATTERN=/^[a-f0-9]{64}$/;
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');

/**
 * Decodes only explicitly supplied gzip bytes. Provenance labels distinguish
 * an archive fetched from a known URL from an arbitrary user-supplied file.
 * Neither mode establishes that its contents have passed scientific review.
 */
export function decodeHipparcos2007Archive(archive,{
  expectedArchiveSha256,
  requireFullCatalog=true,
  acquiredFromOfficialMirror=false,
}={}) {
  if(expectedArchiveSha256!==undefined && !SHA_PATTERN.test(expectedArchiveSha256))
    throw new Error('Expected archive SHA-256 must be 64 lowercase hex characters');
  if(!Buffer.isBuffer(archive) || archive.length<2 || archive.length>MAX_GZIP_BYTES || archive[0]!==0x1f || archive[1]!==0x8b)
    throw new Error('Source did not return a valid size-bounded gzip archive');
  const archiveSha256=sha(archive);
  if(expectedArchiveSha256!==undefined && archiveSha256!==expectedArchiveSha256)
    throw new Error('Archive SHA-256 does not match independently pinned input');
  let raw;
  try {raw=gunzipSync(archive,{maxOutputLength:HIP2_MAX_BYTES});}
  catch {throw new Error('Hipparcos gzip decompression or output-size verification failed');}
  const lines=readHip2Rows(raw,{requireFullCatalog});
  return {
    raw,
    provenance:{
      sourceUrl:acquiredFromOfficialMirror?OFFICIAL_HIP2_GZIP_URL:null,
      acquisitionMode:acquiredFromOfficialMirror?'EXPLICIT_HTTPS_DOWNLOAD':'OPERATOR_SUPPLIED_LOCAL_ARCHIVE',
      catalog:'CDS I/311/hip2 candidate; van Leeuwen 2007; corrected release dated 16 September 2008',
      sourceArchiveSha256:archiveSha256,
      sourceArchiveSha256IndependentPin:expectedArchiveSha256!==undefined,
      rawSha256:sha(raw),
      rows:lines.length,
      rowLength:276,
      provenanceStatus:'SOURCE_AND_LOCAL_BYTES_RECORDED_NOT_DATA_AUTHENTICITY_OR_SCIENCE_CERTIFIED',
      sourceReadMeUrl:'https://vizier.cfa.harvard.edu/ftp/cats/I/311/ReadMe',
      warning:'A checksum pins exact bytes but does not authenticate an untrusted source. Review data authenticity, attribution and redistribution rights; do not commit the raw catalog to Git.',
    },
  };
}

/**
 * Consume an HTTPS response in bounded chunks. Fetch's response.arrayBuffer()
 * would buffer an arbitrarily large response before the compressed-size limit
 * could be applied. Real Node fetch responses expose a Web ReadableStream.
 * Reject missing/non-streaming bodies rather than falling back to unbounded IO.
 */
export async function readBoundedArchiveResponse(response,{maxBytes=MAX_GZIP_BYTES}={}){
  if(!Number.isSafeInteger(maxBytes) || maxBytes<2 || maxBytes>MAX_GZIP_BYTES)
    throw new Error('Invalid compressed-stream size limit');
  const rawHeader=response?.headers?.get?.('content-length');
  let declaredSize=null;
  if(rawHeader!==null && rawHeader!==undefined){
    if(!/^\d+$/.test(rawHeader)) throw new Error('Invalid Hipparcos Content-Length header');
    declaredSize=Number(rawHeader);
    if(!Number.isSafeInteger(declaredSize) || declaredSize>maxBytes)
      throw new Error('Hipparcos compressed archive exceeds the configured 20 MiB cap');
  }
  const reader=response?.body?.getReader?.();
  if(!reader || typeof reader.read!=='function')
    throw new Error('Hipparcos HTTPS response must have a streaming body');
  const chunks=[];
  let bytesRead=0,completed=false;
  try {
    for(;;){
      const {value,done}=await reader.read();
      if(done){completed=true;break;}
      if(!(value instanceof Uint8Array))throw new Error('Invalid Hipparcos download stream chunk');
      if(value.length>maxBytes-bytesRead)
        throw new Error('Hipparcos compressed archive exceeds the configured 20 MiB cap');
      bytesRead+=value.length;
      chunks.push(Buffer.from(value));
    }
  } finally {
    if(!completed)try{await reader.cancel();}catch{}
    try{reader.releaseLock();}catch{}
  }
  if(bytesRead<2)throw new Error('Hipparcos compressed archive is empty or truncated');
  if(declaredSize!==null && bytesRead!==declaredSize)
    throw new Error('Hipparcos Content-Length does not match streamed archive bytes');
  return Buffer.concat(chunks,bytesRead);
}

export async function acquireHipparcos2007({
  fetchImpl=fetch,
  expectedArchiveSha256,
  requireFullCatalog=true,
}={}) {
  if(typeof fetchImpl!=='function') throw new TypeError('A fetch implementation is required');
  if(expectedArchiveSha256!==undefined && !SHA_PATTERN.test(expectedArchiveSha256))
    throw new Error('Expected archive SHA-256 must be 64 lowercase hex characters');
  // An explicit operator action, never an import side effect or a runtime calculation.
  const response=await fetchImpl(OFFICIAL_HIP2_GZIP_URL,{
    method:'GET',redirect:'error',headers:{'accept-encoding':'identity'},
    signal:AbortSignal.timeout(120000),
  });
  if(!response || !response.ok) throw new Error(`Official Hipparcos download failed: HTTP ${response?.status??'unknown'}`);
  const archive=await readBoundedArchiveResponse(response);
  return decodeHipparcos2007Archive(archive,{expectedArchiveSha256,requireFullCatalog,acquiredFromOfficialMirror:true});
}


/** Validate both output paths including their symlink-resolved parents. */
export function assertHipparcosDestination(out,manifestPath){
  if(typeof out!=='string' || !isAbsolute(out) || typeof manifestPath!=='string' || !isAbsolute(manifestPath))
    throw new Error('Output and manifest must be absolute paths outside Git');
  const root=realpathSync(resolve(dirname(fileURLToPath(import.meta.url)),'..'));
  const paths=[resolve(out),resolve(manifestPath)];
  if(paths[0]===paths[1])throw new Error('Source output and provenance manifest must be distinct files');
  for(const dest of paths){
    const candidates=[dest];
    try{candidates.push(resolve(realpathSync(dirname(dest)),basename(dest)));}
    catch(error){if(error?.code!=='ENOENT')throw error;}
    for(const candidate of candidates){
      const rel=relative(root,candidate);
      if(rel==='' || (rel!=='..' && !rel.startsWith('..'+sep) && !isAbsolute(rel)))
        throw new Error('Hipparcos data output must be outside the Git repository');
    }
  }
  if(paths.some(p=>existsSync(p)))throw new Error('Output file already exists; no overwrite allowed');
  return paths;
}

/** Best-effort synchronous rollback, not a crash-atomic filesystem transaction. */
export function writeHipparcosSourcePair(out,manifestPath,raw,provenance){
  const paths=assertHipparcosDestination(out,manifestPath);
  const created=[];
  try{
    for(const [dest,contents] of [[paths[0],raw],[paths[1],JSON.stringify(provenance,null,2)+'\n']]){
      const fd=openSync(dest,'wx',0o600);
      created.push(dest);
      try{writeFileSync(fd,contents);}
      finally{closeSync(fd);}
    }
  }catch(error){
    for(const dest of created.reverse())try{unlinkSync(dest);}catch{}
    throw error;
  }
  return {rawFile:paths[0],manifestFile:paths[1]};
}

export async function runHipparcosDownload(argv) {
  const args=Object.fromEntries(argv.filter(x=>x.startsWith('--')&&x.includes('=')).map(s=>{
    const i=s.indexOf('=');return [s.slice(0,i),s.slice(i+1)];
  }));
  for(const key of Object.keys(args)) if(!['--out','--archive-sha256','--archive'].includes(key))
    throw new Error(`Unknown option ${key}`);
  const out=args['--out'];
  if(typeof out!=='string' || !isAbsolute(out)) throw new Error('--out must be an absolute file path outside the repository');
  const path=resolve(out),manifestPath=path+'.source.json';
  assertHipparcosDestination(path,manifestPath);
  if(args['--archive']!==undefined && (!args['--archive'] || !isAbsolute(args['--archive'])))
    throw new Error('--archive must be an absolute path to an existing gzip file');
  // Bound local archive reads before allocating an entire caller-supplied file.
  // This is not a trust/authenticity check: the independent hash remains optional.
  let localArchive;
  if(args['--archive']!==undefined){
    const localArchivePath=resolve(args['--archive']);
    const stats=statSync(localArchivePath);
    if(!stats.isFile() || stats.size<2 || stats.size>MAX_GZIP_BYTES)
      throw new Error('Local Hipparcos archive must be a regular gzip file of at most 20 MiB');
    localArchive=readFileSync(localArchivePath);
  }
  const {raw,provenance}=args['--archive']===undefined
    ?await acquireHipparcos2007({expectedArchiveSha256:args['--archive-sha256']})
    :decodeHipparcos2007Archive(localArchive,{
      expectedArchiveSha256:args['--archive-sha256'],
    });
  const written=writeHipparcosSourcePair(path,manifestPath,raw,provenance);
  return {...written,rawSha256:provenance.rawSha256,rows:provenance.rows};
}
if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
  try{console.log(JSON.stringify(await runHipparcosDownload(process.argv.slice(2))));}
  catch(error){console.error(error.message);process.exitCode=1;}
}
