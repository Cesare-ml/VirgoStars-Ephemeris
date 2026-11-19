#!/usr/bin/env python3
# EXHIBIT A. Common Public Attribution License Version 1.0.
# The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
# Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
# The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
# The Original Developer is the Initial Developer.
# The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
# Contributors retain applicable rights in their own contributions under the terms of this License.
# [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
# SPDX-License-Identifier: CPAL-1.0
# Modification (2026-11-19): reusable offline contiguous DE441 catalog and
# source-verified slice packaging for a proposed Desktop default profile.

"""Offline, release-ready-in-shape DE441 slices; never uploads or downloads.

The desktop base is one self-contained classic slice covering astronomical
years 1000 through 2200 inclusive. Optional 200-year slices extend outward
without gaps. Year zero denotes 1 BCE; requested years are interpreted by the
canonical Rust exporter as proleptic Gregorian boundaries in JD TDB.

This tool reuses the existing Rust source/slice parity checker. Its unsigned
manifest remains LOCAL and explicitly NOT cleared for redistribution. The
actual source's JD coverage must be confirmed when running a real export.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import re
import subprocess
import sys

import build_de441_mobile_packs as common

SCHEMA = "virgostars.de441.packs.v2"
MANIFEST_NAME = "de441-contiguous-manifest-v2.json"
BASE_FIRST = 1000
BASE_LAST = 2200
BLOCK_YEARS = 200
# Conservative planning defaults for the classic binary described in docs/DE441.md.
# They are NOT asserted to be the source's physical coverage; Rust decides.
DEFAULT_MIN_YEAR = -12999
DEFAULT_MAX_YEAR = 16999
MAX_RELEASE_ASSET_BYTES = 2 * 1024 * 1024 * 1024


def year_label(year: int) -> str:
    return f"m{-year:05d}" if year < 0 else f"{year:05d}"


def build_plan(min_year: int, max_year: int) -> list[tuple[str, int, int, bool]]:
    if (not isinstance(min_year, int) or not isinstance(max_year, int)
            or min_year < -20000 or max_year > 20000
            or min_year > BASE_FIRST or max_year < BASE_LAST):
        raise ValueError("requested catalog must contain 1000–2200 and remain within ±20000")
    earlier = []
    cursor = BASE_FIRST
    while cursor > min_year:
        first = max(min_year, cursor - BLOCK_YEARS)
        last = cursor - 1
        earlier.append((f"history-{year_label(first)}-{year_label(last)}",
                        first, last, False))
        cursor = first

    later = []
    cursor = BASE_LAST + 1
    while cursor <= max_year:
        last = min(max_year, cursor + BLOCK_YEARS - 1)
        later.append((f"future-{year_label(cursor)}-{year_label(last)}",
                      cursor, last, False))
        cursor = last + 1
    plan = [
        *reversed(earlier),
        ("desktop-core-1000-2200", BASE_FIRST, BASE_LAST, True),
        *later,
    ]
    if len(plan) > 256:
        raise ValueError("catalog exceeds the current Rust pack-manager limit of 256")
    assert all(a[2] + 1 == b[1] for a, b in zip(plan, plan[1:]))
    return plan


def filename(pack_id: str) -> str:
    return f"de441-v2-{pack_id}.eph"


def build(source: Path, destination: Path, plan: list[tuple[str, int, int, bool]],
          selected: set[str], expected_source_sha256: str | None,
          cargo: str = "cargo") -> Path:
    if not source.is_file() or source.is_symlink():
        raise ValueError("source is not a regular classic DE441 file")
    source = source.resolve()
    coverage_start, coverage_end, step = common.header_coverage(source)
    if (expected_source_sha256 is not None
            and not re.fullmatch(r"[0-9a-fA-F]{64}", expected_source_sha256)):
        raise ValueError("expected source SHA-256 must be 64 hexadecimal characters")
    source_hash = common.sha256_file(source)
    if expected_source_sha256 and source_hash != expected_source_sha256.lower():
        raise ValueError("source fingerprint does not match")

    ids = {p[0] for p in plan}
    if not selected or not selected.issubset(ids):
        raise ValueError("selected package IDs are unknown or empty")
    destination.mkdir(parents=True, exist_ok=True)
    if destination.is_symlink():
        raise ValueError("destination must not be a symlink")
    target = destination / MANIFEST_NAME
    if target.exists() or target.is_symlink():
        raise FileExistsError("refusing to replace existing manifest")
    for pack_id in selected:
        path = destination / filename(pack_id)
        if path.exists() or path.is_symlink() or path.resolve() == source:
            raise FileExistsError(f"refusing to replace {path.name}")

    packages = []
    for pack_id, first, last, required in plan:
        entry = dict(id=pack_id, first_year=first, last_year_inclusive=last,
                     required=required, status="not_built")
        if pack_id in selected:
            artifact_path = destination / filename(pack_id)
            command = [
                cargo, "run", "--offline", "--locked", "-p",
                "virgostars-ephemeris-core", "--example", "slice_de441_classic",
                "--", str(source), str(artifact_path), str(first), str(last),
            ]
            process = subprocess.run(
                command, cwd=common.ROOT, text=True, capture_output=True,
                check=False,
            )
            if process.returncode:
                raise RuntimeError(
                    f"Rust export failed for {pack_id}: {process.stderr.strip()}"
                )
            summary = common.parse_export_summary(process.stdout, artifact_path)
            if summary["bytes"] >= MAX_RELEASE_ASSET_BYTES:
                raise ValueError(f"{pack_id} exceeds the GitHub release-asset ceiling")
            artifact = {
                "file": artifact_path.name,
                "format": "jpl-de441-classic-le",
                "sha256": common.sha256_file(artifact_path),
                **summary,
            }
            common.verify_artifact(destination, artifact)
            artifact["parity"] = common.verify_source_parity(
                source, artifact_path, summary, cargo
            )
            entry.update(status="built", artifact=artifact)
            print(f"{pack_id}: {summary['bytes']} bytes; parity verified",
                  flush=True)
        packages.append(entry)

    if common.sha256_file(source) != source_hash:
        raise ValueError("source was modified during slicing")
    manifest = {
        "schema": SCHEMA,
        "created_at_utc": datetime.now(timezone.utc).isoformat(),
        "astronomical_year_numbering": "year 0 = 1 BCE; proleptic Gregorian",
        "requested_boundary_scale": "JD TDB; Jan 1 00:00 TDB; half-open",
        "desktop_base_years_inclusive": [BASE_FIRST, BASE_LAST],
        "optional_block_years": BLOCK_YEARS,
        "catalog_first_year": plan[0][1],
        "catalog_last_year_inclusive": plan[-1][2],
        "source": {
            "format": "jpl-de441-classic-le",
            "sha256": source_hash,
            "bytes": source.stat().st_size,
            "coverage_start_jd_tdb": coverage_start,
            "coverage_end_jd_tdb": coverage_end,
            "record_step_days": step,
        },
        "distribution_rights": "review_pending",
        "publication_status": "local_unsigned_not_authorized",
        "scientific_validation": "independent_jpl_reference_pending",
        "packages": packages,
    }
    return common.create_manifest_exclusive(destination, manifest, MANIFEST_NAME)


def verify(manifest_path: Path) -> int:
    data = json.loads(manifest_path.read_text(encoding="utf-8"))
    if data.get("schema") != SCHEMA or not isinstance(data.get("packages"), list):
        raise ValueError("unsupported contiguous pack manifest schema")
    if data.get("desktop_base_years_inclusive") != [BASE_FIRST, BASE_LAST]:
        raise ValueError("unexpected desktop base")
    if data.get("optional_block_years") != BLOCK_YEARS:
        raise ValueError("unexpected year block size")
    expected = build_plan(data["catalog_first_year"], data["catalog_last_year_inclusive"])
    if len(data["packages"]) != len(expected):
        raise ValueError("catalog inventory is incomplete")
    built = 0
    for entry, (pack_id, first, last, required) in zip(data["packages"], expected):
        if not isinstance(entry, dict) or (
            entry.get("id"), entry.get("first_year"),
            entry.get("last_year_inclusive"), entry.get("required")
        ) != (pack_id, first, last, required):
            raise ValueError(f"out-of-order, altered or missing package: {pack_id}")
        if entry.get("status") == "not_built":
            if "artifact" in entry:
                raise ValueError(f"unbuilt package has an artifact: {pack_id}")
            continue
        if entry.get("status") != "built":
            raise ValueError(f"invalid status for {pack_id}")
        artifact = entry.get("artifact")
        if not isinstance(artifact, dict) or artifact.get("file") != filename(pack_id):
            raise ValueError(f"invalid artifact name for {pack_id}")
        common.verify_artifact(manifest_path.parent, artifact)
        parity = artifact.get("parity")
        if (not isinstance(parity, dict)
                or parity.get("status") != "pass_against_same_source"
                or parity.get("independent_jpl_validation") is not False
                or not isinstance(parity.get("compared_epochs"), int)
                or parity["compared_epochs"] < artifact["coefficient_records"] * 3
                or parity.get("compared_state_pairs") != parity["compared_epochs"] * 14):
            raise ValueError(f"missing or invalid Rust parity proof for {pack_id}")
        built += 1
    print(f"verified {built}/{len(expected)} artifacts")
    return built


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--min-year", type=int, default=DEFAULT_MIN_YEAR)
    parser.add_argument("--max-year", type=int, default=DEFAULT_MAX_YEAR)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("plan", help="list proposed contiguous intervals (not source-validated)")
    builder = sub.add_parser("build", help="make checked slices from a local classic source")
    builder.add_argument("--source", required=True, type=Path)
    builder.add_argument("--output-dir", required=True, type=Path)
    builder.add_argument("--packs", nargs="+", default=["desktop-default"])
    builder.add_argument("--expected-source-sha256")
    builder.add_argument("--cargo", default="cargo")
    verifier = sub.add_parser("verify", help="check a locally built manifest and files")
    verifier.add_argument("--manifest", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command == "verify":
            verify(args.manifest)
        else:
            plan = build_plan(args.min_year, args.max_year)
            if args.command == "plan":
                for pack_id, first, last, required in plan:
                    print(f"{pack_id:38} {first:6} .. {last:6} "
                          f"{'DESKTOP BASE' if required else 'optional'}")
                print(f"{len(plan)} packages; proposed only (source not inspected)")
            else:
                if "all" in args.packs:
                    if len(args.packs) != 1:
                        raise ValueError("--packs all cannot be combined with other IDs")
                    selected = {item[0] for item in plan}
                elif "desktop-default" in args.packs:
                    if len(args.packs) != 1:
                        raise ValueError("desktop-default cannot be combined with IDs")
                    selected = {item[0] for item in plan if item[3]}
                else:
                    selected = set(args.packs)
                print(build(args.source, args.output_dir, plan, selected,
                            args.expected_source_sha256, args.cargo))
    except (OSError, ValueError, KeyError, TypeError, RuntimeError,
            subprocess.SubprocessError) as error:
        print(f"DE441 contiguous pack tool: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
