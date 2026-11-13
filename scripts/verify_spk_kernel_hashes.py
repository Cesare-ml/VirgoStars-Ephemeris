# EXHIBIT A. Common Public Attribution License Version 1.0.
# The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
# Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
# The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
# The Original Developer is the Initial Developer.
# The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
# Contributors retain applicable rights in their own contributions under the terms of this License.
# [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
# SPDX-License-Identifier: CPAL-1.0

"""Offline SHA-256 preflight for the experimental Rust SPK/NAIF comparator.

Hashes exactly the supplied kernel bytes, in load order. This does not verify
that a reference was produced by NAIF/SPICE or that its numerical vectors are
correct. No third-party dependencies, downloads or astronomical calculations.
"""

import argparse
import hashlib
import hmac
from pathlib import Path
import sys

CSV_HEADER = "et_tdb_seconds,target_id,center_id,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s"


def read_declared_hashes(reference: Path) -> list[str]:
    """Read the ordered SHA-256 declarations; Rust validates the full CSV."""
    declared = None
    saw_header = False
    with reference.open("r", encoding="utf-8") as stream:
        for line_no, raw in enumerate(stream, 1):
            line = raw.strip()
            if not line:
                continue
            if line.startswith("#"):
                if saw_header:
                    raise ValueError(f"line {line_no}: metadata follows CSV header")
                if "=" not in line:
                    raise ValueError(f"line {line_no}: malformed metadata")
                key, value = (part.strip() for part in line[1:].split("=", 1))
                if key == "ordered_kernel_sha256":
                    if declared is not None:
                        raise ValueError(f"line {line_no}: duplicate ordered_kernel_sha256")
                    declared = value.split(";")
                continue
            if not saw_header:
                if line != CSV_HEADER:
                    raise ValueError(f"line {line_no}: unexpected CSV header")
                saw_header = True
                continue
            # All subsequent vector rows are validated by the Rust comparator.
    if not saw_header or declared is None:
        raise ValueError("reference header or ordered_kernel_sha256 metadata missing")
    if any(len(value.strip()) != 64 or not all(c in "0123456789abcdefABCDEF" for c in value.strip()) for value in declared):
        raise ValueError("ordered_kernel_sha256 requires 64 hexadecimal characters per kernel")
    return [value.strip().lower() for value in declared]


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify(reference: Path, kernels: list[Path]) -> list[str]:
    """Verify every listed kernel; return actual hashes only on full success."""
    expected = read_declared_hashes(reference)
    if len(expected) != len(kernels):
        raise ValueError(f"reference declares {len(expected)} kernels but {len(kernels)} paths were supplied")
    observed = []
    for index, (path, expected_hash) in enumerate(zip(kernels, expected), 1):
        actual_hash = sha256_file(path)
        if not hmac.compare_digest(actual_hash, expected_hash):
            raise ValueError(f"kernel {index} ({path}): SHA-256 mismatch (expected {expected_hash}, actual {actual_hash})")
        observed.append(actual_hash)
    return observed


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference", type=Path, help="reviewed NAIF/SPICE reference CSV")
    parser.add_argument("kernels", type=Path, nargs="+", help="SPK kernel paths, in exact load order")
    args = parser.parse_args(argv)
    try:
        observed = verify(args.reference, args.kernels)
    except (OSError, UnicodeError, ValueError) as exc:
        print(f"SPK SHA-256 preflight FAILED: {exc}", file=sys.stderr)
        return 1
    for index, (path, digest) in enumerate(zip(args.kernels, observed), 1):
        print(f"kernel {index}: SHA-256 OK {digest} {path}")
    print("All declared SPK digests match on-disk bytes; NAIF reference provenance is NOT authenticated.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
