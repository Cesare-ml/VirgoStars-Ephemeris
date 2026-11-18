"""Compare long-term mean axes with separately executed ERFA on explicit TT dates.

SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
Shared coefficients make this implementation conformance, not physical validation.
Requires caller-provided pyerfa 2.0.1.5, NumPy 2 and an existing epoch report.
"""
import hashlib
import json
import math
import pathlib
import subprocess
import sys

import erfa
import numpy as np

ARCSEC_PER_RAD = 180.0 * 3600.0 / math.pi
# Swiss JPL, J2000, equatorial and ICRS flags; NONUT may be returned for J2000.
FIXED_FLAGS = 1 | 32 | 2048 | 131072
# Reject geometric, heliocentric, barycentric, topocentric, sidereal,
# Cartesian, radian and explicitly omitted aberration/deflection results.
FORBIDDEN_FLAGS = 16 | 8 | 512 | 1024 | 32768 | 65536 | 16384 | 4096 | 8192


def check_fixed_flags(expected, returned):
    return (type(expected) is int and expected == FIXED_FLAGS and type(returned) is int
            and returned & FIXED_FLAGS == FIXED_FLAGS and returned & FORBIDDEN_FLAGS == 0)


def digest(path):
    h = hashlib.sha256()
    with open(path, "rb") as stream:
        for chunk in iter(lambda: stream.read(1048576), b""):
            h.update(chunk)
    return h.hexdigest()


def compare(local, reference):
    """Maximum component and axis-angle residuals for proper rotation matrices."""
    if local.shape != reference.shape or len(local.shape) != 3 or local.shape[1:] != (3, 3):
        raise ValueError("wrong matrix shape")
    for matrices in (local, reference):
        if not np.isfinite(matrices).all():
            raise ValueError("nonfinite matrix")
        gram = matrices @ matrices.transpose(0, 2, 1)
        if np.max(np.abs(gram - np.eye(3))) > 1e-12 or np.max(np.abs(np.linalg.det(matrices) - 1.0)) > 1e-12:
            raise ValueError("matrix is not a proper rotation")
    components = np.max(np.abs(local - reference), axis=(1, 2))
    angles = np.arctan2(np.linalg.norm(np.cross(local, reference), axis=2), np.sum(local * reference, axis=2))
    return components, np.max(angles, axis=1) * ARCSEC_PER_RAD


def main():
    if len(sys.argv) != 5 or erfa.__version__ != "2.0.1.5" or int(np.__version__.split(".")[0]) < 2:
        raise ValueError("usage: PYTHON audit_erfa_long_term_axes.py EPOCH_REPORT REPORT_SHA PROJECTOR NEW_REPORT; requires pyerfa 2.0.1.5 and NumPy 2")
    source, source_hash, projector, output = sys.argv[1:]
    inputs = [pathlib.Path(source).resolve(), pathlib.Path(projector).resolve(), pathlib.Path(__file__).resolve(), pathlib.Path(sys.executable).resolve()]
    output = pathlib.Path(output).resolve()
    if len(set([*inputs, output])) != 5 or output.exists():
        raise ValueError("inputs and new output must differ")
    hashes = [digest(p) for p in inputs]
    if hashes[0] != source_hash or len(source_hash) != 64:
        raise ValueError("source hash mismatch")
    data = json.loads(inputs[0].read_text(encoding="utf8"))
    source_rows = data.get("rows")
    schema = data.get("schema")
    if not isinstance(source_rows, list):
        raise ValueError("invalid source grid")
    if schema == "erfa-nutation-grid/1" and len(source_rows) == 3887:
        rows = source_rows
    elif schema == "swiss-fixed-icrs-grid/1" and data.get("grid") in ("baseline", "interleaved") and len(source_rows) == {"baseline": 2760, "interleaved": 2720}[data["grid"]]:
        if not all(check_fixed_flags(data.get("flags"), r.get("rflag")) and type(r.get("id")) is int and 0 <= r["id"] < 10 and type(r.get("referenceRaDeg")) in (int, float) and 0 <= r["referenceRaDeg"] < 360 and type(r.get("referenceDecDeg")) in (int, float) and -90 <= r["referenceDecDeg"] <= 90 for r in source_rows):
            raise ValueError("invalid fixed direction or flags")
        for i in range(0, len(source_rows), 10):
            group = source_rows[i:i + 10]
            if [r["id"] for r in group] != list(range(10)) or any((r["jdTt"], r["year"], r["month"]) != (group[0]["jdTt"], group[0]["year"], group[0]["month"]) for r in group):
                raise ValueError("invalid fixed-direction epoch group")
        rows = list({r["jdTt"]: r for r in source_rows}.values())
        if len(rows) != len(source_rows) // 10:
            raise ValueError("unexpected direction epoch count")
    else:
        raise ValueError("invalid source grid")
    epochs = [row.get("jdTt") for row in rows]
    if any(isinstance(x, bool) or not isinstance(x, (int, float)) or not math.isfinite(x) for x in epochs) or len(set(epochs)) != len(epochs) or epochs != sorted(epochs):
        raise ValueError("invalid source epochs")
    reference_files = {
        "erfaInitSha256": pathlib.Path(erfa.__file__).resolve(),
        "erfaCoreSha256": pathlib.Path(erfa.core.__file__).resolve(),
        "erfaBinarySha256": pathlib.Path(erfa.ufunc.__file__).resolve(),
        "numpyInitSha256": pathlib.Path(np.__file__).resolve(),
        "numpyBinarySha256": pathlib.Path(np._core._multiarray_umath.__file__).resolve(),
    }
    ref_hashes = {key: digest(file) for key, file in reference_files.items()}
    text = subprocess.run([str(inputs[1])], input="".join(f"{jd:.17g}\n" for jd in epochs), text=True, capture_output=True, check=True, timeout=30).stdout
    lines = text.splitlines()
    if len(lines) != len(epochs):
        raise ValueError("wrong projector row count")
    local = []
    for jd, line in zip(epochs, lines):
        fields = line.split(",")
        if len(fields) != 19 or not all(x.strip() for x in fields):
            raise ValueError("invalid projector row")
        values = [float(x) for x in fields]
        if values[0] != jd:
            raise ValueError("projector epoch mismatch")
        local.append(values[1:])
    local = np.asarray(local).reshape((-1, 2, 3, 3))
    epj = 2000.0 + (np.asarray(epochs) - 2451545.0) / 365.25
    eq_ref = erfa.ltp(epj)
    ecl_pole = erfa.ltpecl(epj)
    eq_pole = erfa.ltpequ(epj)
    ecl_ref = np.stack((eq_ref[:, 0, :], np.cross(ecl_pole, eq_ref[:, 0, :]), ecl_pole), axis=1)
    if not np.isfinite(eq_pole).all() or np.max(np.abs(eq_ref[:, 2, :] - eq_pole)) > 2e-14:
        raise ValueError("ERFA equator pole consistency failed")
    eq_component, eq_angle = compare(local[:, 0], eq_ref)
    ecl_component, ecl_angle = compare(local[:, 1], ecl_ref)
    if max(eq_component.max(), ecl_component.max()) >= 2e-14 or max(eq_angle.max(), ecl_angle.max()) >= 1e-8:
        raise ValueError("ERFA implementation conformance failed")
    def maximum(values):
        i = int(np.argmax(values))
        return {"value": float(values[i]), "jdTt": epochs[i], "year": rows[i]["year"], "month": rows[i]["month"]}
    direction_summary = None
    if schema == "swiss-fixed-icrs-grid/1":
        lookup = {jd: i for i, jd in enumerate(epochs)}
        indices = np.array([lookup[r["jdTt"]] for r in source_rows])
        ra = np.radians([r["referenceRaDeg"] for r in source_rows])
        dec = np.radians([r["referenceDecDeg"] for r in source_rows])
        vectors = np.column_stack((np.cos(dec) * np.cos(ra), np.cos(dec) * np.sin(ra), np.sin(dec)))
        # Apply the same IAU 2006 J2000 frame bias to both model projections;
        # this compares the mean precession only, not the bias itself.
        bias = erfa.bp06(2451545.0, 0.0)[0]
        vectors = vectors @ bias.T
        def directions(matrix):
            return np.einsum("nij,nj->ni", matrix[indices], vectors)
        def separation(a, b):
            return np.arctan2(np.linalg.norm(np.cross(a, b), axis=1), np.sum(a * b, axis=1)) * ARCSEC_PER_RAD
        def direction_max(values):
            i = int(np.argmax(values))
            return {"arcsec": float(values[i]), "jdTt": source_rows[i]["jdTt"], "year": source_rows[i]["year"], "month": source_rows[i]["month"], "id": source_rows[i]["id"]}
        eq_dirs = separation(directions(local[:, 0]), directions(eq_ref))
        ecl_dirs = separation(directions(local[:, 1]), directions(ecl_ref))
        if not np.isfinite(eq_dirs).all() or not np.isfinite(ecl_dirs).all() or max(eq_dirs.max(), ecl_dirs.max()) >= 1e-8:
            raise ValueError("planetary direction conformance failed")
        direction_summary = {"cases": len(source_rows), "equatorial": direction_max(eq_dirs), "ecliptic": direction_max(ecl_dirs)}
    if [digest(p) for p in inputs] != hashes or {key: digest(file) for key, file in reference_files.items()} != ref_hashes:
        raise ValueError("input changed during evaluation")
    report = {"schema": "erfa-long-term-axes/1", "status": "EXPLORATORY", "sourceSha256": hashes[0], "projectorSha256": hashes[1], "scriptSha256": hashes[2], "pythonSha256": hashes[3], **ref_hashes,
              "pythonVersion": sys.version.split()[0], "numpyVersion": np.__version__, "pyerfaVersion": erfa.__version__, "erfaVersion": erfa.version.erfa_version, "sofaVersion": erfa.version.sofa_version,
              "notes": "TT; mean J2000 equator to mean equator/ecliptic of date; same Vondrak coefficients, no bias/nutation. Implementation agreement is not physical ancient accuracy.",
              "cases": len(epochs), "sourceGrid": data.get("grid"), "directionSummary": direction_summary, "summary": {"equatorialComponent": maximum(eq_component), "equatorialAxisArcsec": maximum(eq_angle), "eclipticComponent": maximum(ecl_component), "eclipticAxisArcsec": maximum(ecl_angle)},
              "rows": [{"jdTt": jd, "equatorialComponent": float(a), "equatorialAxisArcsec": float(b), "eclipticComponent": float(c), "eclipticAxisArcsec": float(d)} for jd, a, b, c, d in zip(epochs, eq_component, eq_angle, ecl_component, ecl_angle)]}
    with output.open("x", encoding="utf8", newline="\n") as stream:
        json.dump(report, stream, allow_nan=False, indent=2)
        stream.write("\n")
    print(json.dumps(report["summary"], allow_nan=False))


if __name__ == "__main__":
    main()
