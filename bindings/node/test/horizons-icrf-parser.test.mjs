// SPDX-License-Identifier: CPAL-1.0. Synthetic Horizons-like responses.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {parseIcrfTable}=createRequire(import.meta.url)('../scripts/horizons-icrf-parser.cjs');
const raw=['Target body name: Sun (10) {source: DE441}',
  'Center body name: Earth (399) {source: DE441}',
  'Date_________JDTT, , RA_(ICRF-a-app), DEC_(ICRF-a-app),',
  '$$SOE','2451545.000000000, , 280.25, -22.5,','$$EOE'].join('\n');
test('parses a single header with both Unix and Windows line endings',()=>{
  for(const text of [raw,raw.replaceAll('\n','\r\n')])
    assert.deepEqual(parseIcrfTable(text,'Sun','10',[2451545]),[{raDeg:280.25,decDeg:-22.5}]);
});
test('rejects incorrect provenance, epochs and malformed coordinates',()=>{
  assert.throws(()=>parseIcrfTable(raw.replaceAll('DE441','DE440'),'Sun','10',[2451545]),/metadata/);
  assert.throws(()=>parseIcrfTable(raw,'Sun','10',[2451546]),/epoch/);
  assert.throws(()=>parseIcrfTable(raw.replace('280.25','NaN'),'Sun','10',[2451545]),/coordinates/);
  assert.throws(()=>parseIcrfTable(raw,'Moon','301',[2451545]),/metadata/);
});
