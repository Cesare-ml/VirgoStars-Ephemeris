import type {ExperimentalLegacyOptions,ExperimentalLegacyBridge} from './experimental-legacy-bridge.mjs' with { 'resolution-mode': 'import' };
import type {ExperimentalSwissSubset,ExperimentalMutableSubset} from './experimental-swiss-subset.mjs' with { 'resolution-mode': 'import' };
export declare function loadExperimentalLegacyBridge(options:ExperimentalLegacyOptions & {mutableResults:'LEGACY'}):Promise<ExperimentalLegacyBridge<ExperimentalMutableSubset>>;
export declare function loadExperimentalLegacyBridge(options:ExperimentalLegacyOptions & {mutableResults:'FROZEN'}):Promise<ExperimentalLegacyBridge<ExperimentalSwissSubset>>;
