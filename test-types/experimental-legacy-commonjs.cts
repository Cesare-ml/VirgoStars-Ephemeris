import bridge = require('virgostars-ephemeris/experimental-legacy-bridge');
async function example():Promise<void>{
  const api=await bridge.loadExperimentalLegacyBridge({executablePath:'/cli',kernelPath:'/kernel',ephemerisDirectory:'/',defaultCalendar:'GREGORIAN',flagPolicy:'MAP_SWISS_AND_DEFAULT_TO_JPL',mutableResults:'LEGACY',convertUt1ToTtTdb:jd=>({ttJulianDay:jd,tdbJulianDay:jd})});
  const jd:number=api.swe_julday(2000,1,1,12);
  const sun=api.swe_calc_ut(jd,0,1);
  if ('longitude' in sun) { const longitude:number=sun.longitude;void longitude; }
}
void example;
