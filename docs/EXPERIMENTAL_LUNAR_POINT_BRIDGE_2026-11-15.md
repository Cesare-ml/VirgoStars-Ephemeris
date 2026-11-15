# Experimental geocentric lunar-point bridge

Set `lunarPointPolicy:'EXPERIMENTAL_GEOCENTRIC'` in the experimental subset or
legacy bridge to opt into `SE_MEAN_NODE` (10), `SE_TRUE_NODE` (11),
`SE_MEAN_APOG` (12) and `SE_OSCU_APOG` (13). The default remains `DISABLED`.
The caller must supply explicit, coherent UT1→TT/TDB conversions as for the
planetary functions. Only geocentric ecliptic-of-date results with JPL and an
optional speed flag are supported. In the separately configured legacy bridge
the Swiss source flag may be mapped to JPL with an explicit warning. Requests
for heliocentric or equatorial points or unsupported flags fail; no fallback
or fabricated coordinates are returned.

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
for heliocentric and equatorial representations of these points, and for
Chiron. Those requests need their own documented definitions, data and tests.
