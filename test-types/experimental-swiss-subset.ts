import {createExperimentalSwissSubset,SE_GREG_CAL,SE_SUN,SEFLG_JPLEPH,SEFLG_SPEED} from 'virgostars-ephemeris/experimental-swiss-subset';
const api=createExperimentalSwissSubset({executablePath:'example',kernelPath:'example',convertUt1ToTtTdb:jd=>({ttJulianDay:jd,tdbJulianDay:jd})});
const jd:number=api.swe_julday(2000,1,1,12,SE_GREG_CAL);
const longitude:number=api.swe_calc_ut(jd,SE_SUN,SEFLG_JPLEPH|SEFLG_SPEED).longitude;
void longitude;
// @ts-expect-error the calendar flag must be explicit
api.swe_julday(2000,1,1,12);
