'use strict';
// SPDX-License-Identifier: CPAL-1.0. Small-rotation diagnostic only.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
const {parse}=require('./compare-horizons-orientation-components.cjs');
const {unit,separation}=require('./compare-horizons-frame-isolation.cjs');
const AS=180/Math.PI*3600;
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const dot=(a,b)=>a.reduce((s,x,i)=>s+x*b[i],0);
const cross=(a,b)=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
function fitSmallRotation(from,to){
  if(!Array.isArray(from)||!Array.isArray(to)||from.length!==to.length||from.length<2||![...from,...to].every(v=>Array.isArray(v)&&v.length===3&&v.every(Number.isFinite)&&Math.abs(Math.hypot(...v)-1)<1e-9))throw new Error('invalid unit directions');
  const m=Array.from({length:3},()=>[0,0,0]),b=[0,0,0];
  for(let k=0;k<from.length;k++){const u=from[k],v=to[k],z=cross(u,v);for(let i=0;i<3;i++){b[i]+=z[i];for(let j=0;j<3;j++)m[i][j]+=(i===j?1:0)-u[i]*u[j];}}
  const a=m.map((row,i)=>[...row,b[i]]),pivots=[];
  for(let i=0;i<3;i++){let p=i;for(let j=i+1;j<3;j++)if(Math.abs(a[j][i])>Math.abs(a[p][i]))p=j;[a[i],a[p]]=[a[p],a[i]];const d=a[i][i];if(Math.abs(d)<1e-10)throw new Error('ill-conditioned directions');pivots.push(Math.abs(d));for(let j=i;j<4;j++)a[i][j]/=d;for(let k=0;k<3;k++)if(k!==i){const f=a[k][i];for(let j=i;j<4;j++)a[k][j]-=f*a[i][j];}}
  const omega=a.map(row=>row[3]),theta=Math.hypot(...omega),axis=theta?omega.map(x=>x/theta):[1,0,0];
  const residual=from.map((u,i)=>{const q=cross(axis,u),d=dot(axis,u),c=Math.cos(theta),s=Math.sin(theta),rot=u.map((x,j)=>x*c+q[j]*s+axis[j]*d*(1-c));return separation(rot,to[i]);});
  return {rotationArcsec:omega.map(x=>x*AS),magnitudeArcsec:theta*AS,rmsResidualArcsec:Math.sqrt(residual.reduce((s,x)=>s+x*x,0)/residual.length),minPivot:Math.min(...pivots),maxPivot:Math.max(...pivots)};
}
async function main(){
  const [source,sourceHash,executable,out,...extra]=process.argv.slice(2);
  if(!source||!/^[a-f0-9]{64}$/.test(sourceHash||'')||!executable||!out||extra.length||fs.existsSync(out)||new Set([source,executable,out].map(x=>path.resolve(x))).size!==3)throw new Error('usage: <orientation-report> <sha256> <projection-executable> <new-report>');
  if(await sha(source)!==sourceHash)throw new Error('input hash mismatch');const prior=JSON.parse(fs.readFileSync(source));
  if(prior.schema!=='horizons-orientation-components/1'||!Array.isArray(prior.rows)||prior.rows.length!==21||!Array.isArray(prior.responses)||prior.responses.length!==3)throw new Error('invalid source');
  const groups=[['Sun','10'],['Moon','301'],['Venus','299']].map(([name,id])=>{const response=prior.responses.find(r=>r.name===name&&r.id===id),rows=prior.rows.filter(r=>r.body===name);if(!response||rows.length!==7||!rows.every(r=>Number.isFinite(r.jdTt)&&Number.isFinite(r.year)))throw new Error('invalid group');return {name,rows,ref:parse(response.raw,name,id,rows.map(r=>r.jdTt))};});
  for(let i=0;i<7;i++)if(!groups.every(g=>g.rows[i].jdTt===groups[0].rows[i].jdTt&&g.rows[i].year===groups[0].rows[i].year))throw new Error('unaligned epochs');
  const input=groups.flatMap(g=>g.ref.map((r,i)=>`${g.rows[i].jdTt},${r.raDeg},${r.decDeg}`)).join('\n')+'\n';
  const lines=execFileSync(path.resolve(executable),[],{input,encoding:'utf8',maxBuffer:1048576,timeout:30000}).trim().split(/\r?\n/);if(lines.length!==21)throw new Error('wrong projection count');
  const projected=lines.map(line=>{const f=line.split(',');if(f.length!==10||!f.every(x=>x!==''&&Number.isFinite(Number(x))))throw new Error('invalid projection');return f.map(Number);});
  const rows=groups[0].rows.map((record,i)=>{const horizon=groups.map(g=>unit(g.ref[i].ra,g.ref[i].dec)),mean=groups.map((g,b)=>unit(projected[b*7+i][0],projected[b*7+i][1])),actual=groups.map((g,b)=>unit(projected[b*7+i][2],projected[b*7+i][3]));
    const meanFit=fitSmallRotation(mean,horizon),trueFit=fitSmallRotation(actual,horizon);
    return {year:record.year,jdTt:record.jdTt,meanFit,trueFit,fitDifferenceArcsec:Math.hypot(...meanFit.rotationArcsec.map((x,j)=>x-trueFit.rotationArcsec[j]))};});
  if(await sha(source)!==sourceHash)throw new Error('source changed');
  fs.writeFileSync(out,JSON.stringify({schema:'horizons-equatorial-frame-fit/1',status:'EXPLORATORY',sourceSha256:sourceHash,notes:'Least-squares first-order rotation from local mean or true apparent equatorial axes to Horizons true equatorial coordinates. Three same-epoch directions, with conditioning and exact-rotation residual diagnostics. Mean fit still includes Horizons nutation; not a pure precession error.',rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(rows));
}
module.exports={fitSmallRotation};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
