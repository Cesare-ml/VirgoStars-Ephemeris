import {createExperimentalSwissSubset,SE_GREG_CAL,SE_SUN,SEFLG_JPLEPH,SEFLG_SPEED} from 'virgostars-ephemeris/experimental-swiss-subset';
const api=createExperimentalSwissSubset({executablePath:'example',kernelPath:'example',physicalCenterKernels:{Mars:'mars.bsp'},convertUt1ToTtTdb:jd=>({ttJulianDay:jd,tdbJulianDay:jd})});
const jd:number=api.swe_julday(2000,1,1,12,SE_GREG_CAL);
const coords=api.swe_calc_ut(jd,SE_SUN,SEFLG_JPLEPH|SEFLG_SPEED);
if(!('longitude' in coords))throw new Error('expected ecliptic coordinates');
const longitude:number=coords.longitude;
void longitude;
const magnitude:number=api.swe_fixstar_mag('example').magnitude;
const starLongitude:number=api.swe_fixstar_ut('example',jd,SEFLG_JPLEPH).longitude;
void magnitude;void starLongitude;
const houses=api.swe_houses(jd,41.9,12.5,'P');
const housePosition:number=api.swe_houses_pos(houses.armc,41.9,houses.trueObliquity,'P',20,1).housePosition;
void housePosition;
// @ts-expect-error only P positions are supported
api.swe_houses_pos(0,41.9,23.4,'V',20,0);
// @ts-expect-error the calendar flag must be explicit
api.swe_julday(2000,1,1,12);
