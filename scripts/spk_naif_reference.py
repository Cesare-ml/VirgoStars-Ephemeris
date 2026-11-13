# EXHIBIT A. Common Public Attribution License Version 1.0.
# 
# The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
# 
# Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
# 
# The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
# 
# The Original Developer is the Initial Developer.
# 
# The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
# 
# Contributors retain applicable rights in their own contributions under the terms of this License.
# 
# [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
# SPDX-License-Identifier: CPAL-1.0

"""Validation-only independent NAIF/SPICE reference producer and SPK SHA256 verifier.

Production astronomy remains canonical Rust. spiceypy is imported only when
generating actual NAIF vectors; synthetic mocked tests are not proof of parity.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import hmac
import math
from pathlib import Path
import re
import sys
from typing import Any, Sequence

COLUMNS = "et_tdb_seconds,target_id,center_id,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s"
ID_PATTERN = re.compile(r"[+-]?[0-9]+\Z")
HEX_SHA256_PATTERN = re.compile(r"[0-9a-fA-F]{64}\Z")
SCRIPT_NAME = "scripts/spk_naif_reference.py"


def case_spec(value: str) -> tuple[int, int, float]:
    fields = value.split(":")
    if len(fields) != 3:
        raise ValueError("case must be TARGET_ID:CENTER_ID:ET_TDB_SECONDS")
    for part in fields[:2]:
        if not ID_PATTERN.fullmatch(part):
            raise ValueError("target and center must be integer NAIF IDs")
    target, center = (int(part) for part in fields[:2])
    if not all(-2**31 <= item < 2**31 for item in (target, center)):
        raise ValueError("NAIF IDs must fit Rust i32")
    try:
        et = float(fields[2])
    except ValueError as error:
        raise ValueError("ET must be numeric TDB seconds past J2000") from error
    if not math.isfinite(et):
        raise ValueError("ET must be finite TDB seconds past J2000")
    if target == center:
        raise ValueError("target and center must differ")
    return target, center, et


def parse_cases(values: Sequence[str]) -> list[tuple[int, int, float]]:
    if not values:
        raise ValueError("at least one --case must be supplied explicitly")
    result: list[tuple[int, int, float]] = []
    seen: set[tuple[int, int, float]] = set()
    for value in values:
        item = case_spec(value)
        if item in seen:
            raise ValueError("duplicate target/center/ET case")
        seen.add(item)
        result.append(item)
    return result


def kernel_manifest(paths: Sequence[Path]) -> list[tuple[str, int, str]]:
    if not paths:
        raise ValueError("at least one --kernel is required")
    manifest = []
    for path in paths:
        if not path.is_file():
            raise ValueError(f"SPK kernel not found: {path}")
        sha = hashlib.sha256()
        with path.open("rb") as source:
            while chunk := source.read(1024 * 1024):
                sha.update(chunk)
        manifest.append((path.name, path.stat().st_size, sha.hexdigest()))
    return manifest


def _metadata_text(key: str, value: object) -> str:
    line = str(value)
    if not line or "\n" in line or "\r" in line or "=" in key:
        raise ValueError(f"invalid metadata {key}")
    return f"# {key}={line}"


def generate_reference(
    paths: Sequence[Path],
    cases: Sequence[tuple[int, int, float]],
    spice: Any,
    *,
    generated_utc: str,
    generator_digest: str,
) -> str:
    """Produce Rust-compatible numeric CSV using an independent SPICE binding."""
    if not cases:
        raise ValueError("cannot generate an empty reference")
    if not HEX_SHA256_PATTERN.fullmatch(generator_digest):
        raise ValueError("invalid generator SHA256")
    before = kernel_manifest(paths)
    vectors: list[str] = []
    spice.kclear()
    try:
        for path in paths:
            spice.furnsh(str(path))
        toolkit_version = str(spice.tkvrsn("TOOLKIT"))
        for target, center, et in cases:
            if target == center or not math.isfinite(et):
                raise ValueError("invalid target/center or ET in reference request")
            state, _light_time = spice.spkezr(
                str(target), et, "J2000", "NONE", str(center)
            )
            components = [float(value) for value in state]
            if len(components) != 6 or not all(map(math.isfinite, components)):
                raise ValueError("NAIF state must contain six finite km/km-s values")
            vectors.append(
                ",".join(
                    [format(et, ".17g"), str(target), str(center)]
                    + [format(value, ".17g") for value in components]
                )
            )
    finally:
        spice.kclear()
    if kernel_manifest(paths) != before:
        raise ValueError("kernel name/size/SHA256 changed during NAIF generation")
    headers = [
        _metadata_text("provider", "NAIF_SPICE"),
        _metadata_text("method", "spkezr"),
        _metadata_text("time_scale", "TDB"),
        _metadata_text("frame", "J2000"),
        _metadata_text("corrections", "NONE"),
        _metadata_text("units", "KM-S"),
        _metadata_text("toolkit_version", toolkit_version),
        _metadata_text("generated_utc", generated_utc),
        _metadata_text("reference_source", SCRIPT_NAME),
        _metadata_text("generator_sha256", generator_digest.lower()),
        _metadata_text("ordered_kernel_sha256", ";".join(item[2] for item in before)),
        _metadata_text("ordered_kernel_sizes_bytes", ";".join(str(item[1]) for item in before)),
        _metadata_text("ordered_kernel_names", ";".join(item[0] for item in before)),
    ]
    return "\n".join([*headers, COLUMNS, *vectors, ""])


def metadata_from_csv(content: str) -> dict[str, str]:
    metadata: dict[str, str] = {}
    seen_header = False
    for raw in content.splitlines():
        line = raw.strip()
        if not line:
            continue
        if line.startswith("#") and not seen_header:
            part = line[1:].strip().split("=", 1)
            if len(part) != 2 or not part[0] or not part[1]:
                raise ValueError("invalid reference metadata")
            if part[0] in metadata:
                raise ValueError("duplicate reference metadata")
            metadata[part[0]] = part[1]
        elif not seen_header:
            if line != COLUMNS:
                raise ValueError("unexpected reference columns")
            seen_header = True
        elif line.startswith("#"):
            raise ValueError("metadata after CSV rows")
    if not seen_header:
        raise ValueError("missing reference header")
    return metadata


def verify_reference_hashes(content: str, paths: Sequence[Path]) -> None:
    """Fail if current kernel name, size, digest or load order differs from CSV."""
    metadata = metadata_from_csv(content)
    for key in (
        "ordered_kernel_sha256",
        "ordered_kernel_sizes_bytes",
        "ordered_kernel_names",
        "generator_sha256",
    ):
        if key not in metadata:
            raise ValueError(f"missing {key}")
    digests = metadata["ordered_kernel_sha256"].split(";")
    sizes = metadata["ordered_kernel_sizes_bytes"].split(";")
    names = metadata["ordered_kernel_names"].split(";")
    if len(paths) != len(digests) or len(digests) != len(sizes) or len(sizes) != len(names):
        raise ValueError("kernel path/order/count mismatch")
    if not HEX_SHA256_PATTERN.fullmatch(metadata["generator_sha256"]):
        raise ValueError("invalid generator digest declaration")
    actual = kernel_manifest(paths)
    for i, ((name, size, digest), expected_name, expected_size, expected_digest) in enumerate(
        zip(actual, names, sizes, digests), 1
    ):
        if not HEX_SHA256_PATTERN.fullmatch(expected_digest):
            raise ValueError(f"kernel {i} has invalid expected SHA256")
        if name != expected_name:
            raise ValueError(f"kernel {i} filename/order mismatch")
        if str(size) != expected_size:
            raise ValueError(f"kernel {i} byte size mismatch")
        if not hmac.compare_digest(digest, expected_digest.lower()):
            raise ValueError(f"kernel {i} SHA256 mismatch")
    # The CSV's declared digest is not an authenticity signature.


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate NAIF geometric SPK references or verify kernel SHA256."
    )
    subparsers = parser.add_subparsers(dest="action", required=True)
    generate = subparsers.add_parser("generate")
    generate.add_argument("--kernel", action="append", required=True, type=Path)
    generate.add_argument("--case", action="append", required=True)
    generate.add_argument("--output", type=Path, required=True)
    generate.add_argument("--overwrite", action="store_true")
    verify = subparsers.add_parser("verify")
    verify.add_argument("--reference", type=Path, required=True)
    verify.add_argument("--kernel", action="append", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        if args.action == "verify":
            verify_reference_hashes(args.reference.read_text(encoding="utf-8"), args.kernel)
            print(f"Verified {len(args.kernel)} ordered kernel SHA256 hashes, names and sizes.")
            return 0
        cases = parse_cases(args.case)
        if args.output.exists() and not args.overwrite:
            raise ValueError("output exists; pass --overwrite only if intentional")
        try:
            import spiceypy as spice
        except ImportError as error:
            raise ValueError(
                "generate mode requires independently installed spiceypy/NAIF Toolkit"
            ) from error
        generator_digest = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        utc = datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")
        csv = generate_reference(
            args.kernel, cases, spice, generated_utc=utc, generator_digest=generator_digest
        )
        verify_reference_hashes(csv, args.kernel)
        args.output.write_text(csv, encoding="utf-8", newline="\n")
        print(f"Wrote {len(cases)} NAIF reference cases to {args.output}.")
        return 0
    except (OSError, ValueError) as error:
        print(f"SPK reference {args.action} failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
