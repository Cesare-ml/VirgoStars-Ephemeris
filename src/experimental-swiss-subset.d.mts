export declare const SE_JUL_CAL:0,SE_GREG_CAL:1,SEFLG_JPLEPH:1,SEFLG_SWIEPH:2,SEFLG_HELCTR:8,SEFLG_SPEED:256,SEFLG_EQUATORIAL:2048;
export declare const SE_ECL2HOR:0,SE_EQU2HOR:1;
export declare const SE_CALC_RISE:1,SE_CALC_SET:2,SE_CALC_MTRANSIT:4,SE_CALC_ITRANSIT:8;
export declare const SE_ECL_NUT:-1,SE_SUN:0,SE_MOON:1,SE_MERCURY:2,SE_VENUS:3,SE_MARS:4,SE_JUPITER:5,SE_SATURN:6,SE_URANUS:7,SE_NEPTUNE:8,SE_PLUTO:9;
export declare const SE_MEAN_NODE:10,SE_TRUE_NODE:11,SE_MEAN_APOG:12,SE_OSCU_APOG:13;
export interface ExperimentalSwissResult {
  readonly longitude:number; readonly latitude:number; readonly distance:number;
  readonly longitudeSpeed:number; readonly latitudeSpeed:number; readonly distanceSpeed:number;
  readonly rflag:number; readonly model:'EXPERIMENTAL_DE441_IAU2006_2000B'; readonly warnings:readonly string[];
}
export interface ExperimentalEquatorialResult {
  readonly rectAscension:number;readonly declination:number;readonly distance:number;
  readonly rectAscensionSpeed:number;readonly declinationSpeed:number;readonly distanceSpeed:number;
  readonly rflag:number;readonly model:'EXPERIMENTAL_DE441_IAU2006_2000B';readonly warnings:readonly string[];
}
export interface ExperimentalLunarPointResult extends Omit<ExperimentalSwissResult,'model'> {readonly model:'EXPERIMENTAL_GEOCENTRIC_LUNAR_POINT';}
export interface ExperimentalLunarHeliocentricSentinel extends Omit<ExperimentalSwissResult,'model'> {readonly model:'EXPERIMENTAL_LUNAR_HELIOCENTRIC_ZERO_SENTINEL';}
export interface ExperimentalLunarEquatorialResult extends Omit<ExperimentalEquatorialResult,'model'> {readonly model:'EXPERIMENTAL_EQUATORIAL_LUNAR_POINT';}
/** Six Swiss-shaped fields encode obliquities and nutations, not positions or speeds. */
export interface ExperimentalEclipticNutation extends Omit<ExperimentalSwissResult,'model'|'rflag'> {
  readonly model:'EXPERIMENTAL_IAU2006_2000B_NUTATION';readonly rflag:0;
}
export interface ExperimentalStarRecord {
  readonly name:string;readonly aliases:readonly string[];readonly epochTdbJulianDay:number;
  readonly rightAscensionDeg:number;readonly declinationDeg:number;
  readonly properMotionRaCosDecMasPerYear:number;readonly properMotionDecMasPerYear:number;
  readonly parallaxMas:number;readonly radialVelocityKmPerSecond:number;readonly visualMagnitude:number;
}
export interface ExperimentalStarResult extends Omit<ExperimentalSwissResult,'model'> {
  readonly name:string; readonly catalogId:string; readonly model:'EXPERIMENTAL_ICRS_LINEAR_STAR';
}
export interface ExperimentalHouseAngleInput {readonly ut1JulianDay:number;readonly ttJulianDay:number;readonly latitudeDeg:number;readonly longitudeDegEast:number;}
export interface ExperimentalHouseAngles extends ExperimentalHouseAngleInput {readonly model:'IAU2006_2000B_APPROXIMATE_SIDEREAL_TIME';readonly armcDeg:number;readonly ascendantDeg:number;readonly midheavenDeg:number;readonly trueObliquityDeg:number;}
export interface ExperimentalHouseSetInput extends ExperimentalHouseAngleInput {readonly datetimeUtc:string;readonly birthTimeKnown:true;readonly engineVersion:string;readonly kernelSetId:string;readonly timeModel:string;}
export interface ExperimentalHouseSupport {
  readonly calculateHouseSet:(input:ExperimentalHouseSetInput,provider:{calculateHouseAngles(input:ExperimentalHouseAngleInput):ExperimentalHouseAngles})=>{readonly contract:'ExperimentalHouseSet';readonly validation:'CALLER_ASSERTED_INPUTS_NOT_INDEPENDENTLY_VERIFIED';readonly houses:Readonly<Record<'vehlow'|'placidus'|'koch'|'regiomontanus'|'porphyry',{readonly houseSystem:string;readonly cuspsDeg:readonly number[]}>>};
  readonly convertUt1ToUtc:(ut1JulianDay:number)=>string;
  readonly engineVersion:string;readonly kernelSetId:string;readonly timeModel:string;
  readonly calculatePlacidusHousePosition?:(input:{readonly armcDeg:number;readonly latitudeDeg:number;readonly trueObliquityDeg:number;readonly eclipticLongitudeDeg:number;readonly eclipticLatitudeDeg:number;readonly frame:'ECLIPTIC_OF_DATE';readonly zodiac:'TROPICAL';readonly birthTimeKnown:true;readonly datetimeUtc:string;readonly source:{readonly engineVersion:string;readonly kernelSetId:string;readonly timeModel:string;readonly corrections:string;readonly center:'EARTH';readonly observerLongitudeDegEast:number}})=>{readonly contract:'PlacidusHousePosition';readonly housePosition:number};
  readonly positionContext?:{readonly datetimeUtc:string;readonly corrections:string;readonly observerLongitudeDegEast:number};
}
export interface ExperimentalHouses {readonly house:readonly number[];readonly ascendant:number;readonly mc:number;readonly armc:number;readonly trueObliquity:number;readonly model:'EXPERIMENTAL_IAU2006_2000B_HOUSES';readonly warnings:readonly string[];}
export interface ExperimentalHousePosition {readonly housePosition:number;readonly model:'EXPERIMENTAL_PLACIDUS_POSITION';readonly warnings:readonly string[];}
export interface ExperimentalHorizontal {azimuth:number;trueAltitude:number;apparentAltitude:number;readonly model:'EXPERIMENTAL_GEOMETRIC_HORIZON'|'EXPERIMENTAL_NOAA_STANDARD_HORIZON';readonly warnings:readonly string[];}
export interface ExperimentalSolarEvent {readonly name:'';readonly transitTime:number;readonly bracketSeconds:number;readonly model:string;readonly warnings:readonly string[];}
export interface ExperimentalSwissSubset {
  readonly SE_JUL_CAL:0;readonly SE_GREG_CAL:1;readonly SEFLG_JPLEPH:1;readonly SEFLG_SWIEPH:2;readonly SEFLG_HELCTR:8;readonly SEFLG_EQUATORIAL:2048;readonly SEFLG_SPEED:256;
  readonly SE_ECL2HOR:0;readonly SE_EQU2HOR:1;readonly SE_ECL_NUT:-1;readonly SE_SUN:0;readonly SE_MOON:1;readonly SE_MERCURY:2;readonly SE_VENUS:3;readonly SE_MARS:4;readonly SE_JUPITER:5;readonly SE_SATURN:6;readonly SE_URANUS:7;readonly SE_NEPTUNE:8;readonly SE_PLUTO:9;
  readonly SE_MEAN_NODE:10;readonly SE_TRUE_NODE:11;readonly SE_MEAN_APOG:12;readonly SE_OSCU_APOG:13;
  readonly SE_CALC_RISE:1;readonly SE_CALC_SET:2;readonly SE_CALC_MTRANSIT:4;readonly SE_CALC_ITRANSIT:8;
  swe_rise_trans(julianDay:number,body:0,starName:'',flags:1,event:1|2|4|8,longitudeDegEast:number,latitudeDeg:number,heightMetres:0,pressureHpa:0,temperatureC:0,callback?:(result:ExperimentalSolarEvent)=>void):Readonly<ExperimentalSolarEvent>;
  swe_azalt(julianDay:number,mode:0|1,longitudeDegEast:number,latitudeDeg:number,heightMetres:number,pressureHpa:number,temperatureC:number,longitudeOrRa:number,latitudeOrDec:number,distance:number,callback?:(result:ExperimentalHorizontal)=>void):ExperimentalHorizontal;
  swe_get_planet_name(body:number,callback?:(result:Readonly<{name:string}>)=>void):Readonly<{name:string}>;
  swe_calc(julianDayTt:number,body:-1,flags:0,callback?:(result:ExperimentalEclipticNutation)=>void):Readonly<ExperimentalEclipticNutation>;
  swe_julday(year:number,month:number,day:number,hour:number,calendarFlag:0|1):number;
  swe_revjul(julianDay:number,calendarFlag:0|1):Readonly<{year:number;month:number;day:number;hour:number}>;
  swe_calc_ut(julianDay:number,body:-1,flags:0,callback?:(result:ExperimentalEclipticNutation)=>void):Readonly<ExperimentalEclipticNutation>;
  swe_calc_ut(julianDay:number,body:number,flags:number,callback?:(result:ExperimentalSwissResult|ExperimentalEquatorialResult|ExperimentalEclipticNutation|ExperimentalLunarPointResult|ExperimentalLunarHeliocentricSentinel|ExperimentalLunarEquatorialResult)=>void):Readonly<ExperimentalSwissResult|ExperimentalEquatorialResult|ExperimentalEclipticNutation|ExperimentalLunarPointResult|ExperimentalLunarHeliocentricSentinel|ExperimentalLunarEquatorialResult>;
  swe_fixstar_ut(name:string,julianDay:number,flags:number,callback?:(result:ExperimentalStarResult)=>void):Readonly<ExperimentalStarResult>;
  swe_fixstar_mag(name:string,callback?:(result:Readonly<{name:string;magnitude:number;catalogId:string;model:'CALLER_SUPPLIED_CATALOG_MAGNITUDE'}>)=>void):Readonly<{name:string;magnitude:number;catalogId:string;model:'CALLER_SUPPLIED_CATALOG_MAGNITUDE'}>;
  swe_houses(julianDay:number,latitudeDeg:number,longitudeDegEast:number,system:'V'|'P'|'K'|'R'|'O',callback?:(result:ExperimentalHouses)=>void):Readonly<ExperimentalHouses>;
  swe_houses_pos(armcDeg:number,latitudeDeg:number,trueObliquityDeg:number,system:'P',longitudeDeg:number,latitudeEclipticDeg:number,callback?:(result:ExperimentalHousePosition)=>void):Readonly<ExperimentalHousePosition>;
}
export interface ExperimentalSwissOptions {executablePath:string;kernelPath:string;convertUt1ToTtTdb:(ut1JulianDay:number)=>{ttJulianDay:number;tdbJulianDay:number};catalog?:Readonly<{id:string;records:readonly ExperimentalStarRecord[]}>;houseSupport?:Readonly<ExperimentalHouseSupport>;horizontalSupport?:Readonly<{refraction:'GEOMETRIC'|'NOAA_STANDARD'}>;solarEventSupport?:Readonly<{windowDays:number;stepMinutes:number;timeOffsets:'CONSTANT_IN_SEARCH'}>;starSolarDeflection?:'OMIT'|'POINT_MASS';planetSolarDeflection?:'OMIT'|'POINT_MASS';lunarPointPolicy?:'DISABLED'|'EXPERIMENTAL_GEOCENTRIC';lunarHeliocentricPolicy?:'REJECT'|'ZERO_SENTINEL';physicalCenterKernels?:Readonly<Partial<Record<'Mars'|'Jupiter'|'Saturn'|'Uranus'|'Neptune'|'Pluto',string>>>;}
export interface ExperimentalMutableHouses {house:number[];ascendant:number;mc:number;armc:number;readonly trueObliquity:number;readonly model:'EXPERIMENTAL_IAU2006_2000B_HOUSES';readonly warnings:readonly string[];}
export type ExperimentalMutableStar=Omit<{ -readonly [K in keyof ExperimentalStarResult]: ExperimentalStarResult[K] },'name'|'model'|'warnings'|'catalogId'> & Pick<ExperimentalStarResult,'model'|'warnings'|'catalogId'> & {name?:string};
export interface ExperimentalMutableSubset extends Omit<ExperimentalSwissSubset,'swe_houses'|'swe_fixstar_ut'> {
  swe_houses(julianDay:number,latitudeDeg:number,longitudeDegEast:number,system:'V'|'P'|'K'|'R'|'O',callback?:(result:ExperimentalMutableHouses)=>void):ExperimentalMutableHouses;
  swe_fixstar_ut(name:string,julianDay:number,flags:number,callback?:(result:ExperimentalMutableStar)=>void):ExperimentalMutableStar;
}
export declare function createExperimentalSwissSubset(options:ExperimentalSwissOptions & {legacyMutableResults:true}):Readonly<ExperimentalMutableSubset>;
export declare function createExperimentalSwissSubset(options:ExperimentalSwissOptions & {legacyMutableResults?:false}):Readonly<ExperimentalSwissSubset>;
