export declare const SE_JUL_CAL:0,SE_GREG_CAL:1,SEFLG_JPLEPH:1,SEFLG_SWIEPH:2,SEFLG_SPEED:256;
export declare const SE_SUN:0,SE_MOON:1,SE_MERCURY:2,SE_VENUS:3,SE_MARS:4,SE_JUPITER:5,SE_SATURN:6,SE_URANUS:7,SE_NEPTUNE:8,SE_PLUTO:9;
export interface ExperimentalSwissResult {
  readonly longitude:number; readonly latitude:number; readonly distance:number;
  readonly longitudeSpeed:number; readonly latitudeSpeed:number; readonly distanceSpeed:number;
  readonly rflag:number; readonly model:'EXPERIMENTAL_DE441_IAU2006_2000B'; readonly warnings:readonly string[];
}
export interface ExperimentalSwissSubset {
  readonly SE_JUL_CAL:0;readonly SE_GREG_CAL:1;readonly SEFLG_JPLEPH:1;readonly SEFLG_SWIEPH:2;readonly SEFLG_SPEED:256;
  readonly SE_SUN:0;readonly SE_MOON:1;readonly SE_MERCURY:2;readonly SE_VENUS:3;readonly SE_MARS:4;readonly SE_JUPITER:5;readonly SE_SATURN:6;readonly SE_URANUS:7;readonly SE_NEPTUNE:8;readonly SE_PLUTO:9;
  swe_julday(year:number,month:number,day:number,hour:number,calendarFlag:0|1):number;
  swe_revjul(julianDay:number,calendarFlag:0|1):Readonly<{year:number;month:number;day:number;hour:number}>;
  swe_calc_ut(julianDay:number,body:number,flags:number,callback?:(result:ExperimentalSwissResult)=>void):Readonly<ExperimentalSwissResult>;
}
export declare function createExperimentalSwissSubset(options:{executablePath:string;kernelPath:string;convertUt1ToTtTdb:(ut1JulianDay:number)=>{ttJulianDay:number;tdbJulianDay:number}}):Readonly<ExperimentalSwissSubset>;
