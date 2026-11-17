// SPDX-License-Identifier: CPAL-1.0. Fictional diagnostic records only.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {summarizePlanetDiagnostics} from '../summarize_planet_validation_0_3000.mjs';

const names=['Sun','Moon','Mercury','Venus','Mars Barycenter','Jupiter Barycenter',
  'Saturn Barycenter','Uranus Barycenter','Neptune Barycenter','Pluto Barycenter'];
const commands=['10','301','199','299','4','5','6','7','8','9'];
const bytes=x=>Buffer.from(JSON.stringify(x));
function fixtures(){
  const h='a'.repeat(64),years=Array.from({length:13},(_,i)=>250*i);
  const raw=(name,code)=>`Target body name: ${name} (${code}) {source: DE441}\nCenter body name: Earth (399) {source: DE441}\n$$SOE\n$$EOE`;
  const body=(name,i,kind)=>({name,command:commands[i],raw:raw(name,commands[i]),records:years.map(year=>({year,jdTdb:1721242+year,
    ...(kind==='icrf'?{differenceArcsec:i/100}:{longitudeDifferenceArcsec:year/1000,latitudeDifferenceArcsec:0.01,
      longitudeRateDifferenceArcsecPerDay:0.1,latitudeRateDifferenceArcsecPerDay:0.1})}))});
  return [
    {schema:'virgostars-horizons-apparent-icrf/1',kernelSha256:h,rows:names.map((n,i)=>body(n,i,'icrf'))},
    {schema:'virgostars-swiss-long-range/1',kernelSha256:h,rows:names.flatMap((_,body)=>years.map(year=>({body,year,jdTdb:1721242+year,
      longitudeDifferenceArcsec:0.2,latitudeDifferenceArcsec:0.3,distanceDifferenceAu:0,
      engineLongitudeRateVsOwnFiniteArcsecPerDay:0,engineLatitudeRateVsOwnFiniteArcsecPerDay:0})))},
    {schema:'virgostars-horizons-long-range-grid/1',kernelSha256:h,rows:names.map((n,i)=>body(n,i,'grid'))}
  ];
}
test('summarizes exactly 130 matched epochs with per-body maxima',()=>{
  const report=summarizePlanetDiagnostics(...fixtures().map(bytes));
  assert.equal(report.cases,130);assert.equal(report.perBody.length,10);
  assert.equal(report.maxima.horizonsIcrfArcsec.body,'Pluto Barycenter');
  assert.equal(report.maxima.horizonsLongitudeArcsec.year,3000);
});
test('rejects mismatched kernels, duplicated samples and inconsistent epochs',()=>{
  for(const mutate of [
    f=>{f[1].kernelSha256='b'.repeat(64);},
    f=>{f[0].rows[0].records.push({...f[0].rows[0].records[0]});},
    f=>{f[1].rows[0].jdTdb++;},
    f=>{f[2].rows[0].raw='missing evidence';},
  ]){const f=fixtures();mutate(f);assert.throws(()=>summarizePlanetDiagnostics(...f.map(bytes)));}
});
