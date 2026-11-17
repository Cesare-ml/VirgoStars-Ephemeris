// SPDX-License-Identifier: CPAL-1.0
// Re-examines existing caller-supplied diagnostics. Does not recalculate positions.
import {createHash} from 'node:crypto';
import {existsSync, readFileSync, writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

const targets=['Sun','Moon','Mercury','Venus','Mars Barycenter','Jupiter Barycenter',
  'Saturn Barycenter','Uranus Barycenter','Neptune Barycenter','Pluto Barycenter'];
const commands=['10','301','199','299','4','5','6','7','8','9'];
const years=Array.from({length:13},(_,i)=>i*250);
const sha=b=>createHash('sha256').update(b).digest('hex');
const max=(rows,key)=>rows.reduce((a,b)=>Math.abs(b[key])>Math.abs(a[key])?b:a);

export function summarizePlanetDiagnostics(icrfBytes,swissBytes,gridBytes){
  const icrf=JSON.parse(icrfBytes),swiss=JSON.parse(swissBytes),grid=JSON.parse(gridBytes);
  if(icrf.schema!=='virgostars-horizons-apparent-icrf/1'||swiss.schema!=='virgostars-swiss-long-range/1'
    ||grid.schema!=='virgostars-horizons-long-range-grid/1'||!(/^[a-f0-9]{64}$/).test(icrf.kernelSha256)
    ||swiss.kernelSha256!==icrf.kernelSha256||grid.kernelSha256!==icrf.kernelSha256
    ||!Array.isArray(icrf.rows)||icrf.rows.length!==10||!Array.isArray(grid.rows)||grid.rows.length!==10
    ||!Array.isArray(swiss.rows))throw new Error('incompatible diagnostic sources');
  function selected(records,keys){
    if(!Array.isArray(records))throw new Error('missing records');
    return years.map(year=>{
      const matches=records.filter(r=>r.year===year);
      if(matches.length!==1||!Number.isFinite(matches[0].jdTdb)
        ||keys.some(key=>!Number.isFinite(matches[0][key])))throw new Error('missing, duplicate or invalid sample');
      return matches[0];
    });
  }
  const positions=[];
  for(let i=0;i<targets.length;i++){
    const a=icrf.rows[i],g=grid.rows[i],name=targets[i],command=commands[i];
    for(const row of [a,g]){
      if(row?.name!==name||row.command!==command||typeof row.raw!=='string'
        ||!row.raw.includes(`Target body name: ${name} (${command})`)
        ||!/Target body name:.*\{source: DE441\}/.test(row.raw)
        ||!/Center body name: Earth \(399\).*\{source: DE441\}/.test(row.raw)
        ||!row.raw.includes('$$SOE')||!row.raw.includes('$$EOE'))throw new Error('invalid Horizons evidence');
    }
    const aa=selected(a.records,['differenceArcsec']);
    const gg=selected(g.records,['longitudeDifferenceArcsec','latitudeDifferenceArcsec',
      'longitudeRateDifferenceArcsecPerDay','latitudeRateDifferenceArcsecPerDay']);
    const ss=selected(swiss.rows.filter(r=>r.body===i),['longitudeDifferenceArcsec','latitudeDifferenceArcsec',
      'distanceDifferenceAu','engineLongitudeRateVsOwnFiniteArcsecPerDay','engineLatitudeRateVsOwnFiniteArcsecPerDay']);
    for(let j=0;j<years.length;j++){
      if(aa[j].jdTdb!==gg[j].jdTdb||aa[j].jdTdb!==ss[j].jdTdb||aa[j].differenceArcsec<0)
        throw new Error('inconsistent epochs or angular difference');
      positions.push({body:name,year:years[j],jdTdb:aa[j].jdTdb,
        horizonsIcrfArcsec:aa[j].differenceArcsec,
        horizonsLongitudeArcsec:gg[j].longitudeDifferenceArcsec,horizonsLatitudeArcsec:gg[j].latitudeDifferenceArcsec,
        swissLongitudeArcsec:ss[j].longitudeDifferenceArcsec,swissLatitudeArcsec:ss[j].latitudeDifferenceArcsec,
        swissDistanceAu:ss[j].distanceDifferenceAu,
        ownLongitudeRateArcsecPerDay:ss[j].engineLongitudeRateVsOwnFiniteArcsecPerDay,
        ownLatitudeRateArcsecPerDay:ss[j].engineLatitudeRateVsOwnFiniteArcsecPerDay});
    }
  }
  const metric=key=>{const r=max(positions,key);return {absolute:Math.abs(r[key]),body:r.body,year:r.year};};
  const perBody=targets.map(body=>{const r=max(positions.filter(p=>p.body===body),'horizonsIcrfArcsec');
    return {body,cases:years.length,maximumArcsec:r.horizonsIcrfArcsec,year:r.year};});
  return {schema:'virgostars-planet-validation-subset/1',status:'EXPLORATORY_NOT_RELEASE_VALIDATION',
    kernelSha256:icrf.kernelSha256,sourceHashes:{horizonsIcrfSha256:sha(icrfBytes),swissSha256:sha(swissBytes),horizonsGridSha256:sha(gridBytes)},
    years,cases:positions.length,perBody,maxima:{horizonsIcrfArcsec:metric('horizonsIcrfArcsec'),
      horizonsLongitudeArcsec:metric('horizonsLongitudeArcsec'),horizonsLatitudeArcsec:metric('horizonsLatitudeArcsec'),
      swissLongitudeArcsec:metric('swissLongitudeArcsec'),swissLatitudeArcsec:metric('swissLatitudeArcsec'),
      swissDistanceAu:metric('swissDistanceAu'),ownLongitudeRateArcsecPerDay:metric('ownLongitudeRateArcsecPerDay'),
      ownLatitudeRateArcsecPerDay:metric('ownLatitudeRateArcsecPerDay')},
    notes:'Summarizes stored differences, not fresh engine calculations or independently authenticated observations. TDB vs approximate TT. Swiss and Horizons comparisons share DE441; frame models differ.',positions};
}
function main(){
  const [a,b,c,out,...extra]=process.argv.slice(2);
  if(!a||!b||!c||!out||extra.length||new Set([a,b,c,out].map(p=>resolve(p))).size!==4||existsSync(out))
    throw new Error('usage: <horizons-icrf.json> <swiss.json> <horizons-grid.json> <new-output.json>');
  const inputs=[a,b,c].map(p=>readFileSync(p));
  if(inputs.some(b=>b.length>10_000_000))throw new Error('input too large');
  const result=summarizePlanetDiagnostics(...inputs);
  if([a,b,c].some((p,i)=>sha(readFileSync(p))!==sha(inputs[i])))throw new Error('input changed');
  writeFileSync(out,JSON.stringify(result,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({cases:result.cases,maxima:result.maxima,perBody:result.perBody}));
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href)
  try{main();}catch(error){console.error(error.message);process.exitCode=1;}
