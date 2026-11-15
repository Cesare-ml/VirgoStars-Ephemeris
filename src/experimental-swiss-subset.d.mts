export declare const SE_JUL_CAL:0,SE_GREG_CAL:1,SEFLG_JPLEPH:1,SEFLG_SWIEPH:2,SEFLG_SPEED:256;
export declare const SE_SUN:0,SE_MOON:1,SE_MERCURY:2,SE_VENUS:3,SE_MARS:4,SE_JUPITER:5,SE_SATURN:6,SE_URANUS:7,SE_NEPTUNE:8,SE_PLUTO:9;
export interface ExperimentalSwissResult {
  readonly longitude:number; readonly latitude:number; readonly distance:number;
  readonly longitudeSpeed:number; readonly latitudeSpeed:number; readonly distanceSpeed:number;
  readonly rflag:number; readonly model:'EXPERIMENTAL_DE441_IAU2006_2000B'; readonly warnings:readonly string[];
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
export interface ExperimentalSwissSubset {
  readonly SE_JUL_CAL:0;readonly SE_GREG_CAL:1;readonly SEFLG_JPLEPH:1;readonly SEFLG_SWIEPH:2;readonly SEFLG_SPEED:256;
  readonly SE_SUN:0;readonly SE_MOON:1;readonly SE_MERCURY:2;readonly SE_VENUS:3;readonly SE_MARS:4;readonly SE_JUPITER:5;readonly SE_SATURN:6;readonly SE_URANUS:7;readonly SE_NEPTUNE:8;readonly SE_PLUTO:9;
  swe_julday(year:number,month:number,day:number,hour:number,calendarFlag:0|1):number;
  swe_revjul(julianDay:number,calendarFlag:0|1):Readonly<{year:number;month:number;day:number;hour:number}>;
  swe_calc_ut(julianDay:number,body:number,flags:number,callback?:(result:ExperimentalSwissResult)=>void):Readonly<ExperimentalSwissResult>;
  swe_fixstar_ut(name:string,julianDay:number,flags:number,callback?:(result:ExperimentalStarResult)=>void):Readonly<ExperimentalStarResult>;
  swe_fixstar_mag(name:string,callback?:(result:Readonly<{name:string;magnitude:number;catalogId:string;model:'CALLER_SUPPLIED_CATALOG_MAGNITUDE'}>)=>void):Readonly<{name:string;magnitude:number;catalogId:string;model:'CALLER_SUPPLIED_CATALOG_MAGNITUDE'}>;
  swe_houses(julianDay:number,latitudeDeg:number,longitudeDegEast:number,system:'V'|'P'|'K'|'R'|'O',callback?:(result:ExperimentalHouses)=>void):Readonly<ExperimentalHouses>;
  swe_houses_pos(armcDeg:number,latitudeDeg:number,trueObliquityDeg:number,system:'P',longitudeDeg:number,latitudeEclipticDeg:number,callback?:(result:ExperimentalHousePosition)=>void):Readonly<ExperimentalHousePosition>;
}
export declare function createExperimentalSwissSubset(options:{executablePath:string;kernelPath:string;convertUt1ToTtTdb:(ut1JulianDay:number)=>{ttJulianDay:number;tdbJulianDay:number};catalog?:Readonly<{id:string;records:readonly ExperimentalStarRecord[]}>;houseSupport?:Readonly<ExperimentalHouseSupport>;physicalCenterKernels?:Readonly<Partial<Record<'Mars'|'Jupiter'|'Saturn',string>>>}):Readonly<ExperimentalSwissSubset>;
