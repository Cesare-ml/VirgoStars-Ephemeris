// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

import test from 'node:test';
import assert from 'node:assert/strict';
import { createExperimentalRustCli } from '../src/experimental-rust-cli.mjs';

test('requires explicit executable and kernel paths', () => {
  assert.throws(() => createExperimentalRustCli({}), TypeError);
  assert.throws(() => createExperimentalRustCli({executablePath: 'x', kernelPath: ''}), TypeError);
});

test('missing executable fails explicitly', () => {
  const provider = createExperimentalRustCli({executablePath: 'nonexistent-virgostars-ephemeris', kernelPath: 'missing'});
  assert.throws(() => provider.calculate('2026-11-10T00:00:00Z'), /process failed/);
});

test('real Rust process and DE441 yield immutable labelled results', {
  skip: !process.env.VIRGOSTARS_EPHEMERIS_CLI || !process.env.DE441_PATH,
}, () => {
  const provider = createExperimentalRustCli({executablePath: process.env.VIRGOSTARS_EPHEMERIS_CLI, kernelPath: process.env.DE441_PATH});
  const output = provider.calculate('2026-11-10T00:00:00Z');
  const jd = provider.julianDay({year:2000, month:1, day:1, decimalHour:12, calendar:'gregorian', timeScale:'ut1'});
  assert.equal(jd, 2451545);
  assert.deepEqual(provider.reverseJulianDay({julianDay:jd, calendar:'gregorian', timeScale:'ut1'}), {year:2000, month:1, day:1, decimalHour:12, calendar:'gregorian', timeScale:'ut1'});
  assert.throws(() => provider.julianDay({year:1900, month:2, day:29, decimalHour:0, calendar:'gregorian', timeScale:'ut1'}), /exited with status/);
  assert.equal(output.results.length, 10);
  assert.equal(output.results[0].target, 'Sun');
  assert.equal(output.results[1].target, 'Moon');
  assert.ok(output.results[0].longitudeDeg > 227 && output.results[0].longitudeDeg < 228);
  assert.equal(output.results[4].target, 'MarsBarycenter');
  assert.ok(Object.isFrozen(output.results[0]));
  assert.ok(Object.isFrozen(output.results));
  const lunar = provider.calculateLunarPoints('2026-11-10T00:00:00Z');
  assert.deepEqual(lunar.results.map(item=>item.point), ['MeanNode','TrueNode','MeanApogee','OsculatingApogee']);
  assert.ok(Math.abs(lunar.results[0].distanceAu - 0.0025695552898) < 1e-10);
  assert.ok(Math.abs(lunar.results[2].latitudeDeg + 3.9062) < 0.002);
  assert.ok(Object.isFrozen(lunar.results));
  const equatorial = provider.calculateEquatorial('2026-11-10T00:00:00Z');
  assert.equal(equatorial.results.length,10);
  assert.equal(equatorial.frame,'IAU2006_2000B_TRUE_EQUATOR_OF_DATE');
  assert.ok(Math.abs(equatorial.results[0].rightAscensionDeg-225.1366968864)<0.001);
  assert.ok(Math.abs(equatorial.results[0].declinationDeg+17.0811490053)<0.001);
  assert.ok(Object.isFrozen(equatorial.results[0]));
  const heliocentric=provider.calculateHeliocentric('2026-11-10T00:00:00Z');
  assert.equal(heliocentric.results.length,10);
  assert.equal(heliocentric.center,'SUN');
  assert.equal(heliocentric.results[0].directionDefined,false);
  assert.equal(heliocentric.results[0].distanceAu,0);
  assert.equal(heliocentric.results[2].directionDefined,true);
  assert.ok(Math.abs(heliocentric.results[2].longitudeDeg-75.4520326688)<0.001);
  // This date previously exposed an adjacent-JD light-time iteration cycle.
  assert.equal(provider.calculateEquatorial('1980-10-01T00:00:00Z').results.length,10);
  assert.throws(() => provider.calculate('bad date'), /exited with status/);
  const angles = provider.calculateHouseAngles({ut1JulianDay: 2461354.5, ttJulianDay: output.results[0].ttJulianDay, latitudeDeg: 41.9028, longitudeDegEast: 12.4964});
  assert.ok(Math.abs(angles.ascendantDeg - 157.82641479) < 0.01);
  assert.ok(Math.abs(angles.midheavenDeg - 63.67826746) < 0.01);
  const horizontal=provider.calculateHorizontal({ut1JulianDay:2461354.5,ttJulianDay:output.results[0].ttJulianDay,latitudeDeg:41.9028,longitudeDegEast:12.4964,rightAscensionDeg:225.13669688642472,declinationDeg:-17.081149005284875});
  assert.ok(Math.abs(horizontal.azimuthSouthWestDeg-214.65670592212305)<0.001);
  assert.ok(Math.abs(horizontal.trueAltitudeDeg+61.42775066117141)<0.001);
  assert.ok(horizontal.warnings.includes('NO_REFRACTION'));
  const search={target:'Sun',kind:'rise',ut1JulianDay:2461354.5,ttJulianDay:output.results[0].ttJulianDay,tdbJulianDay:output.results[0].tdbJulianDay,latitudeDeg:41.9028,longitudeDegEast:12.4964,thresholdAltitudeDeg:0,windowDays:1,stepMinutes:10};
  const rise=provider.findGeometricEvent(search);
  const transit=provider.findGeometricEvent({...search,kind:'upper_transit'});
  const set=provider.findGeometricEvent({...search,kind:'set'});
  assert.ok(rise.ut1JulianDay<transit.ut1JulianDay && transit.ut1JulianDay<set.ut1JulianDay);
  assert.ok(rise.bracketSeconds<=0.25 && rise.warnings.includes('NO_PARALLAX'));
  assert.ok(Object.isFrozen(rise));
  assert.throws(()=>provider.findGeometricEvent({...search,thresholdAltitudeDeg:80}), /no requested crossing/);
  assert.throws(()=>provider.findGeometricEvent({...search,target:'Chiron'}), TypeError);
  const polar=provider.calculateHorizontal({ut1JulianDay:2451545,ttJulianDay:2451545,latitudeDeg:90,longitudeDegEast:0,rightAscensionDeg:0,declinationDeg:0});
  assert.equal(polar.azimuthNorthEastDeg,null);
  assert.equal(polar.azimuthSouthWestDeg,null);
  assert.throws(() => provider.calculateHouseAngles({ut1JulianDay: 2461354.5, ttJulianDay: 2461355.5, latitudeDeg: 41, longitudeDegEast: 12}), /exited with status/);
});
