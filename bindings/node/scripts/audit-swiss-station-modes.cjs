'use strict';
// SPDX-License-Identifier: CPAL-1.0. Compare Swiss speed vs Swiss position derivative within each mode.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {extrapolatedRate}=require('./compare-swiss-seasonal-speeds.cjs');
const {bisect}=require('./compare-swiss-station-roots.cjs');
const {nearestBrackets,stats}=require('./compare-swiss-station-matrix.cjs');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
async function fileSha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
async function main(){
  const [kernel,kernelSha,swissPackage,matrix,matrixSha,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(kernelSha||'')||!swissPackage||!matrix||!/^[a-f0-9]{64}$/.test(matrixSha||'')||!out||extra.length||fs.existsSync(out)||new Set([kernel,matrix,out].map(p=>path.resolve(p))).size!==3||await fileSha(kernel)!==kernelSha)throw new Error('usage: <de441> <kernel-sha256> <swisseph> <matrix> <matrix-sha256> <new-report>');
  const bytes=fs.readFileSync(matrix);if(sha(bytes)!==matrixSha)throw new Error('matrix hash mismatch');
  const data=JSON.parse(bytes);
  if(data.schema!=='virgostars-swiss-station-matrix/1'||data.kernelSha256!==kernelSha||!Array.isArray(data.rows)||data.rows.length!==944||data.rows.some(r=>r.status!=='ok'))throw new Error('invalid station matrix');
  const swiss=require(path.resolve(swissPackage));swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));swiss.swe_set_jpl_file(path.basename(kernel));
  if(data.swissVersion!==swiss.swe_version())throw new Error('Swiss version differs from station matrix');
  const modes={apparent:0,noAberration:swiss.SEFLG_NOABERR,noDeflection:swiss.SEFLG_NOGDEFL,noNutation:swiss.SEFLG_NONUT,truePosition:swiss.SEFLG_TRUEPOS};
  if(Object.entries(modes).some(([name,value])=>name!=='apparent'&&(!Number.isInteger(value)||!value)))throw new Error('missing Swiss flags');
  const reference=(jd,body,mode)=>{const flags=swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED|modes[mode],r=swiss.swe_calc(tt(jd),body,flags);
    if(!r||r.error||(r.rflag&flags)!==flags||![r.longitude,r.longitudeSpeed].every(Number.isFinite))throw new Error('Swiss JPL/flag unavailable');return r;};
  const finite=(jd,body,mode)=>{const h=0.02,at=d=>reference(d,body,mode).longitude;return extrapolatedRate(at(jd-2*h),at(jd-h),at(jd+h),at(jd+2*h),jd,h,true);};
  const rows=[],center=b=>(b.lo+b.hi)/2;
  for(const r of data.rows)for(const mode of Object.keys(modes)){
    const seed=center(r.finiteBracket);
    try{
      // Find the same direction of crossing in this mode, not the zero from another frame.
      const bracket=nearestBrackets(jd=>finite(jd,r.body,mode),seed,2,0.25)[r.kind];
      if(!bracket){rows.push({year:r.year,body:r.body,kind:r.kind,mode,status:'missing-bracket'});continue;}
      const lo=bracket[0]-0.5,hi=bracket[1]+0.5,
        finiteRoot=bisect(jd=>finite(jd,r.body,mode),lo,hi,0.00001),
        publishedRoot=bisect(jd=>reference(jd,r.body,mode).longitudeSpeed,lo,hi,0.00001);
      rows.push({year:r.year,body:r.body,kind:r.kind,mode,status:'ok',finiteRoot,publishedRoot,
        publishedMinusPositionSeconds:(center(publishedRoot)-center(finiteRoot))*86400});
    }catch(e){rows.push({year:r.year,body:r.body,kind:r.kind,mode,status:'unresolved',reason:String(e.message)});}
  }
  if(await fileSha(kernel)!==kernelSha||sha(fs.readFileSync(matrix))!==matrixSha)throw new Error('input changed');
  const ok=rows.filter(r=>r.status==='ok'),summary={expected:944*Object.keys(modes).length,records:rows.length,ok:ok.length,unresolved:rows.length-ok.length,
    byMode:Object.fromEntries(Object.keys(modes).map(mode=>[mode,stats(ok.filter(r=>r.mode===mode).map(r=>r.publishedMinusPositionSeconds))])),
    byBody:Object.fromEntries(Array.from({length:8},(_,i)=>[i+2,Object.fromEntries(Object.keys(modes).map(mode=>[mode,stats(ok.filter(r=>r.mode===mode&&r.body===i+2).map(r=>r.publishedMinusPositionSeconds))]))]))};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-swiss-station-modes/1',status:'EXPLORATORY',kernelSha256:kernelSha,matrixSha256:matrixSha,swissVersion:swiss.swe_version(),
    notes:'Each mode compares Swiss speed with a derivative of its own longitude, in that mode. Different mode roots use different coordinates/corrections and must not be directly substituted.',summary,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));if(summary.unresolved||summary.records!==summary.expected)process.exitCode=1;
}
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
