import type {ExperimentalStarRecord} from './experimental-swiss-subset.mjs';
export declare function loadExperimentalStarCatalogFile(options:{filePath:string;sha256:string;source:string;license:string}):{
  readonly catalog:Readonly<{id:string;records:readonly ExperimentalStarRecord[]}>;
  provenance:Readonly<{sha256:string;source:string;license:string}>;
};
