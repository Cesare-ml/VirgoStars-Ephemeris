import type {ExperimentalSwissOptions,ExperimentalSwissSubset,ExperimentalMutableSubset,ExperimentalSolarEvent} from './experimental-swiss-subset.mjs';

export interface ExperimentalLegacyOptions extends ExperimentalSwissOptions {
  ephemerisDirectory:string;
  defaultCalendar:'GREGORIAN'|'JULIAN';
  flagPolicy:'MAP_SWISS_AND_DEFAULT_TO_JPL';
  mutableResults:'LEGACY'|'FROZEN';
}
export interface ExperimentalLegacyMethods {
  swe_set_ephe_path(path:string):string;
  swe_julday(year:number,month:number,day:number,hour:number,calendarFlag?:0|1):number;
  swe_rise_trans(julianDay:number,body:0,starName:'',flags:number,event:1|2|4|8,longitudeDegEast:number,latitudeDeg:number,heightMetres:0,pressureHpa:0,temperatureC:0,callback?:(result:ExperimentalSolarEvent)=>void):Readonly<ExperimentalSolarEvent>;
}
export type ExperimentalLegacyBridge<T> = Readonly<Omit<T,'swe_julday'|'swe_rise_trans'> & ExperimentalLegacyMethods>;
export declare function createExperimentalLegacyBridge(options:ExperimentalLegacyOptions & {mutableResults:'LEGACY'}):ExperimentalLegacyBridge<ExperimentalMutableSubset>;
export declare function createExperimentalLegacyBridge(options:ExperimentalLegacyOptions & {mutableResults:'FROZEN'}):ExperimentalLegacyBridge<ExperimentalSwissSubset>;
