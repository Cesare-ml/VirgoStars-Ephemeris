"""Compute an ERFA geocentric TDB-TT model for supplied TT Julian dates.

SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
Ancient extrapolation is a model sensitivity, not a historical accuracy claim.
"""
import hashlib
import json
import math
import pathlib
import sys

import erfa
import numpy as np


def digest(path):
    h = hashlib.sha256()
    with open(path, "rb") as stream:
        for block in iter(lambda: stream.read(1048576), b""):
            h.update(block)
    return h.hexdigest()


def main():
    if len(sys.argv) != 1 or erfa.__version__ != "2.0.1.5" or int(np.__version__.split(".")[0]) < 2:
        raise ValueError("requires pyerfa 2.0.1.5, NumPy 2 and input on stdin")
    epochs = json.load(sys.stdin).get("epochs")
    if not isinstance(epochs, list) or not 1 <= len(epochs) <= 10000 or any(isinstance(x, bool) or not isinstance(x, (int, float)) or not math.isfinite(x) for x in epochs) or len(set(epochs)) != len(epochs):
        raise ValueError("invalid epochs")
    files = {"erfaInitSha256": pathlib.Path(erfa.__file__).resolve(), "erfaCoreSha256": pathlib.Path(erfa.core.__file__).resolve(), "erfaBinarySha256": pathlib.Path(erfa.ufunc.__file__).resolve(), "numpyInitSha256": pathlib.Path(np.__file__).resolve(), "numpyBinarySha256": pathlib.Path(np._core._multiarray_umath.__file__).resolve()}
    hashes = {key: digest(file) for key, file in files.items()}
    dates = np.asarray(epochs, dtype=np.float64)
    # u=v=0 selects the geocenter; UT and longitude then do not contribute.
    values = erfa.dtdb(2451545.0, dates - 2451545.0, 0.0, 0.0, 0.0, 0.0)
    if not np.isfinite(values).all() or not np.array_equal(values, erfa.dtdb(2451545.0, dates - 2451545.0, 0.37, 2.0, 0.0, 0.0)):
        raise ValueError("invalid geocentric ERFA result")
    if hashes != {key: digest(file) for key, file in files.items()}:
        raise ValueError("reference changed")
    print(json.dumps({"pyerfaVersion": erfa.__version__, "erfaVersion": erfa.version.erfa_version, "sofaVersion": erfa.version.sofa_version, "numpyVersion": np.__version__, "pythonVersion": sys.version.split()[0], **hashes, "rows": [[float(jd), float(value)] for jd, value in zip(dates, values)]}, allow_nan=False, separators=(",", ":")))


if __name__ == "__main__":
    main()
