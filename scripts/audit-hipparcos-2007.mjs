// Original VirgoStars source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
// Added 2026-11-15: offline observational-quality audit; NO implied numerical certification.
import {createHash} from 'node:crypto';
import {readFileSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {HIP2_ROWS,parseHip2Line} from './import-hipparcos-2007.mjs';

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const optionalNonnegative = (line,start,end,label) => {
  const raw = line.slice(start,end).trim();
  if (!raw) return null;
  if (!/^(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(raw)) throw new Error(`Invalid ${label}`);
  const value=Number(raw);
  if (!Number.isFinite(value)) throw new Error(`Invalid ${label}`);
  return value;
};
export function parseHip2Uncertainties(line) {
  if (line.length < 103) throw new Error('Hipparcos quality columns truncated');
  return {
    raMas:optionalNonnegative(line,69,75,'e_RArad'),
    decMas:optionalNonnegative(line,76,82,'e_DErad'),
    parallaxMas:optionalNonnegative(line,83,89,'e_Plx'),
    properMotionRaMasPerYear:optionalNonnegative(line,90,96,'e_pmRA'),
    properMotionDecMasPerYear:optionalNonnegative(line,97,103,'e_pmDE'),
  };
}

export function auditHipparcosTargets(hip2Bytes,targetsManifestBytes,{
  hip2Sha256,requireFullCatalog=true,minParallaxSignalToNoise=5,
}={}) {
  if (!Buffer.isBuffer(hip2Bytes) || hip2Bytes.length<1 || hip2Bytes.length>60*1024*1024) throw new Error('Raw Hipparcos must be an uncompressed local Buffer (max 60 MiB)');
  if (typeof hip2Sha256!=='string' || !/^[0-9a-f]{64}$/.test(hip2Sha256) || sha256(hip2Bytes)!==hip2Sha256) throw new Error('Explicit verified raw Hipparcos SHA-256 mismatch');
  if (!Number.isFinite(minParallaxSignalToNoise) || minParallaxSignalToNoise<=0) throw new Error('Positive SNR threshold required');
  const targets=JSON.parse(targetsManifestBytes.toString('utf8'));
  if (targets.schemaVersion!==1 || !Array.isArray(targets.stars) || !targets.stars.length) throw new Error('Valid fixed-star target manifest required');
  const byHip=new Map(), byName=new Set();
  for (const star of targets.stars) {
    if (!Number.isSafeInteger(star.hip) || star.hip<=0 || typeof star.name!=='string' || !star.name.trim() || byHip.has(star.hip) || byName.has(star.name.toLowerCase())) throw new Error('Duplicate or invalid HIP/name in targets');
    if (typeof star.simbadUrl!=='string' || !star.simbadUrl.startsWith('https://')) throw new Error('Each target requires an HTTPS crossmatch reference');
    byHip.set(star.hip,star);
    byName.add(star.name.toLowerCase());
  }
  const raw=hip2Bytes.toString('utf8');
  const lines=raw.split(/\r?\n/);
  if (lines.at(-1)==='') lines.pop();
  if (requireFullCatalog && lines.length!==HIP2_ROWS) throw new Error(`Expected ${HIP2_ROWS} raw Hipparcos rows, got ${lines.length}`);
  const seenHip=new Set(),results=new Map();
  for (let i=0;i<lines.length;i++) {
    const line=lines[i];
    if (line.length!==276) throw new Error(`hip2.dat line ${i+1}: expected 276 characters, got ${line.length}`);
    const hipText=line.slice(0,6).trim();
    if (!/^\d+$/.test(hipText)) throw new Error(`Invalid HIP in source line ${i+1}`);
    const hip=Number(hipText);
    if (seenHip.has(hip)) throw new Error(`Repeated source HIP ${hip}`);
    seenHip.add(hip);
    const target=byHip.get(hip);
    if (!target) continue;
    const obs=parseHip2Line(line);
    const uncertainties=parseHip2Uncertainties(line);
    const isSimple=obs.solutionCode===5 && obs.components===1;
    const snr=uncertainties.parallaxMas!==null && uncertainties.parallaxMas>0 && obs.parallaxMas>0
      ? obs.parallaxMas/uncertainties.parallaxMas : null;
    const status=!isSimple ? 'COMPLEX_ASTROMETRY_REVIEW'
      : obs.parallaxMas<=0 ? 'NONPOSITIVE_PARALLAX'
      : snr===null ? 'PARALLAX_UNCERTAINTY_UNAVAILABLE'
      : snr<minParallaxSignalToNoise ? 'LOW_PARALLAX_SIGNAL_TO_NOISE'
      : 'ASTROMETRY_PRECHECK_PASSED';
    results.set(hip,{
      name:target.name,hip,reference:target.simbadUrl,sourceRow:i+1,
      solutionCode:obs.solutionCode,components:obs.components,
      icrsEpoch:'J1991.25',rightAscensionRad:obs.ra,declinationRad:obs.dec,
      parallaxMas:obs.parallaxMas,
      properMotionRaCosDecMasPerYear:obs.pmRa,properMotionDecMasPerYear:obs.pmDec,
      uncertainties,parallaxSignalToNoise:snr,status,
      catalogReady:false,
      blockers:['No independently vetted Johnson V magnitude attached','No independently vetted barycentric radial velocity attached','No epoch-by-epoch independent position calculation validation'],
    });
  }
  const stars=targets.stars.map(star=>results.get(star.hip) ?? {
    hip:star.hip,name:star.name,reference:star.simbadUrl,status:'HIP_NOT_FOUND',
    catalogReady:false,blockers:['Crossmatch not located in supplied Hipparcos file'],
  });
  const statusCounts={};
  for(const star of stars) statusCounts[star.status]=(statusCounts[star.status]??0)+1;
  return {
    schemaVersion:1,status:'OBSERVATIONAL_AUDIT_ONLY_NOT_SCIENTIFIC_CERTIFICATION',
    source:{catalog:'CDS I/311/hip2',mirror:'https://vizier.cfa.harvard.edu/ftp/cats/I/311/',
      sha256:hip2Sha256,rawRows:lines.length,expectedRows:HIP2_ROWS},
    manifestSha256:sha256(targetsManifestBytes),
    policy:{minParallaxSignalToNoise,requirements:'Sn=5, Nc=1; positive parallax and measured e_Plx'},
    counts:statusCounts,stars,
    note:'Observed astrometric values belong to the external Hipparcos catalog; no V/RV are inferred, no apparent stellar positions are calculated and this is not Swiss compatibility validation.',
  };
}

export function runHipparcosTargetAudit(argv){
  const options=Object.fromEntries(argv.filter(x=>x.includes('=')).map(s=>{const p=s.indexOf('=');return [s.slice(0,p),s.slice(p+1)];}));
  for(const key of ['--hip2','--hip2-sha256','--targets','--out']) if(!options[key])throw new Error(`Required: ${key}=VALUE`);
  const min=options['--min-parallax-snr']===undefined?5:Number(options['--min-parallax-snr']);
  const result=auditHipparcosTargets(readFileSync(resolve(options['--hip2'])),
    readFileSync(resolve(options['--targets'])),{hip2Sha256:options['--hip2-sha256'],minParallaxSignalToNoise:min});
  writeFileSync(resolve(options['--out']),JSON.stringify(result,null,2)+'\n',{flag:'wx'});
  return {sourceRows:result.source.rawRows,targets:result.stars.length,statusCounts:result.counts};
}
if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
  try {console.log(JSON.stringify(runHipparcosTargetAudit(process.argv.slice(2))));}
  catch(error){console.error(error.message);process.exitCode=1;}
}
