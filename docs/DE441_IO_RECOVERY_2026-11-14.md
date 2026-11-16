# Classic DE441 seekable-stream failure and retry regression

Created **2026-11-14**. This is an offline, entirely **synthetic** I/O
regression suite for the existing canonical Rust
`De441<R: Read + Seek>` reader. It does **not** use a real NASA/JPL kernel,
NAIF toolkit, Horizons, network access or external datasets.

## Motivation

The classic DE441 file can be several gigabytes. The Rust reader opens a
seekable source, validates a header, then reads coefficient records on demand.
The public interface accepts arbitrary `Read + Seek` implementations, not just
local `File`. Correct behavior requires:

1. Short but successful `Read::read` calls to be reassembled into whole
   headers and coefficient records, rather than treated as EOF.
2. An interrupted read (`ErrorKind::Interrupted`) to be retried according to
   `Read::read_exact`'s standard contract.
3. A stream error or premature EOF partway through a coefficient record to
   fail with `De441Error::Io` and preserve the original
   `std::io::ErrorKind`.
4. A later valid query on the **same** reader to succeed after the underlying
   stream recovers, with no partial/stale cached state.
5. A failed seek or truncated initial header to return a typed I/O error,
   not panic or produce a plausible astronomical value.

## Fixture and scientific contract

One 32-day synthetic classic-format coefficient record begins at
**JD 2451545.0 TDB**. It describes **Mercury relative to the Solar System
Barycenter**, geometric XYZ in an ICRF-like fixed Cartesian frame, with T0/T1
coefficients deliberately chosen to yield at the midpoint
**[100, 200, 300] km** and **[1, 2, 3] km per TDB day**. The stand-in file
contains no real planetary ephemeris data and is **not** a NAIF SPK file.

The test's custom `Read + Seek` implementation can return at most 3–31
bytes per call, inject one interrupted read, inject one failed seek, or halt
exactly 101 bytes into the coefficient record with `BrokenPipe` or EOF.
Failures are removed explicitly before retrying the identical query.

## Verification

```sh
cargo test -p virgostars-ephemeris-core --locked --test de441_io_recovery
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The 5-test suite is designed to run in the repository's existing read-only
GitHub CI without external files or permissions; the PR should record the
exact-head results before merge.

These tests cannot establish authentic JPL/NAIF/Horizons numerical parity,
real-kernel coverage or correctness of positions and velocities beyond the
explicitly known synthetic polynomial. Scientific acceptance remains gated on
an official kernel and an independently sourced reference matrix.

No public API, binary parser behavior, runtime dependency, release or
production deployment is changed. This addition preserves the repository's
CPAL-1.0 notices and **Powered by VirgoStars** attribution obligations.
