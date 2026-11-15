// Original VirgoStars source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
// 2026-11-15: explicitly invoked, offline-output-only Hipparcos archive acquisition.
// No imports/calculations fetch external data; this CLI runs only when requested.
import {createHash} from 'node:crypto';
import {existsSync,readFileSync,writeFileSync,unlinkSync} from 'node:fs';
import {isAbsolute,resolve,relative,dirname,sep} from 'node:path';
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
  const declaredSize=Number(response.headers?.get?.('content-length'));
  if(Number.isFinite(declaredSize) && declaredSize>MAX_GZIP_BYTES)
    throw new Error('Hipparcos compressed archive exceeds the configured 20 MiB cap');
  const archive=Buffer.from(await response.arrayBuffer());
  return decodeHipparcos2007Archive(archive,{expectedArchiveSha256,requireFullCatalog,acquiredFromOfficialMirror:true});
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
  const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
  const relativeToRepository=relative(root,path);
  if(relativeToRepository==='' || (relativeToRepository!=='..' && !relativeToRepository.startsWith('..'+sep) && !isAbsolute(relativeToRepository)))
    throw new Error('Hipparcos data output must be outside the Git repository');
  if(existsSync(path) || existsSync(manifestPath))
    throw new Error('Output file already exists; no overwrite allowed');
  if(args['--archive']!==undefined && (!args['--archive'] || !isAbsolute(args['--archive'])))
    throw new Error('--archive must be an absolute path to an existing gzip file');
  const {raw,provenance}=args['--archive']===undefined
    ?await acquireHipparcos2007({expectedArchiveSha256:args['--archive-sha256']})
    :decodeHipparcos2007Archive(readFileSync(resolve(args['--archive'])),{
      expectedArchiveSha256:args['--archive-sha256'],
    });
  let wroteRaw=false;
  try{
    writeFileSync(path,raw,{flag:'wx',mode:0o600});wroteRaw=true;
    writeFileSync(manifestPath,JSON.stringify(provenance,null,2)+'\n',{flag:'wx',mode:0o600});
  }catch(error){
    if(wroteRaw) try{unlinkSync(path)}catch{}
    throw error;
  }
  return {rawFile:path,manifestFile:manifestPath,rawSha256:provenance.rawSha256,rows:provenance.rows};
}
if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
  try{console.log(JSON.stringify(await runHipparcosDownload(process.argv.slice(2))));}
  catch(error){console.error(error.message);process.exitCode=1;}
}
