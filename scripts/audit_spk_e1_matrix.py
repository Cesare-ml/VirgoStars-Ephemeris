# EXHIBIT A. Common Public Attribution License Version 1.0.
# The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
# Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
# The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
# The Original Developer is the Initial Developer.
# The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
# Contributors retain applicable rights in their own contributions under the terms of this License.
# [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
# SPDX-License-Identifier: CPAL-1.0

"""Audit minimum E1 reference *case breadth*, without evaluating astronomy.

This check is intentionally independent of any unmerged Rust or SPICE helpers.
A PASS proves only that a correctly shaped candidate CSV contains a sufficiently
broad test matrix. It NEVER proves data provenance or numerical agreement.
"""

from __future__ import annotations

import argparse
from collections import defaultdict
from itertools import combinations
import hashlib
import json
import math
from pathlib import Path
import re
import sys

COLUMNS = "et_tdb_seconds,target_id,center_id,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s"
CORE_PAIRS = {(10, 399): "Sun relative to Earth",
              (301, 399): "Moon relative to Earth",
              (399, 0): "Earth relative to SSB"}
# NAIF planet barycenter IDs and individual planet-center IDs where supported.
# Earth(399), Earth-Moon barycenter(3), Sun(10) and Moon(301) are NOT candidates.
PLANET_IDS = frozenset({1, 2, 4, 5, 6, 7, 8, 9,
                        199, 299, 499, 599, 699, 799, 899, 999})
REQUIRED_CONTRACT = {"provider": "NAIF_SPICE", "method": "spkezr",
                     "time_scale": "TDB", "frame": "J2000",
                     "corrections": "NONE", "units": "KM-S"}
HEX64 = re.compile(r"[0-9a-fA-F]{64}\Z")
Rows = list[tuple[int, int, float]]


def planet_system(target: int) -> int | None:
    """Canonical NAIF planet family, unifying center and system barycenter IDs.

    4 (Mars system barycenter) and 499 (Mars body center) are *one*
    planetary system for test-breadth accounting, not two planets.
    """
    if target not in PLANET_IDS:
        return None
    return target if target < 10 else target // 100


def plan_e1_cases(planet_ids: list[int], epochs: list[float]) -> Rows:
    """Plan explicit E1 target/center/ET triples without touching SPK kernels.

    IDs and epochs must be supplied by the operator. There is no inferred
    coverage, default epoch, civil-to-TDB conversion or network access.
    """
    if len(planet_ids) != 2:
        raise ValueError("exactly two --planet NAIF IDs are required")
    if any(target not in PLANET_IDS for target in planet_ids):
        raise ValueError("unsupported planetary ID: use a listed center or barycenter")
    if planet_system(planet_ids[0]) == planet_system(planet_ids[1]):
        raise ValueError("two physically distinct planetary systems are required")
    if len(epochs) < 3:
        raise ValueError("at least three explicit --epoch ET values are required")
    if not all(math.isfinite(et) for et in epochs):
        raise ValueError("all ET epochs must be finite TDB seconds past J2000")
    if len(set(epochs)) != len(epochs):
        raise ValueError("duplicate ET epochs are not allowed")
    pairs = [*CORE_PAIRS, (planet_ids[0], 399), (planet_ids[1], 399)]
    cases = [(target, center, et) for target, center in pairs for et in epochs]
    # The planned inputs must satisfy the same coverage rules as existing CSVs.
    issues, _, _ = audit_e1_matrix(cases)
    if issues:
        raise ValueError("planned cases fail E1 audit: " + "; ".join(issues))
    return cases


def parse_reference(content: str) -> Rows:
    """Validate numeric rows and metadata compatible with the Rust comparator."""
    metadata: dict[str, str] = {}
    rows: Rows = []
    seen: set[tuple[int, int, float]] = set()
    header = False
    for line_number, original in enumerate(content.splitlines(), 1):
        line = original.strip()
        if not line:
            continue
        if line.startswith("#"):
            if header:
                raise ValueError(f"line {line_number}: metadata after CSV header")
            if "=" not in line:
                raise ValueError(f"line {line_number}: invalid metadata")
            key, value = (item.strip() for item in line[1:].split("=", 1))
            if not key or not value or key in metadata:
                raise ValueError(f"line {line_number}: missing or duplicated metadata")
            metadata[key] = value
            continue
        if not header:
            if line != COLUMNS:
                raise ValueError(f"line {line_number}: wrong reference CSV columns")
            header = True
            continue
        fields = [part.strip() for part in line.split(",")]
        if len(fields) != 9:
            raise ValueError(f"line {line_number}: expected nine fields")
        try:
            et = float(fields[0])
            target, center = int(fields[1]), int(fields[2])
            vector = [float(number) for number in fields[3:]]
        except ValueError as exc:
            raise ValueError(f"line {line_number}: nonnumeric ET, ID or vector") from exc
        if not math.isfinite(et) or not all(math.isfinite(x) for x in vector):
            raise ValueError(f"line {line_number}: nonfinite ET or vector")
        if any(not re.fullmatch(r"[+-]?[0-9]+", v) for v in fields[1:3]):
            raise ValueError(f"line {line_number}: NAIF IDs must be decimal integers")
        if not all(-(2 ** 31) <= value < 2 ** 31 for value in (target, center)):
            raise ValueError(f"line {line_number}: NAIF IDs exceed i32")
        if target == center:
            raise ValueError(f"line {line_number}: identical target and center")
        key = (target, center, et)
        if key in seen:
            raise ValueError(f"line {line_number}: duplicate case")
        seen.add(key)
        rows.append(key)
    if not header or not rows:
        raise ValueError("header and at least one nontrivial reference row required")
    for name, expected in REQUIRED_CONTRACT.items():
        if metadata.get(name) != expected:
            raise ValueError(f"{name} metadata must equal {expected}")
    for name in ("toolkit_version", "generated_utc", "reference_source"):
        if not metadata.get(name):
            raise ValueError(f"missing provenance metadata {name}")
    hashes = metadata.get("ordered_kernel_sha256", "").split(";")
    if not hashes or not all(HEX64.fullmatch(hash_value.strip()) for hash_value in hashes):
        raise ValueError("ordered_kernel_sha256 must contain valid 64-hex digests")
    return rows


def audit_e1_matrix(rows: Rows) -> tuple[list[str], list[tuple[int, int]], list[float]]:
    """Require five pairs from five physical systems across >=3 shared TDB epochs."""
    epochs: dict[tuple[int, int], set[float]] = defaultdict(set)
    for target, center, et in rows:
        epochs[(target, center)].add(et)
    issues = []
    for pair, label in CORE_PAIRS.items():
        count = len(epochs.get(pair, set()))
        if count < 3:
            issues.append(f"{label} ({pair[0]}:{pair[1]}): {count}/3 distinct ET epochs")
    candidates = [pair for pair, times in epochs.items()
                  if pair[0] in PLANET_IDS and pair[1] in (0, 399)
                  and len(times) >= 3]
    best: tuple[list[tuple[int, int]], list[float]] | None = None
    for left, right in combinations(sorted(candidates), 2):
        if planet_system(left[0]) == planet_system(right[0]):
            # NAIF 4 and 499, for example, are the same Mars system even
            # though their SPK trajectories and physical centers differ.
            continue
        required = [*CORE_PAIRS, left, right]
        shared = sorted(set.intersection(*(epochs.get(pair, set()) for pair in required)))
        if len(shared) >= 3:
            best = ([left, right], shared)
            break
    planet_count = len({planet_system(pair[0]) for pair in candidates})
    if planet_count < 2:
        issues.append(f"two distinct planetary systems required; found {planet_count}")
    elif best is None:
        issues.append("no two distinct planetary systems share three ET epochs with all core pairs")
    if best is None:
        return issues, [], []
    return issues, best[0], best[1]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path,
                        help="Audit an existing NAIF/SPICE reference CSV")
    parser.add_argument("--json", action="store_true",
                        help="Print structured audit result with exact CSV byte SHA-256")
    parser.add_argument("--plan", action="store_true",
                        help="Print --case triples for the independent NAIF generator")
    parser.add_argument("--planet", type=int, action="append",
                        help="Two explicit, physically distinct planet NAIF IDs")
    parser.add_argument("--epoch", type=float, action="append",
                        help="Three or more explicit TDB/ET seconds past J2000")
    args = parser.parse_args(argv)
    try:
        if args.plan:
            if args.reference is not None or args.json:
                raise ValueError("--plan cannot be combined with --reference or --json")
            cases = plan_e1_cases(args.planet or [], args.epoch or [])
            for target, center, et in cases:
                print(f"--case {target}:{center}:{format(et, '.17g')}")
            print("NOT CERTIFIED: review actual kernel coverage, time scale, "
                  "source and numeric tolerances before use.", file=sys.stderr)
            return 0
        if args.reference is None or args.planet or args.epoch:
            raise ValueError("supply --reference alone, or use --plan with --planet/--epoch")
        # Hash the exact byte stream, not re-encoded text or normalized newlines.
        reference_bytes = args.reference.read_bytes()
        rows = parse_reference(reference_bytes.decode("utf-8"))
        issues, planets, shared = audit_e1_matrix(rows)
    except (OSError, UnicodeError, ValueError) as exc:
        if args.json:
            print(json.dumps({
                "schema": "virgostars-e1-matrix-audit/v1",
                "status": "invalid",
                "issues": [str(exc)],
                "scientific_accuracy_certified": False,
                "provenance_review_required": True,
            }, sort_keys=True))
        else:
            print(f"E1 matrix audit ERROR: {exc}", file=sys.stderr)
        return 2
    if args.json:
        print(json.dumps({
            "schema": "virgostars-e1-matrix-audit/v1",
            "status": "incomplete" if issues else "pass",
            "reference_sha256": hashlib.sha256(reference_bytes).hexdigest(),
            "case_count": len(rows),
            "planet_pairs": [list(pair) for pair in planets],
            "shared_et_tdb_seconds": [format(epoch, ".17g") for epoch in shared],
            "issues": issues,
            "scientific_accuracy_certified": False,
            "provenance_review_required": True,
        }, sort_keys=True))
        return 1 if issues else 0
    if issues:
        for issue in issues:
            print(f"INCOMPLETE E1 case matrix: {issue}", file=sys.stderr)
        print("Case breadth is insufficient; this is NOT a numerical validation.", file=sys.stderr)
        return 1
    print(f"E1 candidate matrix breadth: PASS ({len(rows)} rows, "
          f"{len(shared)} shared TDB epochs, planet pairs {planets}).")
    print("NOT CERTIFIED: kernel provenance, NAIF vectors, numerical residuals, "
          "coverage boundaries and error tolerances require independent review.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
