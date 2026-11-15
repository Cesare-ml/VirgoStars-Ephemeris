// Original VirgoStars source, CPAL-1.0; see repository LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
// Added 2026-11-15: offline provenance-checked Hipparcos 2007 fixed-star importer.
import {createHash} from 'node:crypto';
import {readFileSync, writeFileSync, existsSync, unlinkSync} from 'node:fs';
import {resolve, relative, dirname, isAbsolute, sep} from 'node:path';
import {pathToFileURL, fileURLToPath} from 'node:url';
import {readHip2Rows,HIP2_EXPECTED_ROWS,parseHip2Uncertainties} from './hipparcos-source-rows.mjs';

export const HIP2_ROWS = HIP2_EXPECTED_ROWS;
export const HIP_EPOCH_TDB_JD = 2448349.0625; // Julian epoch J1991.25
const RAD_TO_DEG = 180 / Math.PI;
const COLUMNS = ['HIP','name','aliases','visualMagnitude','visualMagnitudeSource','radialVelocityKmPerSecond','radialVelocitySource','radialVelocityReferenceFrame','nameSource'];
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const numeric = (text, label) => {
  if (!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(text.trim())) throw new Error(`Missing/invalid ${label}`);
  const value = Number(text);
  if (!Number.isFinite(value)) throw new Error(`Non-finite ${label}`);
  return value;
};
const source = (text, label) => {
  try {const url = new URL(text); if (url.protocol !== 'https:' || !url.hostname) throw new Error();}
  catch {throw new Error(`${label} must be an explicit HTTPS provenance URL`);}
  return text;
};
const checkDigest = (bytes, expected, label) => {
  if (!/^[a-f0-9]{64}$/.test(expected ?? '')) throw new Error(`${label}: explicit lowercase SHA-256 is required`);
  if (sha256(bytes) !== expected) throw new Error(`${label}: SHA-256 mismatch`);
};

// Official CDS I/311/hip2 ReadMe: 1-based byte offsets; fields are RADIANS, epoch J1991.25.
export function parseHip2Line(line) {
  if (line.length < 68) throw new Error('Truncated hip2.dat row');
  const hip = numeric(line.slice(0,6), 'HIP');
  const solutionCode = numeric(line.slice(7,10), 'Sn');
  const components = numeric(line.slice(13,14), 'Nc');
  if (!Number.isSafeInteger(solutionCode) || solutionCode < 0 || solutionCode > 159 || ![1,3,5,7,9].includes(solutionCode % 10) || !Number.isSafeInteger(components) || components < 1) throw new Error('Invalid Hipparcos solution or components');
  const ra = numeric(line.slice(15,28), 'RArad');
  const dec = numeric(line.slice(29,42), 'DErad');
  const parallaxMas = numeric(line.slice(43,50), 'Plx');
  const pmRa = numeric(line.slice(51,59), 'pmRA');
  const pmDec = numeric(line.slice(60,68), 'pmDE');
  if (!Number.isSafeInteger(hip) || hip <= 0 || ra < 0 || ra >= 2*Math.PI || Math.abs(dec) >= Math.PI/2) throw new Error(`HIP ${hip}: invalid astrometry`);
  return {hip,solutionCode,components,ra,dec,parallaxMas,pmRa,pmDec};
}

export function parseSupplementTsv(tsv) {
  const lines = tsv.replace(/^\uFEFF/, '').trimEnd().split(/\r?\n/);
  if (lines.shift() !== COLUMNS.join('\t')) throw new Error(`Supplement TSV must start with: ${COLUMNS.join(' | ')}`);
  const rows = new Map(), aliasesUsed = new Set();
  for (const [index, line] of lines.entries()) {
    if (!line.trim() || line.startsWith('#')) continue;
    const fields = line.split('\t');
    if (fields.length !== COLUMNS.length) throw new Error(`Supplement line ${index+2}: expected ${COLUMNS.length} tab-separated columns`);
    const [id,name,aliases,mag,magSource,rv,rvSource,rvFrame,nameSource] = fields;
    if (rvFrame !== 'BARYCENTRIC') throw new Error('Radial velocity requires explicitly verified BARYCENTRIC reference frame');
    const hip = numeric(id, 'HIP');
    if (!Number.isSafeInteger(hip) || hip <= 0 || rows.has(hip)) throw new Error(`Duplicate or invalid supplement HIP ${id}`);
    const aliasList = aliases ? aliases.split('|').map(v=>v.trim()) : [];
    const completeNames = [name, ...aliasList, `HIP ${hip}`];
    if (completeNames.some(v=> !v || v !== v.trim()) || aliasList.length > 19) throw new Error(`HIP ${hip}: invalid name/aliases`);
    for (const nameValue of completeNames) {
      const key = nameValue.toLowerCase();
      if (aliasesUsed.has(key)) throw new Error(`Duplicate star name/alias ${nameValue}`);
      aliasesUsed.add(key);
    }
    const visualMagnitude = numeric(mag, 'visual V magnitude');
    const radialVelocityKmPerSecond = numeric(rv, 'barycentric radial velocity km/s');
    if (Math.abs(radialVelocityKmPerSecond) >= 299792.458) throw new Error(`HIP ${hip}: invalid radial velocity`);
    rows.set(hip, {hip,name,aliases:[...aliasList,`HIP ${hip}`],visualMagnitude,
      radialVelocityKmPerSecond,
      provenance:{visualMagnitudeSource:source(magSource,'V magnitude'),radialVelocitySource:source(rvSource,'radial velocity'),radialVelocityReferenceFrame:rvFrame,nameSource:source(nameSource,'name')}});
  }
  if (!rows.size) throw new Error('No supplement stars');
  return rows;
}

export function buildHipparcosCatalog(hip2Bytes, supplementsBytes, {
  hip2Sha256, supplementsSha256, requireFullCatalog = true, allowComplexSolutions = false,
  minParallaxSignalToNoise = 5,
} = {}) {
  if (!Number.isFinite(minParallaxSignalToNoise) || minParallaxSignalToNoise <= 0 || minParallaxSignalToNoise > 1000)
    throw new Error('Parallax SNR threshold must be finite, positive and at most 1000');
  checkDigest(hip2Bytes, hip2Sha256, 'Hipparcos I/311 hip2.dat');
  checkDigest(supplementsBytes, supplementsSha256, 'Curated supplements');
  const supplement = parseSupplementTsv(supplementsBytes.toString('utf8'));
  const found = new Set(), records = [], auditStars = [];
  const lines=readHip2Rows(hip2Bytes,{requireFullCatalog});
  for (const line of lines) {
    const hip=Number(line.slice(0,6).trim());
    if (!supplement.has(hip)) continue;
    if (found.has(hip)) throw new Error(`Duplicate Hipparcos HIP ${hip}`);
    found.add(hip);
    const ast = parseHip2Line(line), extra = supplement.get(hip);
    const isStandard = ast.solutionCode === 5 && ast.components === 1;
    if (!isStandard && !allowComplexSolutions) throw new Error(`HIP ${hip}: non-standard/multiple/variable astrometric solution (${ast.solutionCode}, ${ast.components} components); explicit scientific review required`);
    if (!(ast.parallaxMas >= 0.001 && ast.parallaxMas <= 10000)) throw new Error(`HIP ${hip}: missing/nonpositive/out-of-range parallax; no invented distance`);
    const formalUncertainties=parseHip2Uncertainties(line);
    if(Object.values(formalUncertainties).some(x=>x===null))
      throw new Error(`HIP ${hip}: missing formal astrometric uncertainty; source quality cannot be certified`);
    if(!(formalUncertainties.parallaxMas>0))
      throw new Error(`HIP ${hip}: parallax formal uncertainty must be strictly positive`);
    const parallaxSignalToNoise=ast.parallaxMas/formalUncertainties.parallaxMas;
    if(parallaxSignalToNoise<minParallaxSignalToNoise)
      throw new Error(`HIP ${hip}: low parallax signal-to-noise (${parallaxSignalToNoise.toFixed(2)} < ${minParallaxSignalToNoise}); exclude or review separately`);
    const record = {name:extra.name,aliases:extra.aliases,epochTdbJulianDay:HIP_EPOCH_TDB_JD,
      rightAscensionDeg:ast.ra * RAD_TO_DEG,declinationDeg:ast.dec * RAD_TO_DEG,
      properMotionRaCosDecMasPerYear:ast.pmRa,properMotionDecMasPerYear:ast.pmDec,
      parallaxMas:ast.parallaxMas,radialVelocityKmPerSecond:extra.radialVelocityKmPerSecond,
      visualMagnitude:extra.visualMagnitude};
    records.push(record);
    auditStars.push({hip, name:extra.name, solutionCode:ast.solutionCode, components:ast.components,
      astrometryReviewStatus:isStandard?'STANDARD_5_PARAM':'EXPERIMENTAL_COMPLEX_SOLUTION_NOT_CERTIFIED',
      formalUncertainties,parallaxSignalToNoise,qualityPolicyMinParallaxSignalToNoise:minParallaxSignalToNoise,
      ...extra.provenance});
  }
  for (const hip of supplement.keys()) if (!found.has(hip)) throw new Error(`HIP ${hip}: not found in Hipparcos source`);
  const sorted = records.map((record,i)=>({record,audit:auditStars[i]})).sort((a,b)=>a.audit.hip-b.audit.hip);
  const catalog = {id:`hipparcos-2007-curated-${hip2Sha256.slice(0,12)}-${supplementsSha256.slice(0,12)}-${allowComplexSolutions?'complex':'standard'}`,
    records:sorted.map(x=>x.record)};
  const audit = {schemaVersion:1,createdBy:'VirgoStars offline importer (not independent scientific certification)',
    hipparcos:{catalog:'CDS I/311/hip2',url:'https://cdsarc.cds.unistra.fr/viz-bin/ReadMe/I/311',
      epoch:'ICRS J1991.25, JD 2448349.0625 (Julian epoch, TDB convention)',rows:lines.length,sha256:hip2Sha256},
    supplements:{sha256:supplementsSha256,notes:'External V-band magnitudes, barycentric radial velocities and names; check original sources and redistribution rights per row'},
    units:{raDec:'degrees',pm:'mas/Julian year (RA includes cos declination)',parallax:'mas',radialVelocity:'km/s',magnitude:'Johnson V'},
    status:'Catalog provenance asserted by inputs; data rights and independent numeric validation remain separate release gates',
    complexSolutionsAllowed:allowComplexSolutions,
    qualityPolicy:{requireAllFiveFormalUncertainties:true,positiveFormalParallaxError:true,minParallaxSignalToNoise},
    stars:sorted.map(x=>x.audit)};
  return {catalog,audit};
}

export function importHipparcos2007(args) {
  const options = Object.fromEntries(args.filter(x=>x.includes('=')).map(x=>{const i=x.indexOf('=');return [x.slice(0,i),x.slice(i+1)];}));
  const required = ['--hip2','--supplements','--hip2-sha256','--supplements-sha256','--out','--audit'];
  for (const key of required) if (!options[key]) throw new Error(`Required argument: ${key}=VALUE`);
  if (options['--out'] === options['--audit']) throw new Error('Catalog output and audit must differ');
  // Keep downloaded/derived third-party data outside the Git source tree.
  const catalogPath=resolve(options['--out']), auditPath=resolve(options['--audit']);
  const repositoryRoot=resolve(dirname(fileURLToPath(import.meta.url)),'..');
  for (const outputPath of [catalogPath,auditPath]) {
    const p=relative(repositoryRoot,outputPath);
    if (p==='' || (p!=='..' && !p.startsWith('..'+sep) && !isAbsolute(p)))
      throw new Error('Generated catalog and audit must remain outside the Git repository');
  }
  if (catalogPath===auditPath) throw new Error('Catalog output and audit resolve to the same file');
  if (existsSync(catalogPath) || existsSync(auditPath)) throw new Error('Catalog or audit output already exists; no overwrite');
  const {catalog,audit} = buildHipparcosCatalog(readFileSync(resolve(options['--hip2'])),readFileSync(resolve(options['--supplements'])),{
    hip2Sha256:options['--hip2-sha256'],supplementsSha256:options['--supplements-sha256'],
    allowComplexSolutions:options['--allow-complex-solutions']==='true',
    minParallaxSignalToNoise:options['--min-parallax-snr']===undefined?5:numeric(options['--min-parallax-snr'],'minimum parallax SNR')});
  // Best-effort rollback: never leave a newly created catalog without its audit.
  // 'wx' remains authoritative against a concurrent writer; never delete
  // an existing audit or an output we did not successfully create ourselves.
  let catalogWritten=false;
  try {
    writeFileSync(catalogPath,JSON.stringify(catalog,null,2)+'\n',{flag:'wx',mode:0o600});
    catalogWritten=true;
    writeFileSync(auditPath,JSON.stringify(audit,null,2)+'\n',{flag:'wx',mode:0o600});
  } catch(error) {
    if(catalogWritten)try {unlinkSync(catalogPath);}catch {}
    throw error;
  }
  return {count:catalog.records.length,catalogId:catalog.id};
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {console.log(JSON.stringify(importHipparcos2007(process.argv.slice(2))));}
  catch (error) {console.error(error.message); process.exitCode=1;}
}
