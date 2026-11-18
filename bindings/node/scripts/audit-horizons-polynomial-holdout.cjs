'use strict';
// SPDX-License-Identifier: CPAL-1.0. Correlated holdout is not physical truth.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {parseIcrfTable}=require('./horizons-icrf-parser.cjs');
const {tangentOffsets,heldoutRms}=require('./fit-horizons-near-sun-rates.cjs');
const R=Math.PI/180,tdbToTt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*R)/86400;
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
async function main(){
  const [source,expected,out,...extra]=process.argv.slice(2);
  if(!source||!/^[a-f0-9]{64}$/.test(expected||'')||!out||extra.length||fs.existsSync(out)||path.resolve(source)===path.resolve(out))throw new Error('usage: <dense-report> <sha256> <new-report>');
  if(await sha(source)!==expected)throw new Error('input hash mismatch');const data=JSON.parse(fs.readFileSync(source));
  if(data.schema!=='horizons-near-sun-polynomial-rates/1'||!Array.isArray(data.rows)||data.rows.length!==14||!Array.isArray(data.requests)||data.requests.length!==14)throw new Error('invalid source');
  const bodies=new Map([[2,['199','Mercury']],[3,['299','Venus']],[4,['4','Mars Barycenter']],[5,['5','Jupiter Barycenter']],[6,['6','Saturn Barycenter']],[7,['7','Uranus Barycenter']],[8,['8','Neptune Barycenter']]]);
  const rows=data.rows.map((r,i)=>{
    const b=bodies.get(r.id),request=data.requests[i];
    if(!b||r.name!==b[1]||!Number.isFinite(r.jd)||!Number.isFinite(r.year)||request.name!==r.name||request.year!==r.year||typeof request.raw!=='string'||!request.raw.includes('Atmos refraction: NO (AIRLESS)'))throw new Error('invalid source row');
    const times=Array.from({length:61},(_,k)=>(k-30)/256),epochs=times.map(t=>tdbToTt(r.jd+t)),ref=parseIcrfTable(request.raw,r.name,b[0],epochs);
    // The tangent anchor itself is chosen from the training subset: the
    // central sample belongs to the holdout for the narrow window.
    const fit=(slice,degree)=>{const [start,end]=slice,t=times.slice(start,end),anchor=start+2*Math.floor((t.length-1)/4),v=tangentOffsets(ref.slice(start,end),ref[anchor]);const a=heldoutRms(t,v.map(p=>p[0]),degree),d=heldoutRms(t,v.map(p=>p[1]),degree);return {heldoutRmsArcsec:Math.hypot(a.rms,d.rms),heldoutCount:a.count,anchorOffsetDays:times[anchor]};};
    return {id:r.id,name:r.name,year:r.year,jd:r.jd,wide5:fit([0,61],5),wide7:fit([0,61],7),narrow5:fit([15,46],5),narrow7:fit([15,46],7)};
  });
  if(await sha(source)!==expected)throw new Error('source changed');
  fs.writeFileSync(out,JSON.stringify({schema:'horizons-near-sun-polynomial-holdout/1',status:'EXPLORATORY',sourceSha256:expected,notes:'Alternating epoch samples are held out within each window, from the same Horizons solution. Held-out RMS is predictive interpolation error, not physical accuracy or independent-data uncertainty.',rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(rows));
}
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
