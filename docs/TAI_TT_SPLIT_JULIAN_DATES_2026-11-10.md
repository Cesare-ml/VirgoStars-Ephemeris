# TAI ↔ TT using split Julian dates

**Date:** 2026-11-10  
**Status:** Experimental Rust implementation on a proposed feature branch.  
**Owner:** VirgoStars-Ephemeris contributors.

## Conventional relationship and reference

The conventional terrestrial/atomic time offset is

```text
TT = TAI + 32.184 SI seconds
TAI = TT - 32.184 SI seconds
```

Authoritative background:

- United States Naval Observatory, *Terrestrial Time (TT)*:
  https://aa.usno.navy.mil/faq/TT
- International Earth Rotation and Reference Systems Service (IERS),
  *How is TT computed from TAI?*:
  https://www.iers.org/iers/en/service/faqs/time/howisttcomputedfromtai-163

The **32.184-second convention** is exact, although practical realizations of
TT and TAI can differ by a few to tens of microseconds. This implementation
is a deterministic conventional offset, not an Earth-orientation observation
or a calculation of real clock errors.

## Rust API and precision

```rust
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::time::{tai_to_tt, tt_to_tai};

let tai = JulianDate::new(2_451_545, 0.5, TimeScale::Tai)?;
let tt = tai_to_tt(tai)?;
assert_eq!(tt.scale(), TimeScale::Tt);
let recovered = tt_to_tai(tt)?;
assert_eq!(recovered.scale(), TimeScale::Tai);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The functions operate on the existing *split* Julian-date representation.
They add/subtract `32.184 / 86_400` days **to the midnight fraction only**,
normalize across midnight with an integer day carry, and preserve
`TimeScale` as part of the result. No premature conversion to a single
`f64` Julian date is required. The input and output split-day integer must
remain in the exact representable range supported by `JulianDate::parts()`.

Both functions reject the wrong input scale with
`AtomicTimeError::WrongInputScale`; bad day ranges propagate as
`AtomicTimeError::InvalidJulianDate`. `tai_to_tt` accepts **only TAI**
and `tt_to_tai` accepts **only TT**. No implicit reinterpretation is allowed.

A normal `f64` fractional-day value and the decimal 32.184 offset cannot
represent arbitrary sub-nanosecond times exactly; the API does not promise
an exact bitwise round-trip. Test tolerances are arithmetic checks, **not
calibrated astronomical uncertainties**.

## Explicit exclusions

This is **not** a conversion from UTC to TAI: leap-second tables,
historical changes and future announcements are not implemented. It is
**not** a TT ↔ TDB model, an ET converter or a correction of SPK/DE441
positions or velocities. The earlier split TDB-to-ET work belongs to
a separate PR and has not been coupled to these functions. `J2000_JD_TDB`
remains a TDB value, not a TT epoch despite sharing the numeric JD.

No network activity, external data or third-party numerical code is used.

## Verification and provenance

The four deterministic Rust tests exercise:

1. **J2000 conventional instant**: 2000-01-01 12:00:00 TT corresponds
   to 11:59:27.816 TAI;
2. addition across midnight and subtraction into the previous day;
3. rejection of wrong input scales (TDB, UT1 and swapped TAI/TT);
4. handling of unsupported/out-of-range split-day values and rollover.

Run, on the actual PR head:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --release --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

GitHub Actions additionally checks native Windows/macOS. Pass/fail results
must be recorded **after** the corresponding runs have completed. These
tests verify the deterministic convention only and do not satisfy the
separate JPL numerical-validation gate.

## License and modification record

2026-11-10: modified `crates/ephemeris-core/src/time.rs` to add
`AtomicTimeError`, `TT_MINUS_TAI_SECONDS`, `tai_to_tt`, `tt_to_tai`,
and deterministic tests, based on the mathematical convention described
by USNO/IERS. This is original Rust source under the repository's existing
**CPAL-1.0**; the file's Exhibit A notice is retained. No Swiss Ephemeris,
NAIF toolkit, kernels, binaries, datasets or dependencies were copied.
The `LICENSE`, `NOTICE`, and attribution **Powered by VirgoStars**
(https://www.virgostars.it/) remain unchanged. No merge, deploy or release.
