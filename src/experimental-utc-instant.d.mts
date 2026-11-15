export interface ExplicitUtcInstantOptions {
  executablePath:string;datetimeUtc:string;ut1MinusUtcSeconds:number;taiMinusUtcSeconds:number;tdbMinusTtSeconds:number;sourceId:string;
}
export interface ExplicitUtcInstant {
  readonly datetimeUtc:string;readonly unixMilliseconds:number;readonly ut1JulianDay:number;readonly ttJulianDay:number;readonly tdbJulianDay:number;readonly sourceId:string;readonly warnings:readonly string[];
  convertUt1ToTtTdb(jd:number):Readonly<{ttJulianDay:number;tdbJulianDay:number}>;
  convertUt1ToUtc(jd:number):string;
}
export declare function createExplicitUtcInstant(options:ExplicitUtcInstantOptions):Readonly<ExplicitUtcInstant>;
