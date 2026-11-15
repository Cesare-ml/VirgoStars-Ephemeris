// Original VirgoStars source, CPAL-1.0; see repository LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
// Operator-only native Rust ICRS candidate exporter; never uses ERFA positions.
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {readFileSync,statSync,writeFileSync,realpathSync} from 'node:fs';
import {resolve,isAbsolute,dirname,basename,relative,sep} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';

const digest=b=>createHash('sha256').update(b).digest('hex');
const pinned=(path,hash)=>{
  if(!isAbsolute(path)||!/^[a-f0-9]{64}$/.test(hash))throw new Error('Absolute file path and lowercase SHA-256 required');
  const stat=statSync(path);
  if(!stat.isFile()||stat.size<1||stat.size>64*1024*1024)throw new Error('Pinned input must be a regular file <=64 MiB');
  const bytes=readFileSync(path);
  if(bytes.length!==stat.size||digest(bytes)!==hash)throw new Error('Pinned source SHA-256 mismatch');
  return JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(bytes));
};
const epoch=2448349.0625;
const fields=['epochTdbJulianDay','rightAscensionDeg','declinationDeg',
  'properMotionRaCosDecMasPerYear','properMotionDecMasPerYear','parallaxMas','radialVelocityKmPerSecond'];

export function buildNativeIcrsCandidate(catalog,reference,calculate){
  if(!catalog||typeof catalog.id!=='string'||!Array.isArray(catalog.records)||
     !reference||reference.referenceModel!=='ERFA_PMSAFE'||
     reference.coordinateFrame!=='ICRS_ASTROMETRIC'||
     reference.epochConvention!=='TDB_JULIAN_DAY'||
     reference.catalogId!==catalog.id||
     !Array.isArray(reference.epochsTdbJulianDay)||reference.epochsTdbJulianDay.length<3||
     !reference.epochsTdbJulianDay.includes(epoch)||
     !Array.isArray(reference.stars)||!reference.stars.length||
     typeof calculate!=='function')
    throw new Error('Pinned ERFA ICRS metadata and genuine source catalog required');
  const byName=new Map(catalog.records.map(r=>[r.name,r]));
  if(byName.size!==catalog.records.length||catalog.records.length!==reference.stars.length)
    throw new Error('Catalog/reference star coverage mismatch');
  const seen=new Set();
  const stars=reference.stars.map(meta=>{
    if(!Number.isSafeInteger(meta.hip)||meta.hip<1||seen.has(meta.hip)||
       typeof meta.name!=='string'||!byName.has(meta.name))
      throw new Error('Duplicate/missing HIP name');
    seen.add(meta.hip);
    const entry=byName.get(meta.name);
    if(entry.epochTdbJulianDay!==epoch||fields.some(k=>!Number.isFinite(entry[k])))
      throw new Error('Invalid astrometric record');
    const samples=reference.epochsTdbJulianDay.map(jd=>{
      if(!Number.isFinite(jd)||jd<2400000||jd>2600000)
        throw new Error('Invalid TDB epoch');
      const result=calculate(entry,jd,meta.hip);
      if(!result||result.tdbJulianDay!==jd||
         !Number.isFinite(result.rightAscensionDeg)||result.rightAscensionDeg<0||
         result.rightAscensionDeg>=360||!Number.isFinite(result.declinationDeg)||
         Math.abs(result.declinationDeg)>=90)
        throw new Error('Native Rust ICRS output missing/invalid');
      return result;
    });
    return {hip:meta.hip,name:meta.name,samples};
  });
  return {schemaVersion:1,coordinateFrame:'ICRS_ASTROMETRIC',
    epochConvention:'TDB_JULIAN_DAY',catalogId:catalog.id,
    inputCatalogSha256:reference.inputCatalogSha256,
    candidateModel:'VIRGOSTARS_RUST_RECTILINEAR_ICRS_ASTROMETRIC_V1',
    epochsTdbJulianDay:[...reference.epochsTdbJulianDay],
    stars,scientificallyValidated:false,
    limitations:['Kernel-free SSB-centered rectilinear 6D propagation',
      'No stellar light-time, no apparent geocentric/ecliptic transformation',
      'Reference epochs/identities only; no ERFA reference coordinates copied']};
}

export function runNativeIcrsExport(argv){
  const options=Object.fromEntries(argv.map(arg=>{
    const i=arg.indexOf('=');
    if(!arg.startsWith('--')||i<3)throw new Error('Expected --name=value');
    return [arg.slice(0,i),arg.slice(i+1)];
  }));
  const required=['--catalog','--catalog-sha256','--reference','--reference-sha256','--executable','--out'];
  if(argv.length!==required.length||required.some(k=>!options[k])||Object.keys(options).some(k=>!required.includes(k)))
    throw new Error('Required: --catalog --catalog-sha256 --reference --reference-sha256 --executable --out');
  const out=options['--out'];
  if(!isAbsolute(out)||!isAbsolute(options['--executable']))
    throw new Error('Absolute output and Rust executable paths required');
  const root=realpathSync(resolve(dirname(fileURLToPath(import.meta.url)),'..'));
  const paths=[resolve(out)];
  try{paths.push(resolve(realpathSync(dirname(out)),basename(out)));}
  catch(e){if(e?.code!=='ENOENT')throw e;}
  for(const p of paths){
    const rel=relative(root,p);
    if(rel===''||(rel!=='..'&&!rel.startsWith('..'+sep)&&!isAbsolute(rel)))
      throw new Error('Candidate output must remain outside Git');
  }
  const catalog=pinned(options['--catalog'],options['--catalog-sha256']);
  const reference=pinned(options['--reference'],options['--reference-sha256']);
  if(reference.inputCatalogSha256!==options['--catalog-sha256'])
    throw new Error('Reference source SHA-256 differs from catalog input');
  const result=buildNativeIcrsCandidate(catalog,reference,(entry,jd,hip)=>{
    const args=[...fields.map(k=>String(entry[k])),String(jd),String(hip)];
    const child=spawnSync(options['--executable'],args,{encoding:'utf8',windowsHide:true,
      timeout:10000,maxBuffer:4096});
    if(child.error||child.status!==0)throw new Error('Rust ICRS exporter failed: '+(child.error?.message??child.stderr));
    const lines=child.stdout.trim().split(/\r?\n/);
    if(lines.length!==2||lines[0]!=='hip,tdb_jd,ra_icrs_deg,dec_icrs_deg,distance_au')
      throw new Error('Invalid native Rust ICRS CSV header');
    const columns=lines[1].split(',');
    if(columns.length!==5||columns.some(v=>!v.trim()||!Number.isFinite(Number(v)))||
       Number(columns[0])!==hip||Math.abs(Number(columns[1])-jd)>1e-8||
       Number(columns[4])<=0)
      throw new Error('Invalid native Rust ICRS CSV values');
    return {tdbJulianDay:jd,rightAscensionDeg:Number(columns[2]),declinationDeg:Number(columns[3])};
  });
  writeFileSync(out,JSON.stringify(result,null,2)+'\n',{flag:'wx',mode:0o600});
  return {output:out,stars:result.stars.length,epochs:result.epochsTdbJulianDay.length,
    scientificallyValidated:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
  try{console.log(JSON.stringify(runNativeIcrsExport(process.argv.slice(2))));}
  catch(e){console.error(e.message);process.exitCode=1;}
}
