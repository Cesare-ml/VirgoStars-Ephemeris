export interface ExplicitUtcInstantOptions {
  executablePath:string;datetimeUtc:string;ut1MinusUtcSeconds:number;taiMinusUtcSeconds:number;tdbMinusTtSeconds:number;sourceId:string;
}
export interface ExplicitUtcInstant {
  readonly datetimeUtc:string;readonly unixMilliseconds:number;readonly ut1JulianDay:number;readonly ttJulianDay:number;readonly tdbJulianDay:number;readonly sourceId:string;readonly warnings:readonly string[];
  convertUt1ToTtTdb(jd:number):Readonly<{ttJulianDay:number;tdbJulianDay:number}>;
  convertUt1ToUtc(jd:number):string;
}
export declare function createExplicitUtcInstant(options:ExplicitUtcInstantOptions):Readonly<ExplicitUtcInstant>;
export interface ExplicitUt1WindowOptions extends Omit<ExplicitUtcInstantOptions,'datetimeUtc'> {
  startUtc:string;endUtc:string;offsetPolicy:'CONSTANT_IN_WINDOW';leapSecondPolicy:'NO_LEAP_SECOND_IN_WINDOW';
}
export interface ExplicitUt1Window {
  readonly startUtc:string;readonly endUtc:string;readonly startUt1JulianDay:number;readonly endUt1JulianDay:number;readonly sourceId:string;readonly warnings:readonly string[];
  convertUt1ToTtTdb(jd:number):Readonly<{ttJulianDay:number;tdbJulianDay:number}>;
  convertUt1ToUtc(jd:number):string;
}
export declare function createExplicitUt1Window(options:ExplicitUt1WindowOptions):Readonly<ExplicitUt1Window>;
