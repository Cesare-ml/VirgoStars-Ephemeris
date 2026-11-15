import {createExperimentalSwissSubset,SE_GREG_CAL,SE_SUN,SEFLG_JPLEPH,SEFLG_SPEED} from 'virgostars-ephemeris/experimental-swiss-subset';
import {createExplicitUtcInstant} from 'virgostars-ephemeris/experimental-utc-instant';
import {createExperimentalLegacyBridge} from 'virgostars-ephemeris/experimental-legacy-bridge';
import {loadExperimentalStarCatalogFile} from 'virgostars-ephemeris/experimental-star-catalog-file';

const loadedCatalog=loadExperimentalStarCatalogFile({filePath:'/absolute/catalog.json',sha256:'0'.repeat(64),source:'caller source',license:'caller license'});
const catalogId:string=loadedCatalog.catalog.id;void catalogId;
const api=createExperimentalSwissSubset({executablePath:'example',kernelPath:'example',physicalCenterKernels:{Mars:'mars.bsp'},convertUt1ToTtTdb:jd=>({ttJulianDay:jd,tdbJulianDay:jd})});
const jd:number=api.swe_julday(2000,1,1,12,SE_GREG_CAL);
const coords=api.swe_calc_ut(jd,SE_SUN,SEFLG_JPLEPH|SEFLG_SPEED);
if(!('longitude' in coords))throw new Error('expected ecliptic coordinates');
const longitude:number=coords.longitude;
void longitude;
const instant=createExplicitUtcInstant({executablePath:'example',datetimeUtc:'2026-11-10T00:00:00.000Z',ut1MinusUtcSeconds:0,taiMinusUtcSeconds:37,tdbMinusTtSeconds:0,sourceId:'example'});
const epoch:number=instant.convertUt1ToTtTdb(instant.ut1JulianDay).ttJulianDay;void epoch;
const magnitude:number=api.swe_fixstar_mag('example').magnitude;
const starLongitude:number=api.swe_fixstar_ut('example',jd,SEFLG_JPLEPH).longitude;
void magnitude;void starLongitude;
const houses=api.swe_houses(jd,41.9,12.5,'P');
const housePosition:number=api.swe_houses_pos(houses.armc,41.9,houses.trueObliquity,'P',20,1).housePosition;
void housePosition;
const nutation:number=api.swe_calc(jd,api.SE_ECL_NUT,0).longitude;
const name:string=api.swe_get_planet_name(api.SE_SUN).name;
void nutation;void name;
const horizon=api.swe_azalt(jd,api.SE_ECL2HOR,12.5,41.9,0,0,0,20,1,0);
horizon.azimuth=(horizon.azimuth+180)%360;
const rise=api.swe_rise_trans(jd,api.SE_SUN,'',api.SEFLG_JPLEPH,api.SE_CALC_RISE,12.5,41.9,0,0,0);
const transitUt1:number=rise.transitTime;void transitUt1;
const mutable=createExperimentalSwissSubset({executablePath:'example',kernelPath:'example',legacyMutableResults:true,starSolarDeflection:'POINT_MASS',planetSolarDeflection:'POINT_MASS',convertUt1ToTtTdb:jd=>({ttJulianDay:jd,tdbJulianDay:jd})});
const mutableHouse=mutable.swe_houses(jd,-41,12,'P');mutableHouse.house[0]=12;mutableHouse.ascendant=10;
const mutableStar=mutable.swe_fixstar_ut('A',jd,SEFLG_JPLEPH);delete mutableStar.name;mutableStar.longitude=10;
const legacy=createExperimentalLegacyBridge({executablePath:'example',kernelPath:'example',ephemerisDirectory:'example',flagPolicy:'MAP_SWISS_AND_DEFAULT_TO_JPL',defaultCalendar:'GREGORIAN',mutableResults:'LEGACY',convertUt1ToTtTdb:jd=>({ttJulianDay:jd,tdbJulianDay:jd})});
const legacyJd:number=legacy.swe_julday(2000,1,1,12);legacy.swe_set_ephe_path('example');void legacyJd;
// @ts-expect-error only P positions are supported
api.swe_houses_pos(0,41.9,23.4,'V',20,0);
// @ts-expect-error the calendar flag must be explicit
api.swe_julday(2000,1,1,12);
