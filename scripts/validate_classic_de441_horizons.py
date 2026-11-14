# EXHIBIT A. Common Public Attribution License Version 1.0.
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

"""Offline, fail-closed SHA-256 preflight for a classic DE441/Horizons comparison.

This script never downloads or redistributes astronomical data. A matching
digest does NOT prove that a kernel or reference originated from JPL.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import subprocess
import sys

MIN_CLASSIC_BYTES = 1_000_000_000
CLASSIC_PREFIX = b"JPL Planetary Ephemeris DE441"
VALID_HASH = re.compile(r"[a-fA-F0-9]{64}\Z")


def digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(4 * 1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def expected_digest(value: str, label: str) -> str:
    if VALID_HASH.fullmatch(value) is None:
        raise ValueError(f"{label} must be exactly 64 hexadecimal SHA-256 digits")
    return value.lower()


def positive_tolerance(value: str, label: str) -> float:
    try:
        parsed = float(value)
    except ValueError as exc:
        raise ValueError(f"{label} must be finite and strictly positive") from exc
    if not math.isfinite(parsed) or parsed <= 0:
        raise ValueError(f"{label} must be finite and strictly positive")
    return parsed


def preflight(
    kernel: Path,
    reference: Path,
    kernel_sha256: str,
    reference_sha256: str,
    position_tolerance: str,
    velocity_tolerance: str,
) -> dict:
    """Verify both exact source files BEFORE launching the Rust comparator."""
    expected_kernel = expected_digest(kernel_sha256, "kernel_sha256")
    expected_reference = expected_digest(reference_sha256, "reference_sha256")
    position = positive_tolerance(position_tolerance, "position tolerance")
    velocity = positive_tolerance(velocity_tolerance, "velocity tolerance")
    for path, label in ((kernel, "kernel"), (reference, "reference")):
        if not path.is_file():
            raise ValueError(f"{label} is not a readable regular file: {path}")
    if kernel.resolve() == reference.resolve():
        raise ValueError("kernel and reference must be different files")
    size = kernel.stat().st_size
    if size < MIN_CLASSIC_BYTES:
        raise ValueError("kernel is too small for a full classic DE441 binary")
    with kernel.open("rb") as stream:
        header = stream.read(2844)
    if not header.startswith(CLASSIC_PREFIX) or len(header) < 2844:
        raise ValueError("kernel is not a classic little-endian DE441 binary")
    if int.from_bytes(header[2840:2844], "little", signed=True) != 441:
        raise ValueError("kernel lacks the classic DE441 binary header version")
    actual_kernel = digest(kernel)
    if actual_kernel != expected_kernel:
        raise ValueError("kernel SHA-256 differs from the independently supplied expected digest")
    actual_reference = digest(reference)
    if actual_reference != expected_reference:
        raise ValueError("reference SHA-256 differs from the independently supplied expected digest")
    # This file check is deliberately only a preflight: Rust validates all
    # reference metadata, numeric rows and actual ephemeris state semantics.
    return {
        "kernel_name": kernel.name,
        "kernel_bytes": size,
        "kernel_sha256": actual_kernel,
        "reference_name": reference.name,
        "reference_sha256": actual_reference,
        "position_component_tolerance_km": position,
        "velocity_component_tolerance_km_per_day": velocity,
    }


def compare(
    kernel: Path,
    reference: Path,
    kernel_sha256: str,
    reference_sha256: str,
    position_tolerance: str,
    velocity_tolerance: str,
    *,
    runner=subprocess.run,
) -> dict:
    """Run only after both pinned file digests and source header match."""
    inputs = preflight(
        kernel, reference, kernel_sha256, reference_sha256,
        position_tolerance, velocity_tolerance,
    )
    command = [
        "cargo", "run", "--locked", "--quiet",
        "-p", "virgostars-ephemeris-core",
        "--example", "validate_horizons", "--",
        str(kernel.resolve()), str(reference.resolve()),
        str(position_tolerance), str(velocity_tolerance),
    ]
    completed = runner(
        command,
        cwd=Path(__file__).resolve().parents[1],
        capture_output=True,
        text=True,
        check=False,
    )
    return {
        "schema_version": 1,
        "status": (
            "COMPARISON_PASSED_PENDING_INDEPENDENT_PROVENANCE_REVIEW"
            if completed.returncode == 0 else "COMPARISON_FAILED"
        ),
        "scientific_validated": False,
        "scope": "classic_DE441_geometric_ICRF_TDB_JD_km_km_per_day",
        "input": inputs,
        "cargo_command": command,
        "cargo_exit_code": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
        "limitations": (
            "Hashes confirm input byte identity, not NASA/JPL authenticity; "
            "source-revision, Horizons ephemeris provenance, matching query "
            "settings, reference independence, and approved tolerance review "
            "must be audited separately."
        ),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--kernel", type=Path, required=True)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--kernel-sha256", required=True)
    parser.add_argument("--reference-sha256", required=True)
    parser.add_argument("--position-km", required=True)
    parser.add_argument("--velocity-km-per-day", required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args(argv)
    # Never write an error receipt on top of an input file, even when the
    # preflight fails. Abort before opening any path for output.
    if args.receipt.resolve() in (args.kernel.resolve(), args.reference.resolve()):
        print("PREFLIGHT_FAILED: receipt must not overwrite an input file", file=sys.stderr)
        return 1
    try:
        report = compare(
            args.kernel, args.reference, args.kernel_sha256,
            args.reference_sha256, args.position_km,
            args.velocity_km_per_day,
        )
        exit_code = 0 if report["cargo_exit_code"] == 0 else 1
    except (ValueError, OSError) as exc:
        report = {
            "schema_version": 1,
            "status": "PREFLIGHT_FAILED",
            "scientific_validated": False,
            "error": str(exc),
        }
        exit_code = 1
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    args.receipt.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"{report['status']}: receipt at {args.receipt}", file=sys.stderr)
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
