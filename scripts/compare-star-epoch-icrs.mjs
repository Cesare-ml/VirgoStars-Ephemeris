// Original VirgoStars source, CPAL-1.0; see repository LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
// Operator-only ICRS astrometric comparison; NOT a Swiss/apparent-position parity test.
import {createHash} from 'node:crypto';
import {readFileSync,writeFileSync,statSync,realpathSync} from 'node:fs';
import {resolve,isAbsolute,relative,dirname,basename,sep} from 'node:path';
import {pathToFileURL,fileURLToPath} from 'node:url';

const RAD=Math.PI/180, HIP_EPOCH=2448349.0625;
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const validDigest=h=>typeof h==='string'&&/^[0-9a-f]{64}$/.test(h);

/** Great-circle error: stable for tiny angles, RA wrapping and high declination. */
export function angularSeparationArcsec(ra1,dec1,ra2,dec2){
  for(const ra of [ra1,ra2])if(!Number.isFinite(ra)||ra<0||ra>=360)
    throw new RangeError('ICRS RA must be in [0,360) degrees');
  for(const dec of [dec1,dec2])if(!Number.isFinite(dec)||dec<=-90||dec>=90)
    throw new RangeError('ICRS Dec must be strictly between -90 and 90 degrees');
  const dRa=(ra2-ra1)*RAD,dDec=(dec2-dec1)*RAD;
  const hav=Math.sin(dDec/2)**2+Math.cos(dec1*RAD)*Math.cos(dec2*RAD)*Math.sin(dRa/2)**2;
  return 2*Math.asin(Math.sqrt(Math.min(1,Math.max(0,hav))))*180*3600/Math.PI;
}

function validateEpochs(epochs){
  if(!Array.isArray(epochs)||epochs.length<3||epochs.length>50||
     epochs.some(j=>!Number.isFinite(j)||j<2400000||j>2600000)||
     new Set(epochs).size!==epochs.length||!epochs.includes(HIP_EPOCH)||
     epochs.every(j=>Math.abs(j-HIP_EPOCH)<3652.5))
    throw new TypeError('Require 3+ unique plausible TDB Julian epochs including J1991.25 and a 10+ year baseline');
}
function validateMatrix(data,expected,sourceLabel){
  if(!data||typeof data!=='object'||Array.isArray(data)||data.schemaVersion!==1||
     data.coordinateFrame!=='ICRS_ASTROMETRIC'||data.epochConvention!=='TDB_JULIAN_DAY'||
     typeof data.catalogId!=='string'||!data.catalogId||
     !validDigest(data.inputCatalogSha256)||!Array.isArray(data.stars)||
     !data.stars.length||data.stars.length>117955)
    throw new TypeError(sourceLabel+': explicit ICRS astrometric frame, pinned catalog and stars required');
  validateEpochs(data.epochsTdbJulianDay);
  if(expected&&(data.catalogId!==expected.catalogId||
     data.inputCatalogSha256!==expected.inputCatalogSha256||
     JSON.stringify(data.epochsTdbJulianDay)!==JSON.stringify(expected.epochsTdbJulianDay)))
    throw new Error('Catalog/hash/epochs differ between reference and candidate');
  const records=new Map();
  for(const star of data.stars){
    if(!star||typeof star.name!=='string'||!star.name.trim()||
       star.name!==star.name.trim()||!Number.isSafeInteger(star.hip)||star.hip<1||
       records.has(star.hip)||!Array.isArray(star.samples)||
       star.samples.length!==data.epochsTdbJulianDay.length)
      throw new Error(sourceLabel+': duplicate/invalid HIP or missing epoch sample');
    const samples=new Map();
    for(const item of star.samples){
      if(!item||!Number.isFinite(item.tdbJulianDay)||
         !data.epochsTdbJulianDay.includes(item.tdbJulianDay)||
         samples.has(item.tdbJulianDay))
        throw new Error(sourceLabel+': invalid/duplicate sample epoch');
      angularSeparationArcsec(item.rightAscensionDeg,item.declinationDeg,
                              item.rightAscensionDeg,item.declinationDeg);
      samples.set(item.tdbJulianDay,item);
    }
    records.set(star.hip,{name:star.name,samples});
  }
  return records;
}

/** Only compare astrometric ICRS with astrometric ICRS at identical TDB JDs. */
export function compareIcrsStarEpochs(reference,candidate,{maxArcsec}={}){
  if(!Number.isFinite(maxArcsec)||maxArcsec<=0||maxArcsec>60)
    throw new RangeError('Explicit ICRS angular tolerance required in (0,60] arcsec');
  if(reference?.referenceModel!=='ERFA_PMSAFE'||
     reference?.status!=='INDEPENDENT_ALGORITHM_REFERENCE_NOT_ENGINE_PARITY'||
     reference?.scientificallyValidated!==false||!validDigest(reference?.inputAuditSha256))
    throw new Error('Reference must be source-pinned non-certified ERFA pmsafe output');
  if(typeof candidate?.candidateModel!=='string'||!candidate.candidateModel.trim()||
     /^(?:ERFA_PMSAFE|EXPERIMENTAL_ICRS_LINEAR_STAR)$/i.test(candidate.candidateModel)||
     candidate?.algorithm==='ERFA_PMSAFE')
    throw new Error('Candidate must identify a distinct ICRS engine, not ERFA or the apparent/ecliptic bridge');
  const ref=validateMatrix(reference,null,'reference');
  const got=validateMatrix(candidate,reference,'candidate');
  if(ref.size!==got.size)throw new Error('Reference and candidate HIP coverage differs');
  const perStar=[];let failures=0,comparisons=0,maxObservedArcsec=0,sumSquared=0;
  for(const [hip,star] of ref){
    const other=got.get(hip);
    if(!other||other.name!==star.name)
      throw new Error('HIP '+hip+': missing/different candidate named star');
    const errors=[];
    for(const jd of reference.epochsTdbJulianDay){
      const a=star.samples.get(jd),b=other.samples.get(jd);
      const error=angularSeparationArcsec(a.rightAscensionDeg,a.declinationDeg,
                                         b.rightAscensionDeg,b.declinationDeg);
      const withinTolerance=error<=maxArcsec;
      if(!withinTolerance)failures++;
      maxObservedArcsec=Math.max(maxObservedArcsec,error);
      sumSquared+=error**2;comparisons++;
      errors.push({tdbJulianDay:jd,separationArcsec:error,withinTolerance});
    }
    perStar.push({hip,name:star.name,maxSeparationArcsec:Math.max(...errors.map(e=>e.separationArcsec)),epochs:errors});
  }
  return {schemaVersion:1,
    status:failures?'NUMERIC_PARITY_FAILED':'NUMERIC_ICRS_PARITY_PASSED_NOT_PRODUCTION_CERTIFICATION',
    coordinateFrame:'ICRS_ASTROMETRIC',epochConvention:'TDB_JULIAN_DAY',
    referenceModel:reference.referenceModel,candidateModel:candidate.candidateModel,
    catalogId:reference.catalogId,sourceCatalogSha256:reference.inputCatalogSha256,
    toleranceArcsec:maxArcsec,stars:ref.size,epochsPerStar:reference.epochsTdbJulianDay.length,
    comparisons,failures,maxObservedArcsec,rmsArcsec:Math.sqrt(sumSquared/comparisons),
    scientificallyValidated:false,perStar,
    caveat:'Only SSB-centered ICRS astrometric directions are compared. Geocentric apparent/ecliptic positions, aberration, deflection, magnitudes, source authenticity and Swiss API behavior are not certified.'};
}

export function runIcrsEpochAudit(argv){
  const opts=Object.fromEntries(argv.map(x=>{
    const i=x.indexOf('=');
    if(!x.startsWith('--')||i<3)throw new Error('Expected --key=value');
    return [x.slice(0,i),x.slice(i+1)];
  }));
  const keys=['--reference','--reference-sha256','--candidate','--candidate-sha256','--max-arcsec','--out'];
  for(const k of keys)if(!opts[k])throw new Error('Required argument '+k+'=VALUE');
  if(Object.keys(opts).some(k=>!keys.includes(k))||
     new Set(argv.map(x=>x.slice(0,x.indexOf('=')))).size!==argv.length)
    throw new Error('Unknown or duplicate argument');
  const read=(path,digest)=>{
    if(!isAbsolute(path)||!validDigest(digest))
      throw new Error('Absolute input file and pinned lowercase SHA-256 required');
    const size=statSync(path);
    if(!size.isFile()||size.size<1||size.size>64*1024*1024)
      throw new Error('Input must be a regular file at most 64 MiB');
    const bytes=readFileSync(path);
    if(bytes.length!==size.size||sha(bytes)!==digest)
      throw new Error('Pinned input SHA-256 mismatch or size changed');
    return JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(bytes));
  };
  if(!isAbsolute(opts['--out']))throw new Error('Absolute report output required');
  const root=realpathSync(resolve(dirname(fileURLToPath(import.meta.url)),'..'));
  const path=resolve(opts['--out']),possible=[path];
  try{possible.push(resolve(realpathSync(dirname(path)),basename(path)));}
  catch(error){if(error?.code!=='ENOENT')throw error;}
  for(const dest of possible){
    const rel=relative(root,dest);
    if(rel===''||(rel!=='..'&&!rel.startsWith('..'+sep)&&!isAbsolute(rel)))
      throw new Error('Comparison report must remain outside Git repository');
  }
  const reference=read(opts['--reference'],opts['--reference-sha256']);
  const candidate=read(opts['--candidate'],opts['--candidate-sha256']);
  const result=compareIcrsStarEpochs(reference,candidate,{maxArcsec:Number(opts['--max-arcsec'])});
  writeFileSync(path,JSON.stringify(result,null,2)+'\n',{flag:'wx',mode:0o600});
  return {status:result.status,comparisons:result.comparisons,
    failures:result.failures,maxObservedArcsec:result.maxObservedArcsec,scientificallyValidated:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
  try{const r=runIcrsEpochAudit(process.argv.slice(2));console.log(JSON.stringify(r));if(r.failures)process.exitCode=2;}
  catch(error){console.error(error.message);process.exitCode=1;}
}
