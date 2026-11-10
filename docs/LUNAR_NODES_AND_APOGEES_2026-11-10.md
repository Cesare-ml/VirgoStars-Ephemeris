# Lunar Mathematical Points: Nodes and Apogees

**Status:** implemented in Rust core (`crates/ephemeris-core/src/lunar_points.rs`), 2026-11-10.
Part 1 of work package F in [BACKEND_WORK_PACKAGES_2026-11-10.md](BACKEND_WORK_PACKAGES_2026-11-10.md).

## Scientific foundation

Lunar nodes and apogees are mathematical points defined by the geometry and orbital
mechanics of the Earth–Moon system, **not** physical celestial bodies:
- They have no physical radius, surface or mass.
- They do not emit or reflect photons, and thus undergo no stellar aberration or gravitational light deflection.
- Node latitudes are identically $0^\circ$ by definition (intersection with the ecliptic plane).

### Implemented points and algorithms

1. **Mean Lunar Node (`SE_MEAN_NODE`, ID 10)**:
   - Evaluated from the published Simon et al. (1994) / Chapront & Chapront-Touzé (1988) ELP 2000-82 polynomial in TT centuries $T = (JD_{TT} - 2451545.0) / 36525.0$:
     $$\Omega_{\text{mean}}(T) = 125.0445550^\circ - 1934.13626197^\circ T + 0.0020756^\circ T^2 + 0.000002139^\circ T^3 - 0.00000001650^\circ T^4$$
   - Mean retrograde rate at J2000: $\approx -0.0529538^\circ/\text{day}$ ($\approx 18.61$ Julian years per full cycle).
   - In apparent mode, IAU 2000B nutation in longitude ($\Delta\psi$) is added.

2. **Mean Lunar Apogee / Black Moon Lilith (`SE_MEAN_APOG`, ID 12)**:
   - Evaluated from Chapront ELP-2000 mean perigee $+ 180^\circ$:
     $$\varpi_{\text{mean}}(T) = 83.3532431^\circ + 4069.01774306^\circ T - 0.010325^\circ T^2 - 0.00001249^\circ T^3$$
     $$\text{Apogee}_{\text{mean}}(T) = (\varpi_{\text{mean}}(T) + 180^\circ) \pmod{360^\circ}$$
   - Mean prograde rate at J2000: $\approx +0.1114036^\circ/\text{day}$ ($\approx 8.85$ Julian years per full cycle).
   - Nominal mean apogee distance: $405,400\text{ km}$ ($\approx 0.00271\text{ AU}$).

3. **True Lunar Node (`SE_TRUE_NODE`, ID 11)**:
   - Computed from the Moon's instantaneous orbital angular momentum vector $\vec{h} = \vec{r}_{\text{ecl}} \times \vec{v}_{\text{ecl}}$ in the True Ecliptic of Date:
     $$\vec{N} = (0, 0, 1) \times \vec{h} = (-h_y, h_x, 0)$$
     $$\Omega_{\text{true}} = \text{atan2}(h_x, -h_y) \pmod{360^\circ}$$
   - Latitude is strictly $0^\circ$. Rate is determined from the kinematics of the orbital plane.

4. **Osculating Lunar Apogee / True Lilith (`SE_OSCU_APOG`, ID 13)**:
   - Computed from the Laplace–Runge–Lenz eccentricity vector $\vec{e}$ of the Moon's geocentric osculating orbit:
     $$\vec{e} = \frac{1}{\mu} \left[ \left(v^2 - \frac{\mu}{r}\right)\vec{r} - (\vec{r} \cdot \vec{v})\vec{v} \right]$$
   - The apogee vector points opposite to perigee: $\vec{A} = -\vec{e}$.
   - Longitude and latitude are derived from the direction of $\vec{A}$ in the True Ecliptic of Date; distance is the osculating apogee radius $a(1 + e)$.

## Compatibility contract

`LunarPointCoordinates::to_swiss_array()` produces the exact 6-element layout expected by the Swiss Ephemeris interface:
`[longitude_deg, latitude_deg, distance_au, longitude_rate_deg_per_day, latitude_rate_deg_per_day, distance_rate_au_per_day]`.

## Verification evidence

- **J2000 Mean Node Parity:** verified against the independent Simon et al. (1994) benchmark at $125.044555^\circ$ ($< 10^{-5}$ deg) with retrograde speed $-0.0529538^\circ/\text{day}$.
- **J2000 Mean Apogee Parity:** verified against Chapront ELP-2000 at $263.353243^\circ$ ($< 10^{-5}$ deg) with prograde speed $+0.1114036^\circ/\text{day}$.
- **True Node Orthogonality:** verified that the true ascending node vector lies identically in the ecliptic plane ($Z = 0$).
- **Nutation Invariance:** verified that apparent mode shifts the mean longitude by exactly the IAU 2000B nutation in longitude ($\Delta\psi$).
- **Fail-closed:** non-finite inputs, undefined planes, and circular singular apogees return explicit `LunarPointError` variants without panics.
