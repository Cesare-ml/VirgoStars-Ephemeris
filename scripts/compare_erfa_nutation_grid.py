"""Emit ERFA nutation for supplied TT epochs; no external data bundled.

SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
ERFA models are not observations or accuracy guarantees at distant epochs.
Input is JSON with an `epochs` array of TT Julian dates; output is JSON.
"""
import hashlib
import json
import math
import pathlib
import sys

import erfa
import numpy as np


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def main():
    if len(sys.argv) != 1 or erfa.__version__ != "2.0.1.5":
        raise ValueError("requires pyerfa 2.0.1.5 and TT epochs as JSON on stdin")
    if int(np.__version__.split(".")[0]) < 2:
        raise ValueError("requires NumPy 2 or newer for recorded binary provenance")
    payload = json.load(sys.stdin)
    epochs = payload.get("epochs")
    if not isinstance(epochs, list) or not 1 <= len(epochs) <= 10000:
        raise ValueError("invalid epoch count")
    if any(isinstance(x, bool) or not isinstance(x, (int, float)) or not math.isfinite(x) for x in epochs):
        raise ValueError("invalid TT epoch")
    if len(set(epochs)) != len(epochs):
        raise ValueError("duplicate TT epoch")
    reference_files = {
        "erfaInitSha256": pathlib.Path(erfa.__file__).resolve(),
        "erfaCoreSha256": pathlib.Path(erfa.core.__file__).resolve(),
        "erfaBinarySha256": pathlib.Path(erfa.ufunc.__file__).resolve(),
        "numpyInitSha256": pathlib.Path(np.__file__).resolve(),
        "numpyBinarySha256": pathlib.Path(np._core._multiarray_umath.__file__).resolve(),
    }
    before = {key: sha256(file) for key, file in reference_files.items()}
    dates = np.asarray(epochs, dtype=np.float64)
    a_psi, a_eps = erfa.nut00a(2451545.0, dates - 2451545.0)
    b_psi, b_eps = erfa.nut00b(2451545.0, dates - 2451545.0)
    factor = 180.0 * 3600.0 / math.pi
    values = np.column_stack((a_psi, a_eps, b_psi, b_eps)) * factor
    if not np.isfinite(values).all():
        raise ValueError("nonfinite ERFA result")
    if before != {key: sha256(file) for key, file in reference_files.items()}:
        raise ValueError("reference file changed during evaluation")
    print(json.dumps({"erfaVersion": erfa.__version__, "erfaLibraryVersion": erfa.version.erfa_version,
                      "sofaVersion": erfa.version.sofa_version, "numpyVersion": np.__version__,
                      "pythonVersion": sys.version.split()[0], **before,
                      "rows": [[float(jd), *map(float, row)] for jd, row in zip(dates, values)]},
                     allow_nan=False, separators=(",", ":")))


if __name__ == "__main__":
    main()
