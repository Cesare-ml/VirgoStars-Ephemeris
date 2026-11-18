'use strict';
// SPDX-License-Identifier: CPAL-1.0. Exploratory independent-implementation comparison.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
const {parseIcrfTable}=require('./horizons-icrf-parser.cjs');
const {unit,separation}=require('./compare-horizons-frame-isolation.cjs');
const rad=Math.PI/180,as=180/Math.PI*3600;
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
function fourthRate(v,h){if(!Array.isArray(v)||v.length!==4||!Number.isFinite(h)||h<=0||!v.every(x=>Array.isArray(x)&&x.length===3&&x.every(Number.isFinite)))throw new Error('invalid stencil');return [0,1,2].map(i=>(v[0][i]-8*v[1][i]+8*v[2][i]-v[3][i])/(12*h));}
function tangentRates(v,ra,dec){const a=ra*rad,d=dec*rad,e=[-Math.sin(a),Math.cos(a),0],n=[-Math.sin(d)*Math.cos(a),-Math.sin(d)*Math.sin(a),Math.cos(d)];return [v.reduce((s,x,i)=>s+x*e[i],0)*as,v.reduce((s,x,i)=>s+x*n[i],0)*as];}
const bodies=[[2,'199','Mercury'],[3,'299','Venus'],[4,'4','Mars Barycenter'],[5,'5','Jupiter Barycenter'],[6,'6','Saturn Barycenter'],[7,'7','Uranus Barycenter'],[8,'8','Neptune Barycenter']];
const tdbToTt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*rad)/86400;
async function main(){
  const [kernel,kernelHash,source,sourceHash,executable,out,scale='fine',...extra]=process.argv.slice(2);
  if(!kernel||!source||!executable||!out||extra.length||!['fine','coarse'].includes(scale)||![kernelHash,sourceHash].every(x=>/^[a-f0-9]{64}$/.test(x||''))||fs.existsSync(out)||new Set([kernel,source,executable,out].map(x=>path.resolve(x))).size!==4)throw new Error('usage: <de441> <sha256> <case-report> <sha256> <executable> <new-report> [fine|coarse]');
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input hash mismatch');
  const data=JSON.parse(fs.readFileSync(source));if(data.schema!=='solar-deflection-rate-candidates/1'||data.kernelSha256!==kernelHash||!Array.isArray(data.rows))throw new Error('invalid case report');
  const selected=[];
  for(const [id,command,name] of bodies){for(const year of [-8750,2000]){
    const matches=data.rows.filter(r=>r.id===id&&r.year===year&&Number.isFinite(r.jd)&&Number.isFinite(r.elongationDeg)&&r.elongationDeg>=0.35&&r.elongationDeg<=5&&r.denominator>1e-5).sort((a,b)=>a.elongationDeg-b.elongationDeg||a.jd-b.jd);
    if(matches.length)selected.push({id,command,name,year,jd:matches[0].jd,selectionElongationDeg:matches[0].elongationDeg});
  }}
  if(!selected.length)throw new Error('empty selection');
  const input=selected.map(r=>`${r.id},${r.jd}`).join('\n')+'\n';
  const lines=execFileSync(path.resolve(executable),[path.resolve(kernel),'--apparent-state'],{input,encoding:'utf8',maxBuffer:1048576,timeout:120000}).trim().split(/\r?\n/);
  const header='body,jd_tdb,elongation_deg,denominator,ra_deg,dec_deg,old_ra_cosdec_as_day,old_dec_as_day,candidate_ra_cosdec_as_day,candidate_dec_as_day';
  if(lines.shift()!==header||lines.length!==selected.length)throw new Error('invalid candidate output');
  const local=lines.map((line,i)=>{const cells=line.split(',');if(cells.length!==10||!cells.every(x=>x!==''&&Number.isFinite(Number(x))))throw new Error('invalid candidate row');const f=cells.map(Number);if(f[0]!==selected[i].id||f[1]!==selected[i].jd||f[3]<=1e-5)throw new Error('invalid candidate row');return f;});
  const requests=[],rows=[];
  for(let i=0;i<selected.length;i++){
    const item=selected[i],h=scale==='fine'?1/64:1/16,k=h/2,dt=[-2*h,-h,-k,0,k,h,2*h],tdb=dt.map(x=>item.jd+x),tt=tdb.map(tdbToTt);
    const q=new URLSearchParams({format:'json',COMMAND:`'${item.command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",TLIST:`'${tt.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'45'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
    const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${q}`;let response;
    for(let n=0;n<3;n++){response=await fetch(url,{signal:AbortSignal.timeout(30000)});if(response.ok||![429,502,503,504].includes(response.status))break;if(n<2)await new Promise(r=>setTimeout(r,1500*(n+1)));}
    if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);const raw=(await response.json()).result;if(!raw.includes('Atmos refraction: NO (AIRLESS)'))throw new Error('unexpected refraction');
    const refs=parseIcrfTable(raw,item.name,item.command,tt),vectors=refs.map(r=>unit(r.raDeg,r.decDeg)),center=refs[3];
    const coarse=tangentRates(fourthRate([vectors[0],vectors[1],vectors[5],vectors[6]],h),center.raDeg,center.decDeg);
    const fine=tangentRates(fourthRate([vectors[1],vectors[2],vectors[4],vectors[5]],k),center.raDeg,center.decDeg);
    const loc=local[i],old=[loc[6],loc[7]],candidate=[loc[8],loc[9]],dist=(a,b)=>Math.hypot(a[0]-b[0],a[1]-b[1]);
    rows.push({...item,elongationDeg:loc[2],denominator:loc[3],positionSeparationArcsec:separation(unit(loc[4],loc[5]),vectors[3]),horizonsCoarse:coarse,horizonsFine:fine,old,candidate,oldResidual:dist(old,coarse),candidateResidual:dist(candidate,coarse),referenceDisagreement:dist(coarse,fine)});
    requests.push({name:item.name,year:item.year,url,raw});
  }
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input changed');
  fs.writeFileSync(out,JSON.stringify({schema:'horizons-near-sun-position-rates/1',scale,status:'EXPLORATORY',kernelSha256:kernelHash,sourceSha256:sourceHash,notes:'Horizons apparent ICRF position derivatives vs local candidate/current rates; DE441 shared; TT/TDB approximation, rounding, finite-difference steps and near-limb model limit inference. No physical accuracy bound.',rows,requests},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({cases:rows.length,bodies:[...new Set(rows.map(r=>r.name))],maxOld:Math.max(...rows.map(r=>r.oldResidual)),maxCandidate:Math.max(...rows.map(r=>r.candidateResidual)),maxReference:Math.max(...rows.map(r=>r.referenceDisagreement)),improved:rows.filter(r=>r.candidateResidual<r.oldResidual).length,rows}));
}
module.exports={fourthRate,tangentRates};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
