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
import {createExperimentalSwissSubset as create} from '../src/experimental-swiss-subset.mjs';

const fixtureStar={name:'Synthetic A',aliases:['A'],epochTdbJulianDay:2451545,rightAscensionDeg:20,declinationDeg:-30,properMotionRaCosDecMasPerYear:120,properMotionDecMasPerYear:-40,parallaxMas:50,radialVelocityKmPerSecond:20,visualMagnitude:1.23};
test('solar event search requires explicit bounded assumptions',()=>{
  const base={executablePath:'x',kernelPath:'y',convertUt1ToTtTdb:()=>({ttJulianDay:0,tdbJulianDay:0})};
  assert.throws(()=>create(base).swe_rise_trans(2451545,0,'',1,1,0,0,0,0,0),/configuration/);
  assert.throws(()=>create({...base,solarEventSupport:{windowDays:2.1,stepMinutes:10,timeOffsets:'CONSTANT_IN_SEARCH'}}),/window/);
  assert.throws(()=>create({...base,solarEventSupport:{windowDays:1,stepMinutes:10}}),/assumption/);
  const api=create({...base,solarEventSupport:{windowDays:1,stepMinutes:10,timeOffsets:'CONSTANT_IN_SEARCH'}});
  assert.throws(()=>api.swe_rise_trans(2451545,api.SE_MOON,'',1,1,0,0,0,0,0),/solar/);
  assert.throws(()=>api.swe_rise_trans(2451545,api.SE_SUN,'',api.SEFLG_SWIEPH,1,0,0,0,0,0),/JPL/);
  assert.throws(()=>api.swe_rise_trans(2451545,api.SE_SUN,'',1,1,0,0,0,1013,0),/pressure/);
});

test('solar rise, set and meridian transits expose bounded UT1 events',{
  skip:!process.env.VIRGOSTARS_EPHEMERIS_CLI || !process.env.DE441_PATH,
},()=>{
  const configuration={windowDays:1.5,stepMinutes:10,timeOffsets:'CONSTANT_IN_SEARCH'};
  const api=create({executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,solarEventSupport:configuration,
    convertUt1ToTtTdb:ut1=>({ttJulianDay:ut1+0.000800741,tdbJulianDay:ut1+0.000800725})});
  configuration.windowDays=0;
  // Swiss 2.09.03 at Rome for one epoch: diagnostic only, differing models.
  const cases=[[api.SE_CALC_RISE,2461354.7451938586,30],[api.SE_CALC_SET,2461355.1627058745,30],[api.SE_CALC_MTRANSIT,2461354.9541137326,2],[api.SE_CALC_ITRANSIT,2461355.454149394,2]];
  for(const [event,reference,toleranceSeconds] of cases){
    let callback;const value=api.swe_rise_trans(2461354.5,api.SE_SUN,'',api.SEFLG_JPLEPH,event,12.4964,41.9028,0,0,0,v=>{callback=v;});
    assert.equal(value,callback);assert.equal(value.name,'');assert.ok(Math.abs(value.transitTime-reference)*86400<toleranceSeconds);
    assert.ok(value.bracketSeconds>=0 && value.bracketSeconds<=0.25);assert.ok(value.warnings.includes('NOT_SWISS_PARITY'));
    assert.ok(value.warnings.includes('NO_UT1_INTERPOLATION'));
  }
  assert.throws(()=>api.swe_rise_trans(2461354.5,api.SE_SUN,'',api.SEFLG_JPLEPH,api.SE_CALC_RISE,0,89,0,0,0),/crossing/i);
});
test('horizontal operation requires an explicit model and rejects unsupported weather',()=>{
  const options={executablePath:'x',kernelPath:'y',convertUt1ToTtTdb:()=>({ttJulianDay:0,tdbJulianDay:0})};
  assert.throws(()=>create(options).swe_azalt(2451545,0,0,0,0,0,0,0,0,0),/model/);
  assert.throws(()=>create({...options,horizontalSupport:{refraction:'SWISS'}}),/model/);
  const api=create({...options,horizontalSupport:{refraction:'GEOMETRIC'}});
  assert.throws(()=>api.swe_azalt(2451545,0,0,0,0,1013,0,0,0,0),/pressure/);
  assert.throws(()=>api.swe_azalt(2451545,0,0,0,1,0,0,0,0,0),/height/);
});

test('horizontal coordinate modes match sampled Swiss geometry with explicit refraction',{
  skip:!process.env.VIRGOSTARS_EPHEMERIS_CLI || !process.env.DE441_PATH,
},()=>{
  const options={executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,convertUt1ToTtTdb:ut1=>({ttJulianDay:ut1+0.000800741,tdbJulianDay:ut1+0.000800725})};
  const geometric=create({...options,horizontalSupport:{refraction:'GEOMETRIC'}});
  const noaa=create({...options,horizontalSupport:{refraction:'NOAA_STANDARD'}});
  // Recorded Swiss 2.09.03 geometry, Rome, at the same UT1; the two atmospheric models differ.
  for(const [longitude,latitude,az,alt] of [[47.6,0,34.6566451561,61.4276244692],[90,0,299.1958339192,60.0787708356],[227.6,0,214.6566451561,-61.4276244692],[180,30,222.3448671159,-7.4588384727]]){
    let callback;
    const value=geometric.swe_azalt(2461354.5,geometric.SE_ECL2HOR,12.4964,41.9028,0,0,0,longitude,latitude,0,v=>{callback=v;});
    assert.equal(value,callback);assert.ok(Math.abs(value.azimuth-az)<0.0001);assert.ok(Math.abs(value.trueAltitude-alt)<0.0001);
    assert.equal(value.apparentAltitude,value.trueAltitude);
    value.azimuth=(value.azimuth+180)%360;assert.ok(value.azimuth>=0 && value.azimuth<360);
    const withRefraction=noaa.swe_azalt(2461354.5,noaa.SE_ECL2HOR,12.4964,41.9028,0,0,0,longitude,latitude,0);
    if(alt < -1){assert.equal(withRefraction.apparentAltitude,withRefraction.trueAltitude);assert.ok(withRefraction.warnings.includes('BELOW_MINUS_ONE_NO_REFRACTION'));}
    else {assert.ok(withRefraction.apparentAltitude>withRefraction.trueAltitude);assert.ok(withRefraction.warnings.includes('NOAA_FIXED_ATMOSPHERE_APPROXIMATE'));}
  }
  const eq=geometric.swe_azalt(2461354.5,geometric.SE_EQU2HOR,12.4964,41.9028,0,0,0,225.1367,-17.0811,0);
  assert.ok(Math.abs(eq.azimuth-214.6566549742)<0.0001);assert.ok(Math.abs(eq.trueAltitude-(-61.4277080391))<0.0001);
  assert.throws(()=>geometric.swe_azalt(2461354.5,geometric.SE_ECL2HOR,0,90,0,0,0,90,0,0),/undefined/);
});
test('planet names are explicit and synchronous',()=>{
  const api=create({executablePath:'x',kernelPath:'y',convertUt1ToTtTdb:()=>({ttJulianDay:0,tdbJulianDay:0})});
  const names=['Sun','Moon','Mercury','Venus','Mars','Jupiter','Saturn','Uranus','Neptune','Pluto'];
  for(let i=0;i<names.length;i++){let callback;const result=api.swe_get_planet_name(i,v=>{callback=v;});assert.equal(result,callback);assert.deepEqual(result,{name:names[i]});assert.ok(Object.isFrozen(result));}
  assert.throws(()=>api.swe_get_planet_name(-1),/names/);
  assert.throws(()=>api.swe_calc(2451545,api.SE_SUN,0),/SE_ECL_NUT/);
  assert.throws(()=>api.swe_calc(2451545,api.SE_ECL_NUT,1),/flags/);
});

test('ecliptic nutation uses explicit TT and distinguishes UT conversion',{
  skip:!process.env.VIRGOSTARS_EPHEMERIS_CLI || !process.env.DE441_PATH,
},()=>{
  const tt=2461354.5;
  const api=create({executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,convertUt1ToTtTdb:ut1=>({ttJulianDay:ut1+0.0008,tdbJulianDay:ut1+0.0008})});
  let callback;const result=api.swe_calc(tt,api.SE_ECL_NUT,0,v=>{callback=v;});
  assert.equal(result,callback);
  // Swiss 2.09.03 at equal TT; model differs (IAU 2000B vs full nutation).
  for(const [actual,reference] of [[result.longitude,23.437859793212688],[result.latitude,23.435785305361748],[result.distance,0.002194607342977857],[result.longitudeSpeed,0.0020744878509422666]])assert.ok(Math.abs(actual-reference)<0.000001);
  assert.ok(Math.abs(result.longitude-result.latitude-result.longitudeSpeed)<1e-12);
  assert.deepEqual([result.latitudeSpeed,result.distanceSpeed,result.rflag],[0,0,0]);
  assert.ok(result.warnings.includes('FIELDS_ARE_ANGLES_NOT_POSITION_OR_SPEED'));
  assert.deepEqual(api.swe_calc_ut(tt,api.SE_ECL_NUT,0),api.swe_calc(tt+0.0008,api.SE_ECL_NUT,0));
  assert.throws(()=>api.swe_calc(2300000,api.SE_ECL_NUT,0),/status/);
});
test('catalog aliases and magnitudes are explicit immutable input snapshots',()=>{
  const record={...fixtureStar,aliases:[...fixtureStar.aliases]};
  const catalog={id:'synthetic-fixture',records:[record]};
  const options={executablePath:'x',kernelPath:'y',convertUt1ToTtTdb:()=>({ttJulianDay:0,tdbJulianDay:0}),catalog};
  const api=create(options);record.visualMagnitude=99;record.aliases[0]='changed';
  assert.deepEqual(api.swe_fixstar_mag('a'),{name:'Synthetic A',magnitude:1.23,catalogId:'synthetic-fixture',model:'CALLER_SUPPLIED_CATALOG_MAGNITUDE'});
  assert.throws(()=>api.swe_fixstar_mag('changed'),/not in/);
  assert.throws(()=>create({...options,catalog:{id:'x',records:[fixtureStar,{...fixtureStar,name:'other'}]}}),/Duplicate/);
  assert.throws(()=>create({...options,catalog:{id:'x',records:[{...fixtureStar,parallaxMas:0}]}}),/Invalid/);
  assert.throws(()=>create({...options,catalog:undefined}).swe_fixstar_mag('A'),/catalog/);
  assert.throws(()=>create({...options,starSolarDeflection:'SWISS'}),/deflection/);
});

test('optional stellar solar deflection changes near-Sun position and reports occultation',{
  skip:!process.env.VIRGOSTARS_EPHEMERIS_CLI || !process.env.DE441_PATH,
},()=>{
  const tt=2456165.901182685,deg=180/Math.PI,mas=deg*3600000;
  const star={name:'Synthetic ERFA',aliases:['E'],epochTdbJulianDay:2451545,rightAscensionDeg:2.71*deg,declinationDeg:.174*deg,properMotionRaCosDecMasPerYear:1e-5*Math.cos(.174)*mas,properMotionDecMasPerYear:5e-6*mas,parallaxMas:100,radialVelocityKmPerSecond:55,visualMagnitude:1};
  const earth=[0.9033585441304302,-0.4153952370279949,-0.18008401414326577];
  const ra=(Math.atan2(-earth[1],-earth[0])*deg+360)%360,dec=Math.atan2(-earth[2],Math.hypot(earth[0],earth[1]))*deg;
  const occulted={...star,name:'Occulted',aliases:[],epochTdbJulianDay:tt,rightAscensionDeg:ra,declinationDeg:dec,properMotionRaCosDecMasPerYear:0,properMotionDecMasPerYear:0,radialVelocityKmPerSecond:0};
  const opts={executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,convertUt1ToTtTdb:jd=>({ttJulianDay:jd,tdbJulianDay:jd}),catalog:{id:'synthetic-erfa',records:[star,occulted]}};
  const without=create(opts),withBending=create({...opts,starSolarDeflection:'POINT_MASS'});
  const a=without.swe_fixstar_ut('E',tt,without.SEFLG_JPLEPH|without.SEFLG_SPEED);
  let cb;const b=withBending.swe_fixstar_ut('E',tt,withBending.SEFLG_JPLEPH|withBending.SEFLG_SPEED,v=>{cb=v;});
  assert.equal(b,cb);assert.ok(a.warnings.includes('SOLAR_DEFLECTION_OMITTED'));assert.ok(b.warnings.includes('SOLAR_DEFLECTION_POINT_MASS_APPLIED'));
  const delta=Math.hypot((b.longitude-a.longitude)*Math.cos(b.latitude/deg),b.latitude-a.latitude)*3600;
  assert.ok(delta>1 && delta<2,`${delta} arcseconds`);
  const h=.001,before=withBending.swe_fixstar_ut('E',tt-h,1),after=withBending.swe_fixstar_ut('E',tt+h,1),span=(tt+h)-(tt-h);
  assert.ok(Math.abs(b.longitudeSpeed-((after.longitude-before.longitude+540)%360-180)/span)<.001);
  assert.ok(Math.abs(b.latitudeSpeed-(after.latitude-before.latitude)/span)<.001);
  assert.throws(()=>withBending.swe_fixstar_ut('Occulted',tt,1),/solar disk/);
});

test('rejects absent converters, unsupported bodies, flags and incoherent epochs without fallback',()=>{
  assert.throws(()=>create({executablePath:'x',kernelPath:'y'}),TypeError);
  const api=create({executablePath:'x',kernelPath:'y',convertUt1ToTtTdb:()=>({ttJulianDay:0,tdbJulianDay:0})});
  assert.throws(()=>api.swe_calc_ut(2461354.5,api.SE_MARS,api.SEFLG_JPLEPH),/physical-center SPK/);
  assert.throws(()=>api.swe_calc_ut(2461354.5,api.SE_URANUS,api.SEFLG_JPLEPH),/physical-center SPK/);
  assert.throws(()=>api.swe_calc_ut(2461354.5,99,api.SEFLG_JPLEPH),/physical centers/);
  assert.throws(()=>create({executablePath:'x',kernelPath:'y',convertUt1ToTtTdb:()=>({}),physicalCenterKernels:{Mars:''}}),/SPK/);
  assert.throws(()=>api.swe_calc_ut(2461354.5,api.SE_SUN,api.SEFLG_SWIEPH),/flags/);
  assert.throws(()=>api.swe_calc_ut(2461354.5,api.SE_SUN,api.SEFLG_JPLEPH|api.SEFLG_HELCTR|api.SEFLG_EQUATORIAL),/flags/);
  assert.throws(()=>api.swe_calc_ut(2461354.5,api.SE_SUN,4294967297),/flags/);
  assert.throws(()=>api.swe_calc_ut(2461354.5,api.SE_SUN,api.SEFLG_JPLEPH),/same instant/);
  assert.throws(()=>api.swe_julday(2000,1,1,12),/calendar/);
});

test('equatorial and heliocentric physical centers have correct field shapes and rate behavior',{
  skip:['VIRGOSTARS_EPHEMERIS_CLI','DE441_PATH','MARS_SPK_PATH','JUPITER_SPK_PATH','SATURN_SPK_PATH','URANUS_SPK_PATH','NEPTUNE_SPK_PATH','PLUTO_SPK_PATH'].some(name=>!process.env[name]),
},()=>{
  const names=['Mars','Jupiter','Saturn','Uranus','Neptune','Pluto'];
  const paths=Object.fromEntries(names.map(name=>[name,process.env[name.toUpperCase()+'_SPK_PATH']]));
  const api=create({executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,physicalCenterKernels:paths,
    convertUt1ToTtTdb:ut1=>({ttJulianDay:ut1+0.000800741,tdbJulianDay:ut1+0.000800725})});
  // Same-TT Swiss 2.09.03 with DE441: a diagnostic, not independent validation.
  const equatorial=[[225.1366968864,-17.0811490053],[231.9812637222,-24.0082251849],[214.4496015447,-12.3726845775],[200.4698050438,-11.4456221642],[145.9643862234,15.5406805717],[147.8400280809,13.7604197460],[9.1024595198,1.0296467921],[62.3850639777,20.8607267356],[2.3142381902,-0.5343167890],[306.5881003611,-23.5846782762]];
  const heliocentric=[[0,0],[47.5787433082,-0.0133791702],[75.4520326688,3.1712444566],[57.7364502261,-1.1168365583],[105.0583382669,1.5205107226],[134.6866277470,0.7273944051],[12.5470988923,-2.4373068776],[63.4869588981,-0.1429575118],[3.2669696121,-1.3774154524],[304.7638489489,-4.2940054554]];
  for(let body=0;body<10;body++){
    let callback;
    const eq=api.swe_calc_ut(2461354.5,body,api.SEFLG_JPLEPH|api.SEFLG_EQUATORIAL|api.SEFLG_SPEED,v=>{callback=v;});
    assert.equal(eq,callback);assert.equal('longitude' in eq,false);
    assert.ok(Math.abs(eq.rectAscension-equatorial[body][0])<0.0001);
    assert.ok(Math.abs(eq.declination-equatorial[body][1])<0.0001);
    const hel=api.swe_calc_ut(2461354.5,body,api.SEFLG_JPLEPH|api.SEFLG_HELCTR|api.SEFLG_SPEED);
    assert.ok(Math.abs(hel.longitude-heliocentric[body][0])<0.00025);
    assert.ok(Math.abs(hel.latitude-heliocentric[body][1])<0.0001);
    assert.ok(hel.warnings.includes('NO_ANNUAL_ABERRATION'));
    if(body===0){assert.equal(hel.distance,0);assert.ok(hel.warnings.includes('SUN_CENTER_ZERO_SENTINEL_NO_DIRECTION'));}
    else assert.ok(hel.distance>0);
    const eqNoSpeed=api.swe_calc_ut(2461354.5,body,api.SEFLG_JPLEPH|api.SEFLG_EQUATORIAL);
    assert.deepEqual([eqNoSpeed.rectAscensionSpeed,eqNoSpeed.declinationSpeed,eqNoSpeed.distanceSpeed],[0,0,0]);
  }
  for(const body of [api.SE_MERCURY,api.SE_JUPITER,api.SE_PLUTO])for(const [flag,angle,angleSpeed,latitude,latitudeSpeed] of [[api.SEFLG_EQUATORIAL,'rectAscension','rectAscensionSpeed','declination','declinationSpeed'],[api.SEFLG_HELCTR,'longitude','longitudeSpeed','latitude','latitudeSpeed']]){
    const flags=api.SEFLG_JPLEPH|api.SEFLG_SPEED|flag, t=2461354.5, dt=0.001;
    const before=api.swe_calc_ut(t-dt,body,flags),now=api.swe_calc_ut(t,body,flags),after=api.swe_calc_ut(t+dt,body,flags);
    const elapsed=(t+dt)-(t-dt);
    const delta=((after[angle]-before[angle]+540)%360)-180;
    assert.ok(Math.abs(now[angleSpeed]-delta/elapsed)<0.0001,`${body} ${angle} speed`);
    assert.ok(Math.abs(now[latitudeSpeed]-(after[latitude]-before[latitude])/elapsed)<0.0001,`${body} ${latitude} speed`);
    assert.ok(Math.abs(now.distanceSpeed-(after.distance-before.distance)/elapsed)<0.00001,`${body} distance speed`);
  }
  for(const flag of [api.SEFLG_EQUATORIAL,api.SEFLG_HELCTR])assert.throws(()=>api.swe_calc_ut(2451545,api.SE_MARS,api.SEFLG_JPLEPH|flag),/SupplementalSpk/);
});

test('real physical centers require supplemental kernels and expose speeds',{
  skip:!process.env.VIRGOSTARS_EPHEMERIS_CLI || !process.env.DE441_PATH || !process.env.MARS_SPK_PATH || !process.env.JUPITER_SPK_PATH || !process.env.SATURN_SPK_PATH,
},()=>{
  const paths={Mars:process.env.MARS_SPK_PATH,Jupiter:process.env.JUPITER_SPK_PATH,Saturn:process.env.SATURN_SPK_PATH};
  const api=create({executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,physicalCenterKernels:paths,
    convertUt1ToTtTdb:ut1=>({ttJulianDay:ut1+0.000800741,tdbJulianDay:ut1+0.000800725})});
  paths.Mars='changed-after-construction';
  for(const body of [api.SE_MARS,api.SE_JUPITER,api.SE_SATURN]){
    let callback;
    const result=api.swe_calc_ut(2461354.5,body,api.SEFLG_JPLEPH|api.SEFLG_SPEED,value=>{callback=value;});
    assert.equal(callback,result);
    assert.ok(result.longitude>=0 && result.longitude<360 && result.distance>0);
    assert.ok(Number.isFinite(result.longitudeSpeed) && result.longitudeSpeed!==0);
    assert.ok(result.warnings.includes('PHYSICAL_CENTER_SPK_EXPLICIT'));
    const noSpeed=api.swe_calc_ut(2461354.5,body,api.SEFLG_JPLEPH);
    assert.deepEqual([noSpeed.longitudeSpeed,noSpeed.latitudeSpeed,noSpeed.distanceSpeed],[0,0,0]);
  }
  assert.throws(()=>api.swe_calc_ut(2451545,api.SE_MARS,api.SEFLG_JPLEPH),/SupplementalSpk/);
});

test('real outer planets use their explicitly supplied center kernels',{
  skip:!process.env.VIRGOSTARS_EPHEMERIS_CLI || !process.env.DE441_PATH || !process.env.URANUS_SPK_PATH || !process.env.NEPTUNE_SPK_PATH || !process.env.PLUTO_SPK_PATH,
},()=>{
  const api=create({executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,
    physicalCenterKernels:{Uranus:process.env.URANUS_SPK_PATH,Neptune:process.env.NEPTUNE_SPK_PATH,Pluto:process.env.PLUTO_SPK_PATH},
    convertUt1ToTtTdb:ut1=>({ttJulianDay:ut1+0.000800741,tdbJulianDay:ut1+0.000800725})});
  for(const body of [api.SE_URANUS,api.SE_NEPTUNE,api.SE_PLUTO]){
    let callback;
    const result=api.swe_calc_ut(2461354.5,body,api.SEFLG_JPLEPH|api.SEFLG_SPEED,value=>{callback=value;});
    assert.equal(callback,result);
    assert.ok(result.longitude>=0 && result.longitude<360 && result.distance>0);
    assert.ok(Number.isFinite(result.longitudeSpeed) && result.longitudeSpeed!==0);
    assert.ok(result.warnings.includes('PHYSICAL_CENTER_SPK_EXPLICIT'));
  }
});

test('real DE441 subset supports calendar, physical targets and sync callbacks',{
  skip:!process.env.VIRGOSTARS_EPHEMERIS_CLI || !process.env.DE441_PATH,
},()=>{
  let conversions=0;
  const api=create({executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,
    convertUt1ToTtTdb:ut1=>{conversions++;assert.equal(ut1,2461354.5);return {ttJulianDay:2461354.500800741,tdbJulianDay:2461354.500800725};}});
  assert.equal(api.swe_julday(2000,1,1,12,api.SE_GREG_CAL),2451545);
  assert.equal(api.swe_julday(2000,1,1,12,api.SE_JUL_CAL),2451558);
  assert.deepEqual(api.swe_revjul(2451545,api.SE_GREG_CAL),{year:2000,month:1,day:1,hour:12});
  const refs=[
    [227.60006397597988,-0.00011258772084865995,1.0049844397937353],
    [235.61310848307508,-4.994070016873262,12.279083261707692],
    [216.32320925612183,1.3285538374668426,-0.7035704023233343],
    [203.19241664190434,-2.6211204487140023,-0.16419868548715205],
  ];
  for(let body=0;body<4;body++){
    let callbackResult;
    const result=api.swe_calc_ut(2461354.5,body,api.SEFLG_JPLEPH|api.SEFLG_SPEED,value=>{callbackResult=value;});
    assert.equal(result,callbackResult);
    assert.ok(Math.abs(result.longitude-refs[body][0])<1e-5);
    assert.ok(Math.abs(result.latitude-refs[body][1])<1e-5);
    assert.ok(Math.abs(result.longitudeSpeed-refs[body][2])<5e-5);
    assert.equal(result.rflag,257);
    assert.ok(result.warnings.includes('TDB_DAY_RATES_NOT_EXACT_UT_RATES'));
  }
  const noSpeed=api.swe_calc_ut(2461354.5,api.SE_MOON,api.SEFLG_JPLEPH);
  assert.deepEqual([noSpeed.longitudeSpeed,noSpeed.latitudeSpeed,noSpeed.distanceSpeed],[0,0,0]);
  assert.equal(conversions,5);
});

test('real DE441 star calculation exposes the supplied astrometry and signed rates',{
  skip:!process.env.VIRGOSTARS_EPHEMERIS_CLI || !process.env.DE441_PATH,
},()=>{
  const api=create({executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,
    convertUt1ToTtTdb:ut1=>({ttJulianDay:ut1+69.184/86400,tdbJulianDay:ut1+69.184/86400}),
    catalog:{id:'synthetic-fixture',records:[fixtureStar]}});
  let callbackValue;const result=api.swe_fixstar_ut('a',2461354.5,api.SEFLG_JPLEPH|api.SEFLG_SPEED,x=>{callbackValue=x;});
  assert.equal(result,callbackValue);assert.equal(result.name,'Synthetic A');assert.equal(result.catalogId,'synthetic-fixture');
  assert.ok(Number.isFinite(result.longitude) && Number.isFinite(result.longitudeSpeed) && result.distance>0);
  assert.ok(result.warnings.includes('STELLAR_LIGHT_TIME_OMITTED'));
  const before=api.swe_fixstar_ut('A',2461354.25,api.SEFLG_JPLEPH),after=api.swe_fixstar_ut('A',2461354.75,api.SEFLG_JPLEPH);
  const numerical=(((after.longitude-before.longitude+540)%360)-180)/0.5;
  assert.ok(Math.abs(numerical-result.longitudeSpeed)<0.00005);
  assert.equal(before.longitudeSpeed,0);
  const legacy=create({executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,legacyMutableResults:true,
    convertUt1ToTtTdb:ut1=>({ttJulianDay:ut1+69.184/86400,tdbJulianDay:ut1+69.184/86400}),catalog:{id:'synthetic-fixture',records:[fixtureStar]}});
  const mutable=legacy.swe_fixstar_ut('A',2461354.5,legacy.SEFLG_JPLEPH);
  const original=mutable.longitude;delete mutable.name;mutable.longitude=(mutable.longitude+180)%360;
  assert.equal(mutable.name,undefined);assert.equal(mutable.longitude,(original+180)%360);
  assert.equal(mutable.catalogId,'synthetic-fixture');assert.ok(mutable.warnings.includes('STELLAR_LIGHT_TIME_OMITTED'));
  assert.equal(Object.keys(mutable).includes('warnings'),false);
});
