'use strict';
// SPDX-License-Identifier: CPAL-1.0. Offline numerical experiment only.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
const {Engine,julday}=require('../index.js');
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
function angle(a,b){const d=a.reduce((s,x,i)=>s+x*b[i],0),na=Math.hypot(...a),nb=Math.hypot(...b);return Math.acos(Math.max(-1,Math.min(1,d/na/nb)))*180/Math.PI;}
async function main(){
  const [kernel,expected,executable,out,grid='baseline',...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!executable||!out||extra.length||!['baseline','offset'].includes(grid)||fs.existsSync(out)||new Set([kernel,executable,out].map(x=>path.resolve(x))).size!==3)
    throw new Error('usage: <de441> <sha256> <diagnostic-executable> <new-report> [baseline|offset]');
  if(await sha(kernel)!==expected)throw new Error('kernel hash mismatch');
  const engine=new Engine(path.resolve(kernel)), bins=[0,1,5,30,90], cases=[];
  // One geometric sample per day over two years; bins choose examples rather
  // than precisely locating a conjunction or proving continuous coverage.
  for(const year of [-12999,-8750,2000]) {
    const best=Array.from({length:8},()=>bins.map(()=>null)),start=julday(year,1,1,12,true);
    for(let d=0;d<730;d++){
      const jd=start+d,sun=engine.geometricStateTdb(jd,0).positionKm;
      for(let id=2;id<=9;id++){
        const a=angle(engine.geometricStateTdb(jd,id).positionKm,sun);
        for(let i=0;i<bins.length;i++)if(best[id-2][i]===null||Math.abs(a-bins[i])<best[id-2][i].distance)
          best[id-2][i]={id,jd,year,nominalDeg:bins[i],distance:Math.abs(a-bins[i]),geometricElongationDeg:a};
      }
    }
    cases.push(...best.flat());
  }
  cases.push({id:7,jd:-1474538,year:-8750,nominalDeg:0.267,distance:null,geometricElongationDeg:null});
  const selected=grid==='baseline'?cases:cases.flatMap(c=>[-0.125,0.125].map(offset=>({...c,jd:c.jd+offset,offsetDays:offset})));
  const input=selected.map(x=>`${x.id},${x.jd}`).join('\n')+'\n';
  const text=execFileSync(path.resolve(executable),[path.resolve(kernel)],{input,encoding:'utf8',maxBuffer:1024*1024,timeout:120000});
  const lines=text.trim().split(/\r?\n/),header='body,jd_tdb,elongation_deg,denominator,crosses_floor,old_error_as_per_day,analytic_error_as_per_day,reference_disagreement_as_per_day';
  if(lines.shift()!==header||lines.length!==selected.length)throw new Error('unexpected diagnostic output');
  const rows=lines.map((line,i)=>{
    const f=line.split(',');if(f.length!==8||Number(f[0])!==selected[i].id||Number(f[1])!==selected[i].jd||!['true','false'].includes(f[4])||![f[2],f[3],f[5],f[6],f[7]].every(x=>Number.isFinite(Number(x))))throw new Error(`invalid row ${i}`);
    return {...selected[i],elongationDeg:Number(f[2]),denominator:Number(f[3]),crossesFloor:f[4]==='true',oldError:Number(f[5]),analyticError:Number(f[6]),referenceDisagreement:Number(f[7])};
  });
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  fs.writeFileSync(out,JSON.stringify({schema:'solar-deflection-rate-candidates/1',grid,kernelSha256:expected,status:'EXPLORATORY',notes:'Geometric grid selects examples; errors are vector norms of delta-rate minus a finer numerical derivative of the SAME model in fixed ICRF arcseconds/day. Floor crossings and reference disagreement limit interpretation. Not independent physical accuracy.',rows},null,2)+'\n',{flag:'wx'});
  const valid=rows.filter(x=>!x.crossesFloor);const max=(arr,key)=>Math.max(...arr.map(x=>x[key]));
  console.log(JSON.stringify({cases:rows.length,valid:valid.length,crossing:rows.length-valid.length,oldMax:max(valid,'oldError'),analyticMax:max(valid,'analyticError'),referenceMax:max(valid,'referenceDisagreement'),improved:valid.filter(x=>x.analyticError<x.oldError).length}));
}
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
