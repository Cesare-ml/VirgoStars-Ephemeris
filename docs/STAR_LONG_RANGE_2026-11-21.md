# Long-term fixed-star direction (2026-11-21)

The explicit experimental long-term star path keeps the same catalog motion,
parallax, observer state, annual aberration and selected solar-deflection
correction as the modern path. Its TT orientation uses the Vondrak mean axes,
frame bias, direct obliquity and extrapolated nutation used by the existing
long-term planetary pipeline. The long-term nutation and catalog extrapolation
are approximate; synthetic regression tests do not establish independent
astronomical accuracy. Outside the supported model or DE441 data, the call
returns an error.

`star_direction_with_provider` and
`experimental_long_term_star_direction_with_provider` query only the requested
TDB epoch. The separate rate APIs retain their neighbouring samples. This
allows host applications to show positions at an installed pack boundary
without fabricating data from a neighbouring pack.

Reference: Vondrak, Capitaine and Wallace (2011), A&A 534 A22,
https://doi.org/10.1051/0004-6361/201117274. The existing implementation
includes the separately recorded ERFA coefficient attribution.
