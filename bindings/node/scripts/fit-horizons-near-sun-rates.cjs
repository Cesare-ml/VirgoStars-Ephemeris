'use strict';
// SPDX-License-Identifier: CPAL-1.0. Polynomial fits are exploratory, not error bounds.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
const {parseIcrfTable}=require('./horizons-icrf-parser.cjs');
const {unit,separation}=require('./compare-horizons-frame-isolation.cjs');
const R=Math.PI/180,AS=180/Math.PI*3600;
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const dot=(a,b)=>a.reduce((s,x,i)=>s+x*b[i],0);
// Modified Gram-Schmidt with a second orthogonalization pass. Scaled times
// are dimensionless [-1,1]; coefficient 1 divided by window is the rate.
function polynomialSlope(times,values,degree){
  if(!Array.isArray(times)||!Array.isArray(values)||times.length!==values.length||!Number.isInteger(degree)||degree<1||times.length<=degree||!times.every(Number.isFinite)||!values.every(Number.isFinite))throw new Error('invalid polynomial inputs');
  const n=times.length,window=Math.max(...times.map(Math.abs));if(!(window>0))throw new Error('zero window');
  const x=times.map(t=>t/window),q=[],upper=Array.from({length:degree+1},()=>Array(degree+1).fill(0));
  for(let j=0;j<=degree;j++){
    let v=x.map(z=>z**j);
    for(let pass=0;pass<2;pass++)for(let i=0;i<j;i++){const projection=dot(q[i],v);upper[i][j]+=projection;v=v.map((z,k)=>z-projection*q[i][k]);}
    const length=Math.hypot(...v);if(!Number.isFinite(length)||length<1e-11)throw new Error('singular polynomial');
    upper[j][j]=length;q.push(v.map(z=>z/length));
  }
  const rhs=q.map(col=>dot(col,values)),coeff=Array(degree+1).fill(0);
  for(let i=degree;i>=0;i--){let rest=rhs[i];for(let j=i+1;j<=degree;j++)rest-=upper[i][j]*coeff[j];coeff[i]=rest/upper[i][i];}
  const rms=Math.sqrt(values.reduce((s,y,k)=>{const p=coeff.reduceRight((v,c)=>v*x[k]+c,0);return s+(y-p)**2;},0)/n);
  return {rate:coeff[1]/window,rms,coefficients:coeff,window};
}
function heldoutRms(times,values,degree){
  if(!Array.isArray(times)||!Array.isArray(values)||times.length!==values.length||!times.every(Number.isFinite)||!values.every(Number.isFinite))throw new Error('invalid inputs');
  const trainT=times.filter((_,i)=>i%2===0),trainY=values.filter((_,i)=>i%2===0),fit=polynomialSlope(trainT,trainY,degree);
  const residual=times.map((t,i)=>({t,y:values[i],i})).filter(x=>x.i%2===1).map(({t,y})=>y-fit.coefficients.reduceRight((v,c)=>v*(t/fit.window)+c,0));
  if(!residual.length)throw new Error('empty heldout set');
  return {rate:fit.rate,rms:Math.sqrt(residual.reduce((s,x)=>s+x*x,0)/residual.length),count:residual.length};
}
function tangentOffsets(records,center){
  const a=center.raDeg*R,d=center.decDeg*R,e=[-Math.sin(a),Math.cos(a),0],n=[-Math.sin(d)*Math.cos(a),-Math.sin(d)*Math.sin(a),Math.cos(d)],u0=unit(center.raDeg,center.decDeg);
  return records.map(r=>{const u=unit(r.raDeg,r.decDeg),delta=u.map((x,i)=>x-u0[i]);return [dot(delta,e)*AS,dot(delta,n)*AS];});
}
const tdbToTt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*R)/86400;
async function main(){
  const [kernel,kernelHash,source,sourceHash,executable,out,...extra]=process.argv.slice(2);
  if(!kernel||!source||!executable||!out||extra.length||![kernelHash,sourceHash].every(x=>/^[a-f0-9]{64}$/.test(x||''))||fs.existsSync(out)||new Set([kernel,source,executable,out].map(x=>path.resolve(x))).size!==4)throw new Error('usage: <de441> <sha256> <prior-rate-report> <sha256> <executable> <new-report>');
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input hash mismatch');
  const prior=JSON.parse(fs.readFileSync(source));if(prior.schema!=='horizons-near-sun-position-rates/1'||prior.kernelSha256!==kernelHash||!Array.isArray(prior.rows)||prior.rows.length!==14)throw new Error('invalid source');
  const allowed=new Map([[2,['199','Mercury']],[3,['299','Venus']],[4,['4','Mars Barycenter']],[5,['5','Jupiter Barycenter']],[6,['6','Saturn Barycenter']],[7,['7','Uranus Barycenter']],[8,['8','Neptune Barycenter']]]);
  for(const r of prior.rows){const a=allowed.get(r.id);if(!a||r.command!==a[0]||r.name!==a[1]||!Number.isFinite(r.jd)||!Number.isFinite(r.year))throw new Error('invalid case');}
  const input=prior.rows.map(r=>`${r.id},${r.jd}`).join('\n')+'\n',lines=execFileSync(path.resolve(executable),[path.resolve(kernel),'--apparent-state'],{input,encoding:'utf8',maxBuffer:1048576,timeout:120000}).trim().split(/\r?\n/);
  const header='body,jd_tdb,elongation_deg,denominator,ra_deg,dec_deg,old_ra_cosdec_as_day,old_dec_as_day,candidate_ra_cosdec_as_day,candidate_dec_as_day';
  if(lines.shift()!==header||lines.length!==prior.rows.length)throw new Error('invalid diagnostic output');
  const local=lines.map((line,i)=>{const c=line.split(',');if(c.length!==10||!c.every(x=>x!==''&&Number.isFinite(Number(x))))throw new Error('invalid diagnostic row');const v=c.map(Number);if(v[0]!==prior.rows[i].id||v[1]!==prior.rows[i].jd||v[3]<=1e-5)throw new Error('mismatched diagnostic row');return v;});
  const rows=[],requests=[];
  for(let i=0;i<prior.rows.length;i++){
    const item=prior.rows[i],v=local[i],offsets=Array.from({length:61},(_,k)=>(k-30)/256),epochs=offsets.map(t=>tdbToTt(item.jd+t));
    const q=new URLSearchParams({format:'json',COMMAND:`'${item.command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'45'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
    const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${q}`;let response;for(let n=0;n<3;n++){response=await fetch(url,{signal:AbortSignal.timeout(30000)});if(response.ok||![429,502,503,504].includes(response.status))break;if(n<2)await new Promise(r=>setTimeout(r,1500*(n+1)));}
    if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);const raw=(await response.json()).result;if(!raw.includes('Atmos refraction: NO (AIRLESS)'))throw new Error('unexpected refraction');
    const ref=parseIcrfTable(raw,item.name,item.command,epochs),center=ref[30],delta=tangentOffsets(ref,center);
    const fit=(slice,degree)=>{const t=offsets.slice(...slice),points=delta.slice(...slice),ra=polynomialSlope(t,points.map(x=>x[0]),degree),dec=polynomialSlope(t,points.map(x=>x[1]),degree);return {rate:[ra.rate,dec.rate],rmsArcsec:Math.hypot(ra.rms,dec.rms)};};
    const wide5=fit([0,61],5),wide7=fit([0,61],7),narrow5=fit([15,46],5),narrow7=fit([15,46],7),old=[v[6],v[7]],candidate=[v[8],v[9]],dist=(a,b)=>Math.hypot(a[0]-b[0],a[1]-b[1]);
    const spread=Math.max(...[wide7,narrow5,narrow7].map(x=>dist(x.rate,wide5.rate)));
    rows.push({id:item.id,name:item.name,year:item.year,jd:item.jd,elongationDeg:v[2],denominator:v[3],positionSeparationArcsec:separation(unit(v[4],v[5]),unit(center.raDeg,center.decDeg)),old,candidate,wide5,wide7,narrow5,narrow7,fitSpread:spread,oldResidual:dist(old,wide5.rate),candidateResidual:dist(candidate,wide5.rate)});
    requests.push({name:item.name,year:item.year,url,raw});
  }
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input changed');
  const max=k=>Math.max(...rows.map(r=>r[k]));
  const summary={cases:rows.length,bodies:new Set(rows.map(r=>r.id)).size,oldMax:max('oldResidual'),candidateMax:max('candidateResidual'),fitSpreadMax:max('fitSpread'),candidateSmaller:rows.filter(r=>r.candidateResidual<r.oldResidual).length};
  fs.writeFileSync(out,JSON.stringify({schema:'horizons-near-sun-polynomial-rates/1',status:'EXPLORATORY',kernelSha256:kernelHash,sourceSha256:sourceHash,notes:'Apparent ICRF position-polynomial rates at TT samples (approximate TDB); two correlated windows and degrees; shared DE441. Fit spread is not an uncertainty or accuracy bound.',summary,rows,requests},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({summary,rows}));
}
module.exports={polynomialSlope,tangentOffsets,heldoutRms};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
