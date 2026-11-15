// Original VirgoStars source, CPAL-1.0 — see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readHip2Rows,HIP2_ROW_LENGTH,HIP2_EXPECTED_ROWS} from '../scripts/hipparcos-source-rows.mjs';
const row=(hip,fill=' ')=>String(hip).padStart(6,' ')+fill.repeat(HIP2_ROW_LENGTH-6);

test('exact-width data with no terminal newline preserves all 276 columns',()=>{
  const input=Buffer.from(row(17));
  const result=readHip2Rows(input,{requireFullCatalog:false});
  assert.equal(result.length,1);
  assert.equal(result[0].length,276);
  assert.equal(result[0].at(-1),' ');
  assert.equal(HIP2_EXPECTED_ROWS,117955);
});

test('strict complete-file validation checks row counts before using observations',()=>{
  assert.throws(()=>readHip2Rows(Buffer.from(row(17)+'\n')),/expected 117955 rows/);
  const all=readHip2Rows(Buffer.from(row(17)+'\n'),{requireFullCatalog:false});
  assert.equal(all.length,1);
});

test('all rows checked for length and duplicate HIP even when not selected',()=>{
  assert.throws(()=>readHip2Rows(Buffer.from(row(17)+'\n'+row(17)+'\n'),{requireFullCatalog:false}),/duplicate HIP/);
  assert.throws(()=>readHip2Rows(Buffer.from(row(17)+'\nshort\n'),{requireFullCatalog:false}),/expected 276 characters/);
  assert.throws(()=>readHip2Rows(Buffer.from(row(17)+'\n'+row(18)+'  \n'),{requireFullCatalog:false}),/expected 276 characters/);
});

test('invalid encoding, NUL bytes and bad IDs fail closed',()=>{
  assert.throws(()=>readHip2Rows(Buffer.from([0xff]),{requireFullCatalog:false}),/Invalid UTF-8/);
  assert.throws(()=>readHip2Rows(Buffer.from(row(17).slice(0,275)+'\0'),{requireFullCatalog:false}),/Non-ASCII/);
  assert.throws(()=>readHip2Rows(Buffer.from(row(0)),{requireFullCatalog:false}),/invalid HIP ID/);
  assert.throws(()=>readHip2Rows(Buffer.from(''),{requireFullCatalog:false}),/nonempty/);
});

test('Windows CRLF records remain accepted without shifting byte offsets',()=>{
  const rows=readHip2Rows(Buffer.from(row(17)+'\r\n'+row(18)+'\r\n'),{requireFullCatalog:false});
  assert.equal(rows.length,2);
  assert.equal(rows[0].length,276);
  assert.equal(rows[1].slice(0,6).trim(),'18');
});
