// SPDX-License-Identifier: CPAL-1.0. Fictional coordinates only.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {classifyCompatibility}=createRequire(import.meta.url)('../../bindings/node/scripts/attach-iau-physical-identities.cjs');
const positions=Array.from({length:3},()=>[10,20]);
test('classifies missing, physically different and differently named references',()=>{
  assert.equal(classifyCompatibility('A',positions,[null,null,null]).code,'SWISS_LOOKUP_UNAVAILABLE');
  assert.equal(classifyCompatibility('A',positions,Array(3).fill({name:'B,x',longitude:11,latitude:20})).code,'SWISS_POSITION_MISMATCH');
  assert.equal(classifyCompatibility('A',positions,Array(3).fill({name:'B,x',longitude:10,latitude:20})).code,'SWISS_RETURNED_DIFFERENT_NAME');
  assert.equal(classifyCompatibility('A',positions,Array(3).fill({name:'A,x',longitude:10,latitude:20})),null);
  assert.throws(()=>classifyCompatibility('A',[],Array(3).fill({name:'A,x',longitude:10,latitude:20})),/invalid engine/);
});
