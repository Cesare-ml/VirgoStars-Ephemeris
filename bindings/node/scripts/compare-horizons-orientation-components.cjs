'use strict';
// SPDX-License-Identifier: CPAL-1.0. Experimental orientation diagnostics.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
const {parseIcrfTable}=require('./horizons-icrf-parser.cjs');
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const rad=Math.PI/180,wrap=x=>((x%360)+540)%360-180;
function unit(lon,lat){const a=lon*rad,b=lat*rad;return [Math.cos(b)*Math.cos(a),Math.cos(b)*Math.sin(a),Math.sin(b)];}
function effectiveObliquity(ra,dec,lon,lat){
  const a=unit(ra,dec),b=unit(lon,lat),n=a[1]**2+a[2]**2;
  if(n<1e-8)throw new Error('ill-conditioned obliquity');
  return {degrees:Math.atan2(a[2]*b[1]-a[1]*b[2],a[1]*b[1]+a[2]*b[2])/rad,xMismatchArcsec:(a[0]-b[0])/rad*3600};
}
function parse(raw,name,command,epochs){
  if(!raw.includes('Atmos refraction: NO (AIRLESS)'))throw new Error('unexpected refraction');
  const icrf=parseIcrfTable(raw,name,command,epochs),head=raw.split(/\r?\n/).find(x=>x.includes('R.A.__(a-app)')&&x.includes('ObsEcLon'));
  const start=raw.indexOf('$$SOE'),end=raw.indexOf('$$EOE');
  if(!head||start<0||end<=start||raw.indexOf('$$SOE',start+5)!==-1||raw.indexOf('$$EOE',end+5)!==-1)throw new Error('invalid table');
  const cols=head.split(',').map(x=>x.trim()),idx=['R.A.__(a-app)','DEC___(a-app)','ObsEcLon','ObsEcLat'].map(x=>cols.indexOf(x));
  if(idx.some(x=>x<0))throw new Error('missing columns');
  const lines=raw.slice(start+5,end).trim().split(/\r?\n/);if(lines.length!==epochs.length)throw new Error('wrong count');
  return lines.map((line,i)=>{const f=line.split(',').map(x=>x.trim());if(!idx.every(j=>f[j]!==''&&Number.isFinite(Number(f[j]))))throw new Error('invalid coordinate');const [ra,dec,lon,lat]=idx.map(j=>Number(f[j]));return {ra,dec,lon,lat,...icrf[i],effective:effectiveObliquity(ra,dec,lon,lat)};});
}
async function main(){
  const [source,sourceHash,executable,out,...extra]=process.argv.slice(2);
  if(!source||!/^[a-f0-9]{64}$/.test(sourceHash||'')||!executable||!out||extra.length||fs.existsSync(out)||new Set([source,executable,out].map(x=>path.resolve(x))).size!==3)throw new Error('usage: <prior-ecliptic-report> <sha256> <projection-executable> <new-report>');
  if(await sha(source)!==sourceHash)throw new Error('source hash mismatch');
  const prior=JSON.parse(fs.readFileSync(source));
  if(prior.schema!=='horizons-ancient-ecliptic-conventions/1'||!Array.isArray(prior.rows)||prior.rows.length!==7||!prior.rows.every(r=>Number.isFinite(r.jdTt)&&Number.isFinite(r.year)))throw new Error('invalid source');
  const epochs=prior.rows.map(r=>r.jdTt),bodies=[['Sun','10'],['Moon','301'],['Venus','299']];
  const responses=[];
  for(const [name,id] of bodies){
    const q=new URLSearchParams({format:'json',COMMAND:`'${id}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'2,31,45'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
    const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${q}`;let response;
    for(let attempt=0;attempt<3;attempt++){
      response=await fetch(url,{signal:AbortSignal.timeout(30000)});
      if(response.ok||![429,502,503,504].includes(response.status))break;
      if(attempt<2)await new Promise(resolve=>setTimeout(resolve,1500*(attempt+1)));
    }
    if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);const raw=(await response.json()).result;
    responses.push({name,id,url,raw,coordinates:parse(raw,name,id,epochs)});
  }
  const input=responses.flatMap(r=>r.coordinates.map((c,i)=>`${epochs[i]},${c.raDeg},${c.decDeg}`)).join('\n')+'\n';
  const lines=execFileSync(path.resolve(executable),[],{input,encoding:'utf8',maxBuffer:1048576,timeout:30000}).trim().split(/\r?\n/);
  if(lines.length!==responses.length*epochs.length)throw new Error('wrong projection count');
  const rows=responses.flatMap((r,bi)=>r.coordinates.map((ref,i)=>{
    const f=lines[bi*epochs.length+i].split(',').map(Number);if(f.length!==10||!f.every(Number.isFinite))throw new Error('invalid projection');
    return {body:r.name,year:prior.rows[i].year,jdTt:epochs[i],equatorRaCosDecArcsec:wrap(f[2]-ref.ra)*Math.cos(ref.dec*rad)*3600,equatorDecArcsec:(f[3]-ref.dec)*3600,poleLonArcsec:wrap(f[4]-ref.lon)*3600,poleLatArcsec:(f[5]-ref.lat)*3600,directLonArcsec:wrap(f[6]-ref.lon)*3600,directLatArcsec:(f[7]-ref.lat)*3600,horizonsEffectiveObliquityDeg:ref.effective.degrees,horizonsXMismatchArcsec:ref.effective.xMismatchArcsec,poleTrueObliquityMinusHorizonsArcsec:(f[8]-ref.effective.degrees)*3600,directTrueObliquityMinusHorizonsArcsec:(f[9]-ref.effective.degrees)*3600};
  }));
  if(await sha(source)!==sourceHash)throw new Error('source changed');
  fs.writeFileSync(out,JSON.stringify({schema:'horizons-orientation-components/1',status:'EXPLORATORY',sourceSha256:sourceHash,notes:'Same Horizons apparent ICRF vector projected through local experimental Vondrak/nutation axes and compared to Horizons equatorial/ecliptic of date. Effective obliquity is inferred from each pair assuming a common equinox; x mismatch checks that assumption. Not independent truth or isolated precession error.',responses,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(rows));
}
module.exports={unit,effectiveObliquity,parse};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
