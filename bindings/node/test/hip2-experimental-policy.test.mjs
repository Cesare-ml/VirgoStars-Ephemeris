// SPDX-License-Identifier: CPAL-1.0. Caller-supplied real catalog and ephemeris.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const {Engine,inspectHipparcosQuality}=require('../index.js');
const {DE441_PATH,HIP2_PATH}=process.env;
test('broader HIP2 policy is explicit and reports a simplified solution',
  {skip:DE441_PATH&&HIP2_PATH?false:'DE441_PATH and HIP2_PATH required'},()=>{
    const engine=new Engine(DE441_PATH);
    const quality=inspectHipparcosQuality(HIP2_PATH,[32349])[0];
    assert.equal(quality.traditionalEligible,false);
    assert.equal(quality.experimentalEligible,true);
    assert.throws(()=>engine.loadHipparcos(HIP2_PATH,[32349],'traditional'),e=>e.code==='E_STAR_QUALITY');
    assert.equal(engine.loadHipparcos(HIP2_PATH,[32349],'experimental'),1);
    const result=engine.fixstarTdb(32349,null,2451545,256);
    assert.ok(result.xx.every(Number.isFinite));
    assert.match(result.notes.join(' '),/EXPERIMENTAL.*extra astrometric parameters not applied/);
  });
