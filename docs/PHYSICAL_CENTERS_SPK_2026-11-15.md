# Experimental physical centers with supplemental SPK kernels

The Rust SPK Type 2/3 reader, index and multi-kernel composition are recovered
from this repository's `dev/ephemeris` commit
`8a20b2ca56eceecc5ee803488ff9c311dd124416`. The physical-center composition
adds an explicit SPK center-minus-system-barycenter state to the classic DE441
system barycenter at **each emission-time iteration**, then applies annual
aberration and the existing IAU 2006/2000B true-ecliptic projection. Position
offsets are km and velocities are converted from km/TDB second to km/TDB day.
The public result identifies the physical planet, not the barycenter. An SPK
without the requested relative state or time coverage is an error.

The opt-in Node subset accepts `physicalCenterKernels: { Mars, Jupiter,
Saturn, Uranus, Neptune, Pluto }`, with each value a path to a separately obtained SPK. Existing
`kernelPath`, executable and explicit UT1-to-TT/TDB converter remain required.
With a suitable kernel, `swe_calc_ut` supports physical centers from Sun through
Pluto. JPL
and optional speed flags are the only supported calculation flags. These are
experimental API shapes, not a drop-in Swiss replacement. Models may combine
DE441 and satellite SPKs derived from different planetary solutions; callers
must assess their compatibility and provenance. There is no solar deflection,
IAU 2000B nutation is approximate, and rates are per TDB day rather than exact
UT1 day. Full-domain accuracy, alternate flags and production integration
remain unvalidated.

## Reproducible limited check

At TDB JD `2461354.500800725` the geometric ICRF relative offsets and
velocities were compared to [NASA Horizons](https://ssd-api.jpl.nasa.gov/doc/horizons.html)
vector queries with `EPHEM_TYPE=VECTORS`, `VEC_CORR=NONE`, `REF_PLANE=FRAME`,
`OUT_UNITS=KM-S`, and target/center pairs `499/4`, `599/5`, `699/6`.
The maximum component difference was under 0.000001 km and 0.0000000001 km/s.
Horizons used mar099, jup365_merged and sat441l respectively; the local files
below use different solutions. This single epoch checks the reader/composition
only locally; it does not establish apparent-position accuracy across time.
The integration test also compares apparent longitude speed to a central
difference and checks failure on missing coverage.

For `799/7`, `899/8`, `999/9` at the same epoch, Horizons used ura184,
nep098 and plu060, while local kernels were ura112, nep097 and plu060.
The maximum position component differences were respectively about 0.472 km,
0.151 km and 0.000000004 km; velocity component differences were below
0.000002 km/s. The looser Uranus and Neptune differences reflect different
solution inputs and prevent using this comparison as a precision claim.

External files used, deliberately not distributed in this repository:

| SPK | SHA-256 | Declared local coverage |
| --- | --- | --- |
| [mar097_20160314_20300101.bsp](https://naif.jpl.nasa.gov/pub/naif/pds/pds4/em16/em16_spice/spice_kernels/spk/mar097_20160314_20300101.bsp) | `EA3830653A92E10415127DD432ACA2577B6CAF98FC863D285A792A96E7848C02` | 2016–2030 |
| [jup348.bsp](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/satellites/jup348.bsp) | `9B04592FB6D5448EBA480C347B60ED5F0629D049E59417C40039021B64662701` | inspect kernel for coverage |
| [sat480.bsp](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/satellites/sat480.bsp) | `12D2CFD8DA371883BD4EDBD67C31FAFA98CA7779A09DA6819E29D53CDC6DAF32` | 1950–2050 |
| [ura112.bsp](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/satellites/a_old_versions/ura112.bsp) | `2F55575D8592C9E51493B7249900869E1843CBAE76E1DA0781D1B442F8A7208D` | 1899–2099 |
| [nep097.bsp](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/satellites/nep097.bsp) | `5C1132FDC48C54E5D2E4EB663E1ED1A876911EAE9433C203F29A5A1B5E1FA9B1` | 1600–2399 |
| [plu060.bsp](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/satellites/plu060.bsp) | `DFBB102491A26ED41AE08CA3F8963F22F0219DF1D8F265AB87B9AD825A826FC6` | 1800–2199 |

To run the ignored integration test, supply `DE441_PATH`, `MARS_SPK_PATH`,
`JUPITER_SPK_PATH`, and `SATURN_SPK_PATH`, then run
`cargo test --lib physical_center --locked -- --include-ignored`.
The outer-planet test additionally requires `URANUS_SPK_PATH`,
`NEPTUNE_SPK_PATH`, and `PLUTO_SPK_PATH`.
The same paths plus `VIRGOSTARS_EPHEMERIS_CLI` enable the Node integration
test. The classic DE441 and SPK files are separately obtained data and do not
inherit this repository's license.
