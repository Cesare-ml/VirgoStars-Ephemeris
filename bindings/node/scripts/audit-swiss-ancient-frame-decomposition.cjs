'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Same DE441 and Swiss implementation: model/convention comparison only.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
const {Engine,julday}=require('../index.js');
const {yearsFor,tdbToTt,checkReference}=require('./validate-swiss-fixed-icrs-grid.cjs');
const {unit,separation}=require('./compare-horizons-frame-isolation.cjs');
const {effectiveObliquity}=require('./compare-horizons-orientation-components.cjs');
const {checkConsistency}=require('./validate-horizons-planet-frame-grid.cjs');
function digest(file){const h=crypto.createHash('sha256'),fd=fs.openSync(file,'r'),buf=Buffer.allocUnsafe(1024*1024);try{for(;;){const n=fs.readSync(fd,buf,0,buf.length,null);if(!n)break;h.update(buf.subarray(0,n));}}finally{fs.closeSync(fd);}return h.digest('hex');}
function checkedDateReference(r,flags,forbidden,kind,id,jd){
  if(!['equatorial','ecliptic'].includes(kind))throw new Error('invalid reference kind');
  const a=kind==='equatorial'?'rectAscension':'longitude',b=kind==='equatorial'?'declination':'latitude';
  if(!r||r.error||!Number.isInteger(r.rflag)||(r.rflag&flags)!==flags||(r.rflag&forbidden)!==0||![r[a],r[b]].every(Number.isFinite)||r[a]<0||r[a]>=360||Math.abs(r[b])>90)throw new Error(`Swiss ${kind} reference unavailable: ${id}, ${jd}: ${r?.error||'flags or coordinates'}`);
  return [r[a],r[b]];
}
function inferredObliquity(eq,ecl){
  if(![eq,ecl].every(x=>Array.isArray(x)&&x.length===2&&x.every(Number.isFinite)&&x[0]>=0&&x[0]<360&&Math.abs(x[1])<=90))throw new Error('invalid obliquity coordinates');
  const v=unit(eq[0],eq[1]);
  if(v[1]*v[1]+v[2]*v[2]<1e-8)return null;
  return effectiveObliquity(eq[0],eq[1],ecl[0],ecl[1]);
}
function summarize(rows){
  if(!Array.isArray(rows)||!rows.length||!rows.every(r=>Number.isInteger(r.id)&&r.id>=0&&r.id<10&&Number.isFinite(r.year)&&[r.fixedArcsec,r.poleFrameArcsec,r.directFrameArcsec,r.poleActualArcsec,r.directActualArcsec].every(x=>Number.isFinite(x)&&x>=0)&&([r.poleObliquityDifferenceArcsec,r.directObliquityDifferenceArcsec,r.swissEquinoxXMismatchArcsec].every(x=>x===null)||[r.poleObliquityDifferenceArcsec,r.directObliquityDifferenceArcsec,r.swissEquinoxXMismatchArcsec].every(Number.isFinite))))throw new Error('invalid rows');
  const max=(list,key)=>{if(!list.length)throw new Error('empty subgroup');const r=list.reduce((a,b)=>Math.abs(b[key])>Math.abs(a[key])?b:a);return {absolute:Math.abs(r[key]),year:r.year,month:r.month,id:r.id};};
  const group=list=>({cases:list.length,fixed:max(list,'fixedArcsec'),poleFrame:max(list,'poleFrameArcsec'),directFrame:max(list,'directFrameArcsec'),poleActual:max(list,'poleActualArcsec'),directActual:max(list,'directActualArcsec')});
  const inferred=rows.filter(r=>r.poleObliquityDifferenceArcsec!==null);
  return {all:group(rows),preHorizons:group(rows.filter(r=>r.year< -9998)),perBody:Array.from({length:10},(_,id)=>({id,...group(rows.filter(r=>r.id===id))})),
    obliquity:{valid:inferred.length,illConditioned:rows.length-inferred.length,pole:inferred.length?max(inferred,'poleObliquityDifferenceArcsec'):null,direct:inferred.length?max(inferred,'directObliquityDifferenceArcsec'):null,xMismatch:inferred.length?max(inferred,'swissEquinoxXMismatchArcsec'):null}};
}
function main(){
  const [kernel,kernelHash,swissDir,source,sourceHash,projector,out,...extra]=process.argv.slice(2);
  if(!kernel||!swissDir||!source||!projector||!out||extra.length||![kernelHash,sourceHash].every(x=>/^[a-f0-9]{64}$/.test(x||''))||fs.existsSync(out))throw new Error('usage: KERNEL KERNEL_SHA SWISSEPH_PACKAGE FIXED_GRID_REPORT REPORT_SHA PROJECTOR NEW_REPORT');
  const pkg=path.resolve(swissDir),inputs=[path.resolve(kernel),path.resolve(source),path.resolve(projector),path.join(pkg,'package.json'),path.join(pkg,'lib/swisseph.js'),path.join(pkg,'build/Release/swisseph.node'),path.resolve(__dirname,'../native/virgostars_ephemeris.node')];
  if(new Set([...inputs,path.resolve(out)]).size!==inputs.length+1)throw new Error('inputs and output must differ');
  const hashes=inputs.map(digest);if(hashes[0]!==kernelHash||hashes[1]!==sourceHash)throw new Error('input hash mismatch');
  const fixed=JSON.parse(fs.readFileSync(source,'utf8'));
  if(fixed.schema!=='swiss-fixed-icrs-grid/1'||!['baseline','interleaved'].includes(fixed.grid)||fixed.kernelSha256!==hashes[0]||fixed.swissPackageSha256!==hashes[3]||fixed.swissWrapperSha256!==hashes[4]||fixed.swissNativeSha256!==hashes[5]||fixed.addonSha256!==hashes[6]||!Array.isArray(fixed.rows))throw new Error('fixed-grid provenance mismatch');
  const swiss=require(pkg),version=swiss.swe_version(),engine=new Engine(path.resolve(kernel));
  if(version!==fixed.swissVersion)throw new Error('Swiss version mismatch');
  const names=['SEFLG_JPLEPH','SEFLG_EQUATORIAL','SEFLG_J2000','SEFLG_ICRS','SEFLG_TRUEPOS','SEFLG_HELCTR','SEFLG_NOGDEFL','SEFLG_NOABERR','SEFLG_NONUT','SEFLG_TOPOCTR','SEFLG_SIDEREAL','SEFLG_BARYCTR','SEFLG_XYZ','SEFLG_RADIANS'];if(!names.every(k=>Number.isInteger(swiss[k])))throw new Error('Swiss flags unavailable');
  const jpl=swiss.SEFLG_JPLEPH,eq=swiss.SEFLG_EQUATORIAL,j2000=swiss.SEFLG_J2000,icrs=swiss.SEFLG_ICRS,fixedFlags=jpl|eq|j2000|icrs;
  const wrongCorrections=['SEFLG_TRUEPOS','SEFLG_HELCTR','SEFLG_NOGDEFL','SEFLG_NOABERR','SEFLG_TOPOCTR','SEFLG_SIDEREAL','SEFLG_BARYCTR','SEFLG_XYZ','SEFLG_RADIANS'].reduce((v,k)=>v|swiss[k],0),dateForbidden=wrongCorrections|swiss.SEFLG_NONUT;
  if(fixed.flags!==fixedFlags)throw new Error('fixed-grid flags mismatch');
  swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));swiss.swe_set_jpl_file(path.basename(kernel));
  const expected=yearsFor(fixed.grid).flatMap(year=>[1,4,7,10].flatMap(month=>Array.from({length:10},(_,id)=>({year,month,id}))));
  if(fixed.rows.length!==expected.length)throw new Error('wrong fixed-grid count');
  for(let i=0;i<expected.length;i++){
    const r=fixed.rows[i],e=expected[i],tdb=julday(e.year,e.month,1,12,true),tt=tdbToTt(tdb);
    if(r.year!==e.year||r.month!==e.month||r.id!==e.id||r.jdTdb!==tdb||r.jdTt!==tt||!Number.isFinite(r.differenceArcsec)||r.differenceArcsec<0)throw new Error(`fixed-grid mismatch at ${i}`);
    const reference=checkReference(swiss.swe_calc(tt,e.id,fixedFlags),fixedFlags,e.id,tt);
    if(reference.rectAscension!==r.referenceRaDeg||reference.declination!==r.referenceDecDeg||reference.rflag!==r.rflag||(reference.rflag&wrongCorrections)!==0)throw new Error(`fixed reference changed at ${i}`);
  }
  const input=fixed.rows.map(r=>`${r.jdTt},${r.referenceRaDeg},${r.referenceDecDeg}`).join('\n')+'\n';
  const lines=execFileSync(path.resolve(projector),[],{input,encoding:'utf8',maxBuffer:4*1048576,timeout:30000}).trim().split(/\r?\n/);
  if(lines.length!==fixed.rows.length)throw new Error('wrong projection count');
  const projected=lines.map(line=>{const f=line.split(',');if(f.length!==10||!f.every(x=>x!==''&&Number.isFinite(Number(x))))throw new Error('invalid projection');return f.map(Number);});
  const rows=fixed.rows.map((r,i)=>{
    const p=projected[i],eclResult=swiss.swe_calc(r.jdTt,r.id,jpl),eqResult=swiss.swe_calc(r.jdTt,r.id,jpl|eq);
    const dateEcl=checkedDateReference(eclResult,jpl,eq|j2000|icrs|dateForbidden,'ecliptic',r.id,r.jdTt),dateEq=checkedDateReference(eqResult,jpl|eq,j2000|icrs|dateForbidden,'equatorial',r.id,r.jdTt);
    const reference=unit(...dateEcl),projectedPole=unit(p[4],p[5]),projectedDirect=unit(p[6],p[7]);
    const localPole=engine.experimentalLongTermApparentTdb(r.jdTdb,r.id).xx,localDirect=engine.experimentalLongTermDirectObliquityTdb(r.jdTdb,r.id).xx;
    if(![localPole,localDirect].every(x=>x.length===6&&x.every(Number.isFinite)))throw new Error('invalid local ecliptic');
    const pole=unit(localPole[0],localPole[1]),direct=unit(localDirect[0],localDirect[1]);
    const cmp={icrfArcsec:r.differenceArcsec,poleArcsec:separation(pole,reference),directArcsec:separation(direct,reference),samePoleArcsec:separation(pole,projectedPole),sameDirectArcsec:separation(direct,projectedDirect),poleFrameArcsec:separation(projectedPole,reference),directFrameArcsec:separation(projectedDirect,reference)};
    checkConsistency(cmp);
    const ob=inferredObliquity(dateEq,dateEcl);
    return {year:r.year,month:r.month,id:r.id,jdTdb:r.jdTdb,jdTt:r.jdTt,solarElongationDeg:r.solarElongationDeg,fixedArcsec:cmp.icrfArcsec,
      poleFrameArcsec:cmp.poleFrameArcsec,directFrameArcsec:cmp.directFrameArcsec,poleActualArcsec:cmp.poleArcsec,directActualArcsec:cmp.directArcsec,
      samePoleArcsec:cmp.samePoleArcsec,sameDirectArcsec:cmp.sameDirectArcsec,swissFixedFlags:r.rflag,swissEclipticFlags:eclResult.rflag,swissEquatorialFlags:eqResult.rflag,swissEclipticDeg:dateEcl,swissEquatorialDeg:dateEq,
      swissObliquityDeg:ob?.degrees??null,swissEquinoxXMismatchArcsec:ob?.xMismatchArcsec??null,
      poleObliquityDifferenceArcsec:ob?(p[8]-ob.degrees)*3600:null,directObliquityDifferenceArcsec:ob?(p[9]-ob.degrees)*3600:null};
  });
  if(inputs.some((p,i)=>digest(p)!==hashes[i]))throw new Error('input changed');
  const report={schema:'swiss-ancient-frame-decomposition/1',status:'EXPLORATORY',grid:fixed.grid,kernelSha256:hashes[0],sourceSha256:hashes[1],projectorSha256:hashes[2],swissPackageSha256:hashes[3],swissWrapperSha256:hashes[4],swissNativeSha256:hashes[5],addonSha256:hashes[6],swissVersion:version,
    notes:'Both use DE441. Project Swiss ICRS apparent direction into local axes and compare Swiss apparent of-date coordinates. Inferred obliquity requires the same apparent equinox; x mismatch checks that assumption. This is not independent Earth orientation or physical orbital validation.',summary:summarize(rows),rows};
  fs.writeFileSync(out,JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(report.summary));
}
module.exports={checkedDateReference,inferredObliquity,summarize};
if(require.main===module){try{main();}catch(e){console.error(e.message);process.exitCode=1;}}
