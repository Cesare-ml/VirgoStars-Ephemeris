# EXHIBIT A. Common Public Attribution License Version 1.0.
# The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
# Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
# The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
# The Original Developer is the Initial Developer.
# The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
# Contributors retain applicable rights in their own contributions under the terms of this License.
# [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
# SPDX-License-Identifier: CPAL-1.0
# Modification (2026-11-14): verify optional ordered SPK basenames and byte sizes.
# Modification (2026-11-14): detect observable kernel file changes during hashing.

"""Offline SHA-256 preflight for the experimental Rust SPK/NAIF comparator.

Checks supplied kernel hashes, load order and declared names/sizes when present. This does not verify
that a reference was produced by NAIF/SPICE or that its numerical vectors are
correct. No third-party dependencies, downloads or astronomical calculations.
"""

import argparse
import hashlib
import hmac
import os
from pathlib import Path
import sys

CSV_HEADER = "et_tdb_seconds,target_id,center_id,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s"


def read_declared_manifest(
    reference: Path,
) -> tuple[list[str], list[str] | None, list[int] | None]:
    """Parse kernel identity declarations only; Rust owns the full CSV schema.

    The original hash-only CSV format remains supported. When a newer
    reference declares names/sizes, require *both* complete ordered lists.
    This is a byte-identity preflight, not independent SPICE verification.
    """
    metadata: dict[str, str] = {}
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
                if key in (
                    "ordered_kernel_sha256",
                    "ordered_kernel_names",
                    "ordered_kernel_sizes_bytes",
                ):
                    if key in metadata:
                        raise ValueError(f"line {line_no}: duplicate {key}")
                    metadata[key] = value
                continue
            if not saw_header:
                if line != CSV_HEADER:
                    raise ValueError(f"line {line_no}: unexpected CSV header")
                saw_header = True
                continue
            # Astronomical vector rows are validated by the Rust comparator.
    raw_hashes = metadata.get("ordered_kernel_sha256")
    if not saw_header or raw_hashes is None:
        raise ValueError("reference header or ordered_kernel_sha256 metadata missing")
    hashes = [digest.strip().lower() for digest in raw_hashes.split(";")]
    if not hashes or any(
        len(digest) != 64 or not all(char in "0123456789abcdef" for char in digest)
        for digest in hashes
    ):
        raise ValueError("ordered_kernel_sha256 requires 64 hexadecimal characters per kernel")

    raw_names = metadata.get("ordered_kernel_names")
    raw_sizes = metadata.get("ordered_kernel_sizes_bytes")
    if (raw_names is None) != (raw_sizes is None):
        raise ValueError("ordered kernel names and sizes must be declared together")
    if raw_names is None:
        return hashes, None, None  # Legacy hash-only references.

    names = raw_names.split(";")
    size_fields = raw_sizes.split(";")
    if len(names) != len(hashes) or len(size_fields) != len(hashes):
        raise ValueError("ordered kernel names/sizes/hash counts differ")
    if any(
        not name or name in (".", "..") or "/" in name or "\\" in name
        or name != name.strip()
        for name in names
    ):
        raise ValueError("ordered kernel names must be plain, nonempty basenames")
    # Disk sizes are nonnegative integers in canonical decimal notation.
    # Limit digit count to keep untrusted CSV metadata inexpensive to parse.
    if any(
        not size or len(size) > 20 or not size.isascii() or not size.isdecimal()
        or (len(size) > 1 and size[0] == "0")
        for size in size_fields
    ):
        raise ValueError("ordered kernel sizes must be canonical decimal byte counts")
    return hashes, names, [int(size) for size in size_fields]


def read_declared_hashes(reference: Path) -> list[str]:
    """Compatibility API for hash-only callers."""
    return read_declared_manifest(reference)[0]


def _file_identity(stat: os.stat_result) -> tuple[int, int, int, int, int]:
    """Exclude access time; reads may update atime without changing file bytes."""
    return (
        stat.st_dev,
        stat.st_ino,
        stat.st_size,
        stat.st_mtime_ns,
        stat.st_ctime_ns,
    )


def sha256_file(path: Path) -> str:
    """Hash the opened file and reject observable in-flight file replacement.

    Snapshot the path before opening, the descriptor before/after streaming,
    and the path after closing. This detects typical concurrent file edits
    and swaps; it is *not* a race-free lock or a guarantee that a later Rust
    comparator opens the same bytes. Modification (2026-11-14).
    """
    path_before = _file_identity(path.stat())
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        opened = _file_identity(os.fstat(stream.fileno()))
        if opened != path_before:
            raise ValueError(
                f"kernel path changed before SHA-256 read: {path}; "
                f"before={path_before!r}; opened={opened!r}"
            )
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
        if _file_identity(os.fstat(stream.fileno())) != opened:
            raise ValueError(f"kernel bytes or metadata changed during SHA-256 read: {path}")
    if _file_identity(path.stat()) != opened:
        raise ValueError(f"kernel path changed during SHA-256 read: {path}")
    return digest.hexdigest()


def verify(reference: Path, kernels: list[Path]) -> list[str]:
    """Verify every listed kernel; return actual hashes only on full success."""
    expected, names, sizes = read_declared_manifest(reference)
    if len(expected) != len(kernels):
        raise ValueError(f"reference declares {len(expected)} kernels but {len(kernels)} paths were supplied")
    observed = []
    for index, (path, expected_hash) in enumerate(zip(kernels, expected), 1):
        if names is not None and path.name != names[index - 1]:
            raise ValueError(
                f"kernel {index} ({path}): filename/load-order mismatch "
                f"(expected {names[index - 1]!r})"
            )
        if sizes is not None and path.stat().st_size != sizes[index - 1]:
            raise ValueError(
                f"kernel {index} ({path}): byte size mismatch "
                f"(expected {sizes[index - 1]})"
            )
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
    print("SPK kernel identities match declared hashes and optional name/size manifest; NAIF reference provenance is NOT authenticated.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
