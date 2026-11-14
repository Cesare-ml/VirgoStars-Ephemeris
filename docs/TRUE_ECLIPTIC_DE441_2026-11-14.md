# Experimental true ecliptic positions and rates

The Rust core composes the classic DE441 reader with Earth-centered reception
light-time, annual aberration, IAU 2006 precession and IAU 2000B nutation.
The output is in the moving true ecliptic of date; it provides longitude and
latitude in degrees, distance in AU, and all three derivatives per TDB day.
The moving-frame derivative uses a centered 0.01-day matrix difference,
assuming dTT/dTDB = 1. Planet identifiers retain the actual DE441 barycenter
where a physical planet center is unavailable. No external data is bundled.

An offline example reads the file locally and prints ten bodies as CSV:

```powershell
cargo run --release --example true_ecliptic_at -- C:/data/de441.eph 2026-11-10T00:00:00Z
```

The CSV is printed only after all ten calculations succeed. The Rust API
`true_ecliptic::geocentric_true_ecliptic_from_civil` accepts a
`first_sky::CivilTimestamp` with explicit UTC offset. The supported civil
interval is 1972-01-01 through 2027-06-27 UTC, as bounded by the embedded
leap-second table; leap-second labels are rejected. The TDB−TT conversion is
a truncated approximation, and single-f64 JDs limit subsecond resolution.
The lower-level `geocentric_true_ecliptic` takes separate, coherent TT and TDB
JDs. The one-second agreement guard detects gross errors but does not establish
that the epochs are genuinely equivalent. Frame derivatives close to the
1900/2100 model limits fail when either numerical sample leaves the interval.

For Node, build the example with `cargo build --release --example
true_ecliptic_at`, then explicitly supply the resulting binary and local
kernel to the thin process adapter:

```js
import { createExperimentalRustCli } from 'virgostars-ephemeris/experimental-rust-cli';
const ephemeris = createExperimentalRustCli({
  executablePath: 'C:/path/to/true_ecliptic_at.exe',
  kernelPath: 'C:/data/de441.eph',
});
const sky = ephemeris.calculate('2026-11-10T00:00:00Z');
console.log(sky.results[0].longitudeDeg);
const jd = ephemeris.julianDay({year: 2000, month: 1, day: 1,
  decimalHour: 12, calendar: 'gregorian', timeScale: 'ut1'});
const date = ephemeris.reverseJulianDay({julianDay: jd,
  calendar: 'gregorian', timeScale: 'ut1'});
const lunar = ephemeris.calculateLunarPoints('2026-11-10T00:00:00Z');
```

The synchronous adapter starts one process per ten-body batch and is intended
for isolated experimentation. It validates the CSV contract and returns
immutable, explicitly labelled results; it does not reimplement astronomy.
The Julian-day methods are strict calendar arithmetic on the selected
proleptic calendar and time-scale label. They do not convert UTC to UT1, nor
normalize invalid dates or hours outside [0, 24). They are not ABI-compatible
replacements for `swe_julday` and `swe_revjul`.

`ephemeris.calculateHouseAngles({ut1JulianDay, ttJulianDay, latitudeDeg,
longitudeDegEast})` separately returns ARMC, Ascendant, Midheaven and true
obliquity, in degrees. Both dates must refer to the same instant and UT1 must
be supplied explicitly; UTC is not silently substituted. Sidereal time uses
an approximate equation of the equinoxes without complementary terms and does
not include polar motion. Geographic poles and singular intersections fail.
The equivalent Rust functions are `house_angles::from_epochs` and
`house_angles::from_armc`; they supply geometry, not house-system cusps.

This is **not** a drop-in `swe_calc_ut`: there is no solar gravitational
deflection, complete nutation, full ephemeris of physical planet centers,
Chiron, topocentric options, heliocentric/equatorial modes,
legacy flags or native Node binding. The modeled derivatives of the corrected
position are not assumed identical to Swiss output speeds. A UT Julian date
alone cannot be interpreted as UTC, TT or TDB without an explicit time model.

## Validation and provenance

The offline regression suite covers analytic/synthetic fixtures and an ERFA
nutation/matrix conformance sample. An optional local-file test checks the
moving-frame derivatives against centered position differences for six bodies.
The DE441 light-time stage has four single-epoch reference vectors recorded
from Horizons; see [the light-time documentation](GEOCENTRIC_LIGHT_TIME_2026-11-10.md).
These do not establish independent accuracy for the full composed pipeline or
Swiss numerical equivalence. The existing CPAL notices and BSD-3 ERFA
coefficient attribution in `THIRD_PARTY_NOTICES.md` apply.

An exploratory local comparison on 2026-11-14 used the installed Node
`swisseph` 0.5.17 with the same local DE441 file, `swe_calc` at the same TT
epoch, `SEFLG_JPLEPH | SEFLG_SPEED` (returned flags 257), and the ten targets
in the CLI. At 1980-01-01, 2000-01-01 12:00, and 2026-11-10 UTC, maximum
absolute longitude differences across the ten targets were respectively
0.00000666°, 0.00000556°, and 0.00000146°. Maximum absolute radial-speed
differences were 0.00005195, 0.00005056, and 0.00003570 AU/day. This small
comparison is diagnostic only: target-center models and correction sets differ,
the reference is not independent JPL validation, and the speed discrepancies
mean that Swiss equivalence must not be claimed. `swe_calc_ut` additionally
uses a different UT-to-TT policy; supplying UTC as UT1 can change results.

The separate [experimental lunar-point model](LUNAR_NODES_AND_APOGEES_2026-11-10.md)
has its own assumptions, reference checks and limitations.

2026-11-14: reconciled the existing civil/aberration and 2000B development
branches, added the moving-frame projection and CLI, and corrected the two
reserved `SE_ECL_NUT` array fields to zero according to the public Swiss API
description. No Swiss source code or third-party kernel is copied or bundled.
