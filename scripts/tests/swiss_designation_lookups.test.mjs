// SPDX-License-Identifier: CPAL-1.0. Fictional coordinates only.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require=createRequire(import.meta.url);
const {matchDesignations}=require('../../bindings/node/scripts/extend-swiss-designation-lookups.cjs');
const stars=[{name:'A',aliases:[',existing'],hip:1},{name:'B',aliases:[],hip:2}];
const positions=new Map([[1,[[10,1],[10,1],[10,1]]],[2,[[20,2],[20,2],[20,2]]]]);
const reference=alias=>alias===',match' ? Array(3).fill({longitude:10.0001,latitude:1})
  : alias===',missing' ? [null,null,null] : Array(3).fill({longitude:99,latitude:1});
test('only uniquely matched, previously unavailable designations become candidates',()=>{
  const result=matchDesignations(stars,['existing','match','missing','none'],positions,reference);
  assert.equal(result.added.length,1); assert.equal(result.added[0].alias,',match'); assert.equal(result.added[0].hip,1);
  assert.deepEqual(result.failures.map(x=>x.reason),['REFERENCE_UNAVAILABLE','NO_MATCH']);
});
test('ambiguous matches and duplicate names are not silently accepted',()=>{
  const coincident=new Map([[1,[[10,1],[10,1],[10,1]]],[2,[[10,1],[10,1],[10,1]]]]);
  assert.equal(matchDesignations(stars,['match'],coincident,reference).failures[0].reason,'AMBIGUOUS_MATCH');
  assert.throws(()=>matchDesignations(stars,['match','MATCH'],positions,reference),/duplicate/);
});
