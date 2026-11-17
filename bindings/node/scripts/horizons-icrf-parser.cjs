'use strict';
// SPDX-License-Identifier: CPAL-1.0. Validate a Horizons observer table.
function parseIcrfTable(raw,name,command,epochs){
  if(typeof raw!=='string'||!raw.includes(`Target body name: ${name} (${command})`)
    ||!/Target body name:.*\{source: DE441\}/.test(raw)
    ||!/Center body name: Earth \(399\).*\{source: DE441\}/.test(raw))throw new Error(`wrong target metadata: ${name}`);
  const header=raw.split(/\r?\n/).find(line=>line.includes('RA_(ICRF-a-app)'));
  const start=raw.indexOf('$$SOE'),end=raw.indexOf('$$EOE');
  if(!header||!header.includes('Date_________JDTT')||start<0||end<=start)throw new Error(`invalid ICRF table: ${name}`);
  const cols=header.split(',').map(x=>x.trim()),ra=cols.indexOf('RA_(ICRF-a-app)'),dec=cols.indexOf('DEC_(ICRF-a-app)');
  if(ra<0||dec<0)throw new Error(`missing ICRF columns: ${name}`);
  const rows=raw.slice(start+5,end).trim().split(/\r?\n/).map(line=>line.split(',').map(x=>x.trim()));
  if(rows.length!==epochs.length)throw new Error(`wrong row count: ${name}`);
  return rows.map((fields,i)=>{
    if(!fields[0]||!Number.isFinite(Number(fields[0]))||Math.abs(Number(fields[0])-epochs[i])>1e-8)
      throw new Error(`wrong TT epoch: ${name} row ${i}`);
    if(!fields[ra]||!fields[dec]||!Number.isFinite(Number(fields[ra]))||!Number.isFinite(Number(fields[dec])))
      throw new Error(`invalid coordinates: ${name}`);
    return {raDeg:Number(fields[ra]),decDeg:Number(fields[dec])};
  });
}
function parseIcrfRatesTable(raw,name,command,epochs){
  const positions=parseIcrfTable(raw,name,command,epochs);
  const header=raw.split(/\r?\n/).find(line=>line.includes('RA_(ICRF-a-app)'));
  const columns=header.split(',').map(x=>x.trim()),ra=columns.indexOf('I_dRA*cosD'),dec=columns.indexOf('I_d(DEC)/dt');
  if(ra<0||dec<0)throw new Error(`missing ICRF rate columns: ${name}`);
  const lines=raw.slice(raw.indexOf('$$SOE')+5,raw.indexOf('$$EOE')).trim().split(/\r?\n/);
  return lines.map((line,i)=>{
    const fields=line.split(',').map(x=>x.trim());
    if(!fields[ra]||!fields[dec]||![Number(fields[ra]),Number(fields[dec])].every(Number.isFinite))
      throw new Error(`invalid ICRF rates: ${name} row ${i}`);
    return {...positions[i],raCosDecArcsecHour:Number(fields[ra]),decArcsecHour:Number(fields[dec])};
  });
}
module.exports={parseIcrfTable,parseIcrfRatesTable};
