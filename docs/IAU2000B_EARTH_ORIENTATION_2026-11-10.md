# IAU 2000B Earth Orientation, Nutation and ECL_NUT

**Status:** implemented in Rust core (`crates/ephemeris-core/src/earth_orientation.rs`), 2026-11-10.
Work package D in [BACKEND_WORK_PACKAGES_2026-11-10.md](BACKEND_WORK_PACKAGES_2026-11-10.md).

## Scientific foundation

- **IAU 2000B Nutation Series**: 77-term luni-solar periodic series with Simon et al. (1994)
  linearized Delaunay arguments and published fixed planetary offsets (-0.135 mas in $\Delta\psi$,
  +0.388 mas in $\Delta\varepsilon$). Basis: McCarthy & Luzum (2003), Celestial Mechanics and
  Dynamical Astronomy 85, 37–49.
- **IAU 2006 Mean Obliquity**: Fukushima–Williams / Hilton et al. (2006) 5th-order polynomial in TT centuries.
- **IAU 2006 Bias-Precession**: Fukushima–Williams 4-angle rotation from ICRF to Mean Equator of Date.
- **True Equator of Date Matrix**: Composition $M_{\text{eq}} = N \cdot P_B$ where $N$ is the nutation
  matrix $R_1(-\varepsilon_0 - \Delta\varepsilon) \cdot R_3(-\Delta\psi) \cdot R_1(\varepsilon_0)$.
- **True Ecliptic of Date Matrix**: $M_{\text{ecl}} = R_1(\varepsilon_{\text{true}}) \cdot M_{\text{eq}}$.
- **Sidereal Time & Equation of Equinoxes**:
  - Earth Rotation Angle $\text{ERA}(UT1)$ strictly evaluated on UT1.
  - $\text{GMST}(UT1, TT) = \text{ERA}(UT1) + p_{\text{ra}}(TT)$, distinguishing UT1 from TT.
  - Equation of the equinoxes $EE = \Delta\psi \cos(\varepsilon_0)$.
  - $\text{GAST}(UT1, TT) = \text{GMST} + EE \pmod{2\pi}$.

## Validated epoch range

- **1900–2100 TT** ($2415020.0 \le JD_{TT} < 2488070.0$).
- Out-of-range epochs and non-finite numeric inputs fail closed with explicit typed `OrientationError`
  variants; no silent fallbacks.

## Independent numerical verification

1. **ERFA `nut00b` oracle at TT JD 2453736.5 (2006-01-15 00:00:00 TT)**:
   - Expected $\Delta\psi = -0.9632552291148362783 \times 10^{-5}$ rad.
   - Expected $\Delta\varepsilon = 0.4063197106621159367 \times 10^{-4}$ rad.
   - Tested agreement: difference $< 10^{-13}$ rad (evaluated difference $\approx 4.6 \times 10^{-20}$ rad).
2. **ERFA `num00b` 3x3 nutation matrix oracle**:
   - Matches all 9 elements of the ERFA reference matrix to $< 10^{-13}$ element-wise
     (maximum deviation $\approx 5.6 \times 10^{-17}$).
3. **Kinematic and Orthogonality Invariants**:
   - $M \cdot M^T = I$ and $\det(M) = 1.0$ for nutation, bias-precession, true equator, and true ecliptic matrices.
   - Euclidean vector lengths are preserved under all 3D rotations ($< 10^{-11}$ relative error).
4. **Swiss Ephemeris `SE_ECL_NUT` Compatibility**:
   - `EclipticNutation::to_swiss_ecl_nut()` outputs `[eps_true_deg, eps_mean_deg, dpsi_deg, deps_deg, ee_deg, 0.0]`.

## Provenance and licensing

The 77 coefficient terms are standard astronomical constants from the BSD-3 licensed ERFA table
(`nut00b.c`), copyrighted by NumFOCUS Foundation (see `THIRD_PARTY_NOTICES.md` and
`third_party/ERFA_LICENSE.txt`). No ERFA C code is copied or translated.

## Limitations

- Free Core Nutation (FCN) is not included in the standard IAU 2000B model.
- Polar motion ($x_p, y_p$) and observed IERS $\Delta UT1$ require EOP tables and are not part of this isolated model.
