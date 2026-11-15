import bridge = require('virgostars-ephemeris/experimental-legacy-bridge');
async function example():Promise<void>{
  const api=await bridge.loadExperimentalLegacyBridge({executablePath:'/cli',kernelPath:'/kernel',ephemerisDirectory:'/',defaultCalendar:'GREGORIAN',flagPolicy:'MAP_SWISS_AND_DEFAULT_TO_JPL',mutableResults:'LEGACY',lunarPointPolicy:'EXPERIMENTAL_GEOCENTRIC',lunarHeliocentricPolicy:'ZERO_SENTINEL',convertUt1ToTtTdb:jd=>({ttJulianDay:jd,tdbJulianDay:jd})});
  const jd:number=api.swe_julday(2000,1,1,12);
  const boundary:54=api.SE_PLUTO_PICKERING;void boundary;
  const sun=api.swe_calc_ut(jd,0,1);
  if ('longitude' in sun) { const longitude:number=sun.longitude;void longitude; }
  const point=api.swe_calc_ut(jd,api.SE_MEAN_NODE,1);
  if(point.model==='EXPERIMENTAL_GEOCENTRIC_LUNAR_POINT'){const longitude:number=point.longitude;void longitude;}
  if(point.model==='EXPERIMENTAL_LUNAR_HELIOCENTRIC_ZERO_SENTINEL'){const distance:number=point.distance;void distance;}
  if(point.model==='EXPERIMENTAL_EQUATORIAL_LUNAR_POINT'){const ra:number=point.rectAscension;void ra;}
}
void example;
