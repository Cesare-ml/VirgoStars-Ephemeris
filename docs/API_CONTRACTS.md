# Proposed API contracts (DESIGNED, not implemented)

Goal: an observable/explicit astronomical API consumed by `astrological-functions`. This document is a **design candidate**, not public npm documentation.

## Fundamental rules

All numeric results must carry units, reference frame, origin/center, time scale, coordinate epoch, correction mode, observer (if relevant), source kernel solution and version. A naked `longitude` without frame, center, epoch and apparent/geometric convention is forbidden.

### Proposed TypeScript contract (illustrative)

```ts
type TimeInput = {
  isoUtc: string;          // ISO8601 with explicit offset, leap-second policy defined
  calendar?: "gregorian" | "julian";
};
type FrameId = "ICRF" | "EQUATORIAL_OF_DATE" | "ECLIPTIC_OF_DATE";
type CorrectionId = "GEOMETRIC" | "ASTROMETRIC" | "APPARENT";
type CenterId = "SSB" | "SUN" | "EARTH" | string; // typed NAIF ID mapping eventually
type Observer = { latitudeDeg: number; longitudeDegEast: number; heightMeters: number };
type StateRequest = {
  time: TimeInput;
  target: string;              // e.g. NAIF 301 or stable enum
  center: CenterId;
  frame: FrameId;
  corrections: CorrectionId;
  observer?: Observer;         // mandatory for topocentric requests
  kernelSetId: string;         // always explicit or pinned configured default
};
type StateResult = {
  positionKm: readonly [number, number, number];
  velocityKmPerSec: readonly [number, number, number] | null;
  time: { utc: string; jdTdb: number; jdTt: number; jdUt1?: number };
  provenance: {
    engineVersion: string; algorithmVersion: string;
    kernelSetId: string; kernelHash: string; frame: FrameId;
    corrections: CorrectionId; center: CenterId; target: string;
  };
  warnings: readonly { code: string; detail: string }[];
};
```

This cartesian layer precedes an optional spherical projection returning longitude/latitude in degrees and distance in kilometers/AU, and a separate equatorial projection returning right ascension/declination; derivatives and alt/az are separate capabilities. Refraction is opt-in with specified pressure/temperature; no hidden defaults that alter apparent coordinates.

## Proposed functions

- `loadKernelManifest(manifest)`, `inspectSpk(file)`, `getCoverage(target, center, frame)`
- `getState(request): StateResult`
- `getGeocentricEcliptic(request)`, `getHeliocentricEcliptic(request)`
- `getEquatorial(request)`, `getHorizontal(requestWithObserver)`
- `getTimeScales(input)`, `getSiderealTime(time, longitude, model)`
- Optional (later): `findRiseSet`, `findMeridianTransit`, `findLongitudeCrossing`

Names are working proposals; publish stable TypeScript types only after validation. Avoid recreating the `swe_*` API or copying Swiss structs; `astrological-functions` will adapt VirgoStars legacy shapes externally.

## Error taxonomy

`INVALID_TIME`, `UNSUPPORTED_TIME_SCALE`, `OUT_OF_KERNEL_RANGE`, `MISSING_KERNEL`, `MISSING_CENTER_CHAIN`, `UNSUPPORTED_SPK_TYPE`, `UNSUPPORTED_FRAME`, `UNSUPPORTED_CORRECTIONS`, `PRECISION_REQUIREMENT_UNMET`, `NUMERIC_NON_CONVERGENCE`.

No silently returning zeros, `NaN`, obsolete data, or a lower-precision model. Any fallback must be explicitly requested and annotated.

## Stability gates

- `0.x`: experimental; all changes require regression fixtures.
- `1.0`: documented behavior, documented supported date ranges/targets/frames, reproducible reference comparisons, compatibility/support policy, and offline packaging.
