# Experimental geocentric lunar-point bridge

Set `lunarPointPolicy:'EXPERIMENTAL_GEOCENTRIC'` in the experimental subset or
legacy bridge to opt into `SE_MEAN_NODE` (10), `SE_TRUE_NODE` (11),
`SE_MEAN_APOG` (12) and `SE_OSCU_APOG` (13). The default remains `DISABLED`.
The caller must supply explicit, coherent UT1→TT/TDB conversions as for the
planetary functions. Only geocentric ecliptic-of-date results with JPL and an
optional speed flag are supported. In the separately configured legacy bridge
the Swiss source flag may be mapped to JPL with an explicit warning. Requests
for heliocentric or equatorial points or unsupported flags fail by default.

For a legacy caller that explicitly expects Swiss's heliocentric zero
convention, additionally set `lunarHeliocentricPolicy:'ZERO_SENTINEL'`. This
returns six zeros labelled `EXPERIMENTAL_LUNAR_HELIOCENTRIC_ZERO_SENTINEL`,
with `HELIOCENTRIC_ZERO_SENTINEL_NO_PHYSICAL_POSITION`: these zeros are a
compatibility sentinel, not a physical position, distance or speed. It requires
the lunar-point opt-in above, validates the supplied time conversion, and
remains disabled by default. The [Swiss programming documentation §5.3](https://www.astro.com/ftp/swisseph/doc/swephprg.2.10.pdf)
documents this convention. `rflag` in this experiment reports the configured
effective flags and does not reproduce the additional flags Swiss may return.
The legacy source translation warning remains accessible in the result.

The four points are mathematical constructions rather than physical sources.
The mean points use approximate polynomials, a fixed orbit inclination and
nominal distances; they are analytical models rather than JPL interpolations,
even when the source flag is JPL. The true points derive from the DE441 geometric lunar state.
Their speeds are per TT day; osculating rates use numerical differences.
No light-time, aberration or gravitational deflection is applied. The flag
describes input/shape compatibility, not complete Swiss semantics. Each result
is labelled `EXPERIMENTAL_GEOCENTRIC_LUNAR_POINT` and carries warnings.

The real-file test compares one sample against the previously documented
Swiss comparison with a deliberately loose tolerance; it tests interface
composition and is not independent scientific validation or an error bound.
See [the lunar model note](LUNAR_NODES_AND_APOGEES_2026-11-10.md) for model
definitions, time scales, data provenance and known differences. No new data,
dependency or independent validation is claimed.

This does not yet make the backend's default chart work: that code also asks
for equatorial representations of these points and for Chiron. Those requests
need their own documented definitions, data and tests. The backend must also
retain or otherwise surface warnings instead of displaying sentinel values as
physical positions.
