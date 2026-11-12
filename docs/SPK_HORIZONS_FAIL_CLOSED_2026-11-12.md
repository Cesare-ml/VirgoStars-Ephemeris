# Fail-closed SPK precedence and Horizons CSV validation

**2026-11-12 — experimental JavaScript-reference safety fix, not a Rust production feature.**

## SPK segment selection

The JavaScript prototype `src/spk.mjs` now selects the **last covering segment per target** in DAF file order **before** checking the segment's frame and type. This honors the SPK/DAF priority rule within one loaded kernel. If the selected highest-priority segment is not in supported NAIF frame ID 1 (J2000), the function raises `UNSUPPORTED_SPK_FRAME` rather than silently using an older segment in the supported frame. An unsupported high-priority SPK data type also fails instead of falling back. The chosen segment's center determines the target-center chain.

Scope is **one supplied SPK Buffer only**; multiple-kernel furnishing and load-order priority, non-J2000 frame transformations, broader SPK types, and full center/observer coverage remain unsupported and require a separately validated design.

## Horizons reference integrity

The development-only parser `parseHorizonsVector` rejects missing/malformed JD or XYZ/VXYZ numeric cells (including cells JavaScript would otherwise turn into zero), missing calendar labels, unexpected extra columns, non-finite values, and multi-row responses. It accepts a single optional terminal comma and conventional decimal/scientific notation. The online manual script now passes the requested JD(TDB) and refuses a reference row whose returned date differs by more than `1e-8` day (less than one millisecond).

The numerical epoch check is a **comparison safeguard**, not proof that the remote service used the same JPL ephemeris solution or corrections. The script must still record file SHA-256, requested target, center, J2000 ICRF geometric frame, TDB epoch and km/km/s units; scientific parity with real kernels is **not** established by these tests.

## Validation and boundaries

- Deterministic synthetic DAF records cover segment overlap, inclusive boundaries, later segments outside coverage, unsupported later frame/type, center-chain changes and both endiannesses.
- Synthetic CSV tests cover exact row shape, omitted or malformed fields, scientific notation, multi-row text and returned/requested JD mismatch.
- Run `npm test` with Node.js 20+ (CI uses Node.js 22). No online call or JPL data is needed.
- The canonical astronomical engine remains **Rust**. These are defensive fixes to a historical research prototype and to its independent-validation tooling; they are not new astronomical algorithms or shipped bindings.
- **Not covered:** authentic NAIF SPK and JPL Horizons cross-checks, multiple loaded-kernel precedence, frame transformations, timezone/TT/TDB model matching, and production migration.

No third-party data, kernels, dependencies or vendored source have been added.

## References

- [NAIF SPK Required Reading — segment selection and priority](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html)
- [JPL Horizons API](https://ssd.jpl.nasa.gov/doc/horizons.html)
- [Repository validation protocol](VALIDATION.md)

### Modification record

2026-11-12: Updated experimental `src/spk.mjs`, `tools/horizons.mjs`, `scripts/compare-horizons.mjs`; added `test/spk-horizons-fail-closed.test.mjs`. Original-source license: CPAL-1.0, including VirgoStars required attribution (see [LICENSE](../LICENSE) and [NOTICE](../NOTICE)).
