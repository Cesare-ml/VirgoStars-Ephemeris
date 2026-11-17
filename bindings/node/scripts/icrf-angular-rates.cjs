'use strict';
// SPDX-License-Identifier: CPAL-1.0. Cartesian ICRF rates, no astronomical model.
function icrfAngularRates(position,velocity){
  if(!Array.isArray(position)||!Array.isArray(velocity)||position.length!==3||velocity.length!==3
    ||![...position,...velocity].every(Number.isFinite))throw new Error('invalid ICRF state');
  const [x,y,z]=position,[vx,vy,vz]=velocity,rho=Math.hypot(x,y),r=Math.hypot(rho,z);
  if(!Number.isFinite(r)||!Number.isFinite(rho)||rho===0||r===0)throw new Error('undefined ICRF angles');
  const factor=(180/Math.PI)*3600/24;
  const raCosDecArcsecHour=(x*vy-y*vx)/(rho*r)*factor;
  const decArcsecHour=(vz*rho*rho-z*(x*vx+y*vy))/(r*r*rho)*factor;
  if(![raCosDecArcsecHour,decArcsecHour].every(Number.isFinite))throw new Error('nonfinite angular rates');
  return {raCosDecArcsecHour,decArcsecHour};
}
function icrfAngles(position){
  if(!Array.isArray(position)||position.length!==3||!position.every(Number.isFinite)||Math.hypot(...position)===0)
    throw new Error('invalid ICRF position');
  return {raDeg:Math.atan2(position[1],position[0])*180/Math.PI,
    decDeg:Math.atan2(position[2],Math.hypot(position[0],position[1]))*180/Math.PI};
}
function icrfFiniteRates(before,center,after,spanDays){
  if(!Number.isFinite(spanDays)||spanDays<=0||![before,center,after].every(x=>
    x&&Number.isFinite(x.raDeg)&&Number.isFinite(x.decDeg)&&Math.abs(x.decDeg)<=90))throw new Error('invalid finite-difference inputs');
  const deltaRa=((after.raDeg-before.raDeg)%360+540)%360-180;
  return {raCosDecArcsecHour:deltaRa*Math.cos(center.decDeg*Math.PI/180)*3600/(spanDays*24),
    decArcsecHour:(after.decDeg-before.decDeg)*3600/(spanDays*24)};
}
module.exports={icrfAngularRates,icrfAngles,icrfFiniteRates};
