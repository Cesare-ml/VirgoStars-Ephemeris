'use strict';
// SPDX-License-Identifier: CPAL-1.0. Exploratory matched-epoch comparison.
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const {Engine, julday} = require('../index.js');
const {validationYears} = require('./planet-validation-years.cjs');
const {extrapolatedRate} = require('./compare-swiss-seasonal-speeds.cjs');
const wrap = x => ((x % 360) + 540) % 360 - 180;
const tt = jd => jd - 0.001657 * Math.sin((357.5291092 + 35999.0502909 * ((jd - 2451545) / 36525)) * Math.PI / 180) / 86400;
async function sha(file) { const h=crypto.createHash('sha256'); for await(const c of fs.createReadStream(file)) h.update(c); return h.digest('hex'); }
function maxima(rows) {
  const specs = {
    poleLongitude:r=>r.pole.lon, poleLatitude:r=>r.pole.lat,
    directLongitude:r=>r.direct.lon, directLatitude:r=>r.direct.lat,
    poleLongitudeRate:r=>r.pole.rateLon, poleLatitudeRate:r=>r.pole.rateLat,
    directLongitudeRate:r=>r.direct.rateLon, directLatitudeRate:r=>r.direct.rateLat,
    directOwnLongitudeRate:r=>r.direct.ownRateLon, directOwnLatitudeRate:r=>r.direct.ownRateLat,
  };
  return Object.fromEntries(Object.entries(specs).map(([key, select]) => {
    const r=rows.reduce((a,b)=>Math.abs(select(b))>Math.abs(select(a))?b:a);
    return [key,{absolute:Math.abs(select(r)),year:r.year,month:r.month,body:r.body}];
  }));
}
async function main() {
  const [kernel, expected, swissPackage, out, grid='baseline', ...extra] = process.argv.slice(2);
  if (!kernel || !/^[a-f0-9]{64}$/.test(expected||'') || !swissPackage || !out || extra.length
      || !['baseline','interleaved'].includes(grid) || path.resolve(kernel)===path.resolve(out) || fs.existsSync(out))
    throw new Error('usage: <de441> <sha256> <swisseph> <new-report> [baseline|interleaved]');
  if(await sha(kernel)!==expected) throw new Error('kernel hash mismatch');
  const swiss=require(path.resolve(swissPackage)), engine=new Engine(path.resolve(kernel)), h=0.02, rows=[];
  swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel))); swiss.swe_set_jpl_file(path.basename(kernel));
  const reference=(jd,body)=>{
    const r=swiss.swe_calc(tt(jd),body,swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED);
    if(!r||r.error||!(r.rflag&swiss.SEFLG_JPLEPH)||![r.longitude,r.latitude].every(Number.isFinite))
      throw new Error(`Swiss JPL reference unavailable: ${body}, ${jd}`);
    return [r.longitude,r.latitude];
  };
  const years=grid==='baseline'?validationYears('13000bce-3999-step-250'):Array.from({length:68},(_,i)=>-12875+i*250);
  for(const year of years) for(const month of [1,4,7,10]) for(let body=0;body<10;body++){
    const jd=julday(year,month,1,12,true), epochs=[jd-2*h,jd-h,jd,jd+h,jd+2*h];
    const ref=epochs.map(x=>reference(x,body));
    const pole=engine.experimentalLongTermApparentTdb(jd,body).xx;
    const direct=epochs.map(x=>engine.experimentalLongTermDirectObliquityTdb(x,body).xx);
    if(![pole,...direct].every(r=>r.length===6&&r.every(Number.isFinite)))throw new Error(`invalid engine state ${year}, ${month}, ${body}`);
    const rate=(samples,i)=>extrapolatedRate(samples[0][i],samples[1][i],samples[3][i],samples[4][i],jd,h,i===0);
    const refLonRate=rate(ref,0),refLatRate=rate(ref,1), d=direct[2];
    rows.push({year,month,body,jdTdb:jd,
      pole:{lon:wrap(pole[0]-ref[2][0])*3600,lat:(pole[1]-ref[2][1])*3600,
        rateLon:(pole[3]-refLonRate)*3600,rateLat:(pole[4]-refLatRate)*3600},
      direct:{lon:wrap(d[0]-ref[2][0])*3600,lat:(d[1]-ref[2][1])*3600,
        rateLon:(d[3]-refLonRate)*3600,rateLat:(d[4]-refLatRate)*3600,
        ownRateLon:(d[3]-rate(direct,0))*3600,ownRateLat:(d[4]-rate(direct,1))*3600}});
  }
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  const holdout=rows.filter(r=>r.month!==7), july=rows.filter(r=>r.month===7);
  const summary={grid,cases:rows.length,julyCases:july.length,otherSeasonCases:holdout.length,
    halfStepDays:h,all:maxima(rows),july:maxima(july),otherSeasons:maxima(holdout)};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-ancient-obliquity-comparison/1',status:'EXPLORATORY',
    kernelSha256:expected,swissVersion:swiss.swe_version(),
    notes:'Apparent geocentric ecliptic of date; TDB vs approximate TT; shared DE441. Baseline July repeats previous design; interleaved grid uses different years. Rates compared to Swiss position derivative, not Swiss-reported speeds. No accuracy guarantee.',summary,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
