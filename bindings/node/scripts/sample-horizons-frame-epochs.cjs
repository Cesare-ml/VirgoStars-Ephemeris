'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Independent implementation comparison, not independent Earth orientation truth.
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { execFileSync } = require('node:child_process');
const { julday } = require('../index.js');
const { parse } = require('./compare-horizons-orientation-components.cjs');
const { unit, separation } = require('./compare-horizons-frame-isolation.cjs');
const { fitSmallRotation } = require('./analyze-horizons-frame-rotation.cjs');
// January of astronomical year -9998 is before the Horizons DE441 service
// limit when converted from proleptic Gregorian dates; start one year later.
const years = [-9997, -9000, -8000, -7000, -6000, -5000, -4000, -3000, -2000, -1000, 0, 1000, 2000, 3000, 3999];
const bodies = [['Sun','10'], ['Moon','301'], ['Venus','299'], ['Mars Barycenter','4'], ['Jupiter Barycenter','5'], ['Saturn Barycenter','6']];
const train = [0,2,4,5], heldout = [1,3];
function rotate(v, arcsec) {
  if (!Array.isArray(v) || v.length !== 3 || !v.every(Number.isFinite) || Math.abs(Math.hypot(...v)-1) > 1e-9 || !Array.isArray(arcsec) || arcsec.length !== 3 || !arcsec.every(Number.isFinite)) throw new Error('invalid rotation');
  const w = arcsec.map(x => x * Math.PI / (180 * 3600)), angle = Math.hypot(...w);
  if (!angle) return [...v];
  const [x,y,z] = w.map(x => x/angle), dot = x*v[0]+y*v[1]+z*v[2], c = Math.cos(angle), s = Math.sin(angle);
  return [v[0]*c+(y*v[2]-z*v[1])*s+x*dot*(1-c), v[1]*c+(z*v[0]-x*v[2])*s+y*dot*(1-c), v[2]*c+(x*v[1]-y*v[0])*s+z*dot*(1-c)];
}
function fitRotation(from, to) {
  if (!Array.isArray(from) || !Array.isArray(to) || from.length !== to.length || from.length<2 || ![...from,...to].every(v=>Array.isArray(v) && v.length===3 && v.every(Number.isFinite) && Math.abs(Math.hypot(...v)-1)<1e-9)) throw new Error('expected unit directions');
  if (from.some((v,i)=>separation(v,to[i])>3600)) throw new Error('directions exceed small-rotation diagnostic range');
  let current = from.map(v=>[...v]);
  const target = to, steps = [], pivots = [];
  let converged = false;
  // Refit after each exact Rodrigues rotation: at 200 arcseconds a single
  // linearized fit has second-order residuals large enough to bias this audit.
  for (let i=0;i<6;i++) {
    const fit = fitSmallRotation(current, target);
    steps.push(fit.rotationArcsec); pivots.push(fit.minPivot);
    current = current.map(v=>rotate(v,fit.rotationArcsec));
    if (Math.hypot(...fit.rotationArcsec)<1e-10) { converged=true; break; }
  }
  if (!converged) throw new Error('rotation fit did not converge');
  const apply = v => steps.reduce((p, step)=>rotate(p,step),v);
  // Recover the rotation vector from the composed rotation, rather than
  // adding increments (rotations about different axes do not commute).
  const cols=[[1,0,0],[0,1,0],[0,0,1]].map(apply);
  const skew=[cols[1][2]-cols[2][1],cols[2][0]-cols[0][2],cols[0][1]-cols[1][0]].map(x=>x/2);
  const sine=Math.hypot(...skew), cosine=(cols[0][0]+cols[1][1]+cols[2][2]-1)/2;
  const scale=sine ? Math.atan2(sine,cosine)/sine*180/Math.PI*3600 : 0;
  const rotationArcsec=skew.map(x=>x*scale);
  const trainingResiduals=current.map((v,i)=>separation(v,target[i]));
  return { rotationArcsec, magnitudeArcsec: Math.hypot(...rotationArcsec),
    rmsResidualArcsec: Math.hypot(...trainingResiduals)/Math.sqrt(trainingResiduals.length),
    minPivot: Math.min(...pivots), iterations: steps.length, stepsArcsec: steps };
}
function heldoutFit(from, to) {
  if (!Array.isArray(from) || !Array.isArray(to) || from.length !== 6 || to.length !== 6 || ![...from,...to].every(v=>Array.isArray(v) && v.length===3 && v.every(Number.isFinite) && Math.abs(Math.hypot(...v)-1)<1e-9)) throw new Error('expected six unit directions');
  const fit=fitRotation(train.map(i=>from[i]),train.map(i=>to[i]));
  const residuals = heldout.map(i => separation(fit.stepsArcsec.reduce((p,step)=>rotate(p,step),from[i]), to[i]));
  return { ...fit,
    heldoutResidualArcsec: residuals, heldoutRmsArcsec: Math.hypot(...residuals)/Math.sqrt(residuals.length) };
}
function epochGrid() { return years.flatMap(year => [1,4,7,10].map(month => ({ year, month, jdTt: julday(year,month,1,12,true) }))); }
function summarize(rows) {
  if (!Array.isArray(rows) || !rows.length) throw new Error('empty epochs');
  return years.map(year => {
    const group = rows.filter(r => r.year === year);
    if (group.length !== 4) throw new Error(`wrong count for ${year}`);
    const mean = [0,1,2].map(i => group.reduce((a,r) => a+r.trueFit.rotationArcsec[i],0)/group.length);
    return { year, trueMeanMagnitudeArcsec: Math.hypot(...mean),
      maxSeasonalDeviationArcsec: Math.max(...group.map(r=>Math.hypot(...r.trueFit.rotationArcsec.map((x,i)=>x-mean[i])))),
      maxTrainingRmsArcsec: Math.max(...group.map(r=>r.trueFit.rmsResidualArcsec)),
      maxHeldoutRmsArcsec: Math.max(...group.map(r=>r.trueFit.heldoutRmsArcsec)),
      minPivot: Math.min(...group.map(r=>r.trueFit.minPivot)),
      maxPoleEclipticArcsec: Math.max(...group.flatMap(r=>r.ecliptic.map(x=>x.poleSeparationArcsec))),
      maxDirectEclipticArcsec: Math.max(...group.flatMap(r=>r.ecliptic.map(x=>x.directSeparationArcsec))) };
  });
}
async function main() {
  const [executable, output, source, sourceHash, ...extra] = process.argv.slice(2);
  if (!executable || !output || extra.length || Boolean(source)!==Boolean(sourceHash) || (source && !/^[a-f0-9]{64}$/.test(sourceHash)) || fs.existsSync(output) || new Set([executable,output,...(source?[source]:[])].map(p=>path.resolve(p))).size !== (source?3:2)) throw new Error('usage: <projection-executable> <new-report> [prior-report prior-sha256]');
  const digest = file => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
  if (source && digest(source)!==sourceHash) throw new Error('prior report hash mismatch');
  const exeHash = crypto.createHash('sha256').update(fs.readFileSync(executable)).digest('hex');
  const grid = epochGrid(), epochs = grid.map(r=>r.jdTt), responses=[];
  const prior=source?JSON.parse(fs.readFileSync(source,'utf8')):null;
  if (prior && (prior.schema!=='horizons-frame-epoch-holdout/1' || !Array.isArray(prior.responses) || prior.responses.length!==bodies.length || !bodies.every(([name,id])=>prior.responses.filter(r=>r.name===name && r.id===id && Array.isArray(r.parts) && r.parts.length===2).length===1))) throw new Error('invalid prior report');
  // Keep TLIST well below URI limits while validating every returned epoch.
  for (const [name,id] of bodies) {
    const parts=[];
    for (let start=0;start<epochs.length;start+=30) {
      const batch=epochs.slice(start,start+30);
      if (prior) {
        const entry=prior.responses.find(r=>r.name===name && r.id===id);
        const old=entry?.parts?.[start/30];
        if (!old || typeof old.url!=='string' || typeof old.raw!=='string') throw new Error(`missing prior response: ${name}`);
        parts.push({url:old.url,raw:old.raw,coordinates:parse(old.raw,name,id,batch)});
        continue;
      }
      const q=new URLSearchParams({format:'json',COMMAND:`'${id}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",TLIST:`'${batch.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'2,31,45'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
      const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${q}`;
      let response;
      for (let attempt=0;attempt<3;attempt++) {
        response=await fetch(url,{signal:AbortSignal.timeout(30000)});
        if (response.ok || ![429,502,503,504].includes(response.status)) break;
        if (attempt<2) await new Promise(resolve=>setTimeout(resolve,1500*(attempt+1)));
      }
      if (!response.ok) throw new Error(`Horizons HTTP ${response.status}`);
      const payload=await response.json();
      if (payload.error) throw new Error(`Horizons: ${payload.error}`);
      const raw=payload.result;
      parts.push({url,raw,coordinates:parse(raw,name,id,batch)});
    }
    responses.push({name,id,parts,coordinates:parts.flatMap(p=>p.coordinates)});
  }
  const input=responses.flatMap(r=>r.coordinates.map((c,i)=>`${epochs[i]},${c.raDeg},${c.decDeg}`)).join('\n')+'\n';
  const lines=execFileSync(path.resolve(executable),[],{input,encoding:'utf8',maxBuffer:1048576,timeout:30000}).trim().split(/\r?\n/);
  if (lines.length!==epochs.length*bodies.length) throw new Error('wrong projection count');
  const projected=lines.map(line=>{const f=line.split(',');if(f.length!==10||!f.every(x=>x!==''&&Number.isFinite(Number(x)))) throw new Error('invalid projection');return f.map(Number);});
  const rows=grid.map((date,i)=>{
    const refs=responses.map(r=>unit(r.coordinates[i].ra,r.coordinates[i].dec));
    const actual=responses.map((_,b)=>unit(projected[b*epochs.length+i][2],projected[b*epochs.length+i][3]));
    const ecliptic=responses.map((r,b)=>{
      const p=projected[b*epochs.length+i],reference=unit(r.coordinates[i].lon,r.coordinates[i].lat);
      return {body:r.name,poleSeparationArcsec:separation(unit(p[4],p[5]),reference),directSeparationArcsec:separation(unit(p[6],p[7]),reference)};
    });
    return {...date,trueFit:heldoutFit(actual,refs),ecliptic};
  });
  if (crypto.createHash('sha256').update(fs.readFileSync(executable)).digest('hex')!==exeHash) throw new Error('executable changed');
  if (source && digest(source)!==sourceHash) throw new Error('prior report changed');
  const report={schema:'horizons-frame-epoch-holdout/1',status:'EXPLORATORY',executableSha256:exeHash,
    priorReportSha256:sourceHash || null,
    notes:'Iterated small-rotation fit with exact rotation composition using four bodies; two held-out directions from the same Horizons solution are not independent observations. Input directions come from Horizons DE441; no local orbit is evaluated. Differing orientation conventions, extrapolated nutation and model errors are not an accuracy bound.',
    training:train.map(i=>bodies[i][0]),heldout:heldout.map(i=>bodies[i][0]),summary:summarize(rows),rows,responses};
  fs.writeFileSync(output,JSON.stringify(report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(report.summary));
}
module.exports={rotate,fitRotation,heldoutFit,epochGrid,summarize};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
