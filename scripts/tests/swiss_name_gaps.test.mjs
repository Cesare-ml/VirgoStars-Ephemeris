// SPDX-License-Identifier: CPAL-1.0. Fictional records only.
import {test} from 'node:test';import assert from 'node:assert/strict';import {createRequire} from 'node:module';
const {auditNameGaps}=createRequire(import.meta.url)('../../bindings/node/scripts/audit-swiss-name-gaps.cjs');
const stars=[{name:'Known',aliases:[],hip:1}];const positions=new Map([[1,[[10,1],[10,1],[10,1]]]]);
const ref=name=>name==='Alias'?Array(3).fill({name:'Alias,X',longitude:10,latitude:1})
  :name==='Prefix'?Array(3).fill({name:'PrefixLong,X',longitude:10,latitude:1}):[null,null,null];
test('classifies prefix mismatch, missing reference and existing positional lead without approval',()=>{
  const out=auditNameGaps(stars,['Known','Alias','Prefix','Unknown'],positions,ref);
  assert.deepEqual(out.results.map(r=>r.status),['EXISTING_POSITION_REVIEW','REFERENCE_PREFIX_OR_NAME_MISMATCH','REFERENCE_UNAVAILABLE']);
  assert.equal(out.results[0].matches[0].hip,1);
  assert.throws(()=>auditNameGaps(stars,['Alias','alias'],positions,ref),/duplicate/);
});
