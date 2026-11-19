'use strict';
// SPDX-License-Identifier: CPAL-1.0. Incomplete velocity is a diagnostic only.
const fs=require('fs'),path=require('path'),{execFileSync}=require('child_process'),crypto=require('crypto');
const {icrfAngularRates}=require(path.resolve('bindings/node/scripts/icrf-angular-rates.cjs'));
const [kernel,executable,reference,out]=process.argv.slice(2);
if(!kernel||!executable||!reference||!out||fs.existsSync(out))throw Error('usage: kernel executable reference new-output');
const hash=f=>{let h=crypto.createHash('sha256'),fd=fs.openSync(f,'r'),buf=Buffer.alloc(4*1024*1024);try{for(;;){let n=fs.readSync(fd,buf,0,buf.length,null);if(!n)break;h.update(buf.subarray(0,n));}}finally{fs.closeSync(fd)}return h.digest('hex')};
const expected='476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da';
if(hash(kernel)!==expected)throw Error('kernel mismatch');
const data=JSON.parse(fs.readFileSync(reference));
const names={'Sun':'sun','Moon':'moon','Mercury':'mercury','Jupiter Barycenter':'jupiter'};
let results=data.results.map(r=>{
 let output=execFileSync(executable,[kernel,r.center==='Earth'?'earth':'ssb',names[r.name],String(r.tdb)],{encoding:'utf8'}).trim().split(/\r?\n/);
 let x=output[1].split(',').map(Number);if(x.length!==10||!x.every(Number.isFinite))throw Error('invalid state');
 return {name:r.name,year:r.year,center:r.center,q46:r.q46,q45:r.steps.find(s=>s.h===0.002).derivative,
  lightTimeRate:x[9],full:icrfAngularRates(x.slice(0,3),x.slice(3,6)),hybrid:icrfAngularRates(x.slice(0,3),x.slice(6,9))};
});
if(hash(kernel)!==expected)throw Error('kernel changed');
fs.writeFileSync(out,JSON.stringify({kernelSha256:expected,executableSha256:hash(executable),referenceSha256:hash(reference),notes:'No deflection or stellar aberration; direction is light-time only; hybrid is deliberately incomplete.',results},null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify(results));
