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
# Modification (2026-11-18): offline DE441 mobile slice pack orchestration
# and SHA-256 manifest verification, without any astronomical calculations.
# Modification (2026-11-19): allow a separate, versioned catalog to reuse the
# atomic manifest writer without changing the mobile v1 output filename.

"""Offline DE441 mobile pack builder; astronomical slicing is done by Rust.

Modification 2026-11-18: original VirgoStars tool and manifest schema under
CPAL-1.0 (see repository LICENSE, Exhibit A/B and NOTICE). No JPL data included.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import sys
import tempfile
from datetime import datetime, timezone


ROOT = Path(__file__).resolve().parent.parent
MANIFEST_NAME = "de441-mobile-manifest-v1.json"
SCHEMA = "virgostars.de441.mobile-packs.v1"
PACKS = (
    ("core", 1600, 2100, True),
    *((f"history-{start:04d}-{start + 199:04d}", start, start + 199, False)
      for start in range(0, 1600, 200)),
    ("future-2101-2300", 2101, 2300, False),
)
PACK_BY_ID = {pack[0]: pack for pack in PACKS}
EXPORT_OUTPUT = re.compile(
    r"^requested_jd_tdb=\[([^,]+), ([^)]+)\) "
    r"stored_jd_tdb=\[([^,]+), ([^]]+)\] "
    r"records=(\d+) bytes=(\d+) output=(.+)$"
)
PARITY_OUTPUT = re.compile(
    r"^parity_ok records=(\d+) epochs=(\d+) state_pairs=(\d+) "
    r"jd_tdb=\[([^,]+), ([^]]+)\]$"
)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        while chunk := file.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def header_coverage(path: Path) -> tuple[float, float, float]:
    with path.open("rb") as file:
        header = file.read(2856)
    if len(header) != 2856 or not header.startswith(b"JPL Planetary Ephemeris DE441"):
        raise ValueError(f"not a classic JPL DE441 file: {path.name}")
    if struct.unpack_from("<i", header, 2840)[0] != 441:
        raise ValueError(f"invalid DE441 identifier: {path.name}")
    start, end, step = struct.unpack_from("<ddd", header, 2652)
    if not (float("-inf") < start < end < float("inf") and 0 < step < float("inf")):
        raise ValueError(f"invalid DE441 coverage: {path.name}")
    return start, end, step


def artifact_name(pack_id: str) -> str:
    return f"de441-{pack_id}.eph"


def verify_artifact(folder: Path, artifact: dict) -> None:
    name = artifact.get("file")
    if (
        not isinstance(name, str)
        or name in (".", "..")
        or Path(name).name != name
        or "/" in name
        or "\\" in name
    ):
        raise ValueError("artifact filename must be a simple basename")
    path = folder / name
    if path.is_symlink() or not path.is_file():
        raise ValueError(f"missing or linked artifact: {name}")
    if path.stat().st_size != artifact["bytes"]:
        raise ValueError(f"file size mismatch for {name}")
    if sha256_file(path) != artifact["sha256"]:
        raise ValueError(f"SHA-256 mismatch for {name}")
    stored_start, stored_end, _ = header_coverage(path)
    if (stored_start, stored_end) != (
        artifact["stored_start_jd_tdb"],
        artifact["stored_end_jd_tdb"],
    ):
        raise ValueError(f"coverage mismatch for {name}")
    if not (
        stored_start <= artifact["requested_start_jd_tdb"]
        < artifact["requested_end_exclusive_jd_tdb"] <= stored_end
    ):
        raise ValueError(f"requested dates not covered by {name}")
    records = artifact["coefficient_records"]
    if (
        not isinstance(records, int)
        or records < 1
        or artifact["bytes"] % (records + 2)
    ):
        raise ValueError(f"invalid coefficient record count for {name}")
    record_size = artifact["bytes"] // (records + 2)
    if record_size < 2856:
        raise ValueError(f"invalid coefficient record length for {name}")
    with path.open("rb") as file:
        file.seek(2 * record_size)
        first_record_start = struct.unpack("<d", file.read(8))[0]
        file.seek((records + 1) * record_size + 8)
        last_record_end = struct.unpack("<d", file.read(8))[0]
    if (first_record_start, last_record_end) != (stored_start, stored_end):
        raise ValueError(f"coefficient record coverage mismatch for {name}")


def create_manifest_exclusive(folder: Path, document: dict,
                              name: str = MANIFEST_NAME) -> Path:
    if (not isinstance(name, str) or Path(name).name != name
            or name in (".", "..") or "\\" in name or "/" in name):
        raise ValueError("manifest filename must be a simple basename")
    target = folder / name
    if target.exists() or target.is_symlink():
        raise FileExistsError(f"manifest already exists: {target}")
    payload = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode("utf-8")
    with tempfile.NamedTemporaryFile(
        mode="wb", dir=folder, prefix=".de441-manifest-", suffix=".partial", delete=False
    ) as temp:
        temporary = Path(temp.name)
        try:
            temp.write(payload)
            temp.flush()
            os.fsync(temp.fileno())
        except BaseException:
            temporary.unlink(missing_ok=True)
            raise
    try:
        # A hard link publishes atomically and fails if target already exists.
        os.link(temporary, target)
    finally:
        temporary.unlink(missing_ok=True)
    return target


def parse_export_summary(stdout: str, expected_path: Path) -> dict:
    lines = stdout.strip().splitlines()
    if not lines or not (match := EXPORT_OUTPUT.fullmatch(lines[-1])):
        raise ValueError("Rust exporter returned no recognized summary")
    requested_start, requested_end, stored_start, stored_end = map(
        float, match.group(1, 2, 3, 4)
    )
    records, byte_count = int(match.group(5)), int(match.group(6))
    if Path(match.group(7)).resolve() != expected_path.resolve():
        raise ValueError("Rust exporter reported a different output path")
    if not (records > 0 and byte_count > 0):
        raise ValueError("Rust exporter reported empty records")
    return {
        "requested_start_jd_tdb": requested_start,
        "requested_end_exclusive_jd_tdb": requested_end,
        "stored_start_jd_tdb": stored_start,
        "stored_end_jd_tdb": stored_end,
        "coefficient_records": records,
        "bytes": byte_count,
    }


def verify_source_parity(source: Path, artifact_path: Path, summary: dict,
                         cargo: str) -> dict:
    """Call the canonical Rust evaluator across every record and main body."""
    command = [
        cargo, "run", "--offline", "--locked", "-p",
        "virgostars-ephemeris-core", "--example", "verify_de441_classic_slice",
        "--", str(source), str(artifact_path),
    ]
    result = subprocess.run(
        command, cwd=ROOT, capture_output=True, text=True, check=False
    )
    if result.returncode:
        raise RuntimeError(
            f"source/slice parity failed for {artifact_path.name}: {result.stderr.strip()}"
        )
    lines = result.stdout.strip().splitlines()
    if not lines or not (match := PARITY_OUTPUT.fullmatch(lines[-1])):
        raise ValueError(f"Rust parity tool returned no recognized summary for {artifact_path.name}")
    records, epochs, state_pairs = map(int, match.group(1, 2, 3))
    start, end = float(match.group(4)), float(match.group(5))
    if (
        records != summary["coefficient_records"]
        or epochs < records * 3
        or state_pairs != epochs * 14
        or (start, end) != (
            summary["stored_start_jd_tdb"],
            summary["stored_end_jd_tdb"],
        )
    ):
        raise ValueError(f"Rust parity summary disagrees with slice for {artifact_path.name}")
    return {
        "status": "pass_against_same_source",
        "compared_epochs": epochs,
        "compared_state_pairs": state_pairs,
        "independent_jpl_validation": False,
    }


def build(source: Path, destination: Path, selected: set[str],
          expected_source_sha256: str | None, cargo: str) -> Path:
    if not source.is_file():
        raise FileNotFoundError(f"missing source: {source}")
    source = source.resolve()
    source_start, source_end, source_step = header_coverage(source)
    if expected_source_sha256 and not re.fullmatch(r"[0-9a-fA-F]{64}", expected_source_sha256):
        raise ValueError("expected source SHA-256 must contain exactly 64 hex characters")
    source_hash = sha256_file(source)
    if expected_source_sha256 and source_hash != expected_source_sha256.lower():
        raise ValueError("source SHA-256 is not the expected fingerprint")

    destination.mkdir(parents=True, exist_ok=True)
    if (destination / MANIFEST_NAME).exists():
        raise FileExistsError(f"manifest already exists: {destination / MANIFEST_NAME}")
    for pack_id in selected:
        file = destination / artifact_name(pack_id)
        if file.exists() or file.is_symlink() or file.resolve() == source:
            raise FileExistsError(f"refusing to replace file: {file}")

    packages = []
    for pack_id, first_year, last_year, mandatory in PACKS:
        entry = {
            "id": pack_id,
            "first_year": first_year,
            "last_year_inclusive": last_year,
            "required": mandatory,
            "status": "not_built",
        }
        if pack_id in selected:
            path = destination / artifact_name(pack_id)
            command = [
                cargo, "run", "--offline", "--locked", "-p",
                "virgostars-ephemeris-core", "--example", "slice_de441_classic",
                "--", str(source), str(path), str(first_year), str(last_year),
            ]
            result = subprocess.run(
                command, cwd=ROOT, capture_output=True, text=True, check=False
            )
            if result.returncode:
                raise RuntimeError(f"export failed for {pack_id}: {result.stderr.strip()}")
            summary = parse_export_summary(result.stdout, path)
            entry.update(
                status="built",
                artifact={
                    "file": path.name,
                    "sha256": sha256_file(path),
                    **summary,
                    "format": "jpl-de441-classic-le",
                },
            )
            verify_artifact(destination, entry["artifact"])
            entry["artifact"]["parity"] = verify_source_parity(
                source, path, summary, cargo
            )
            print(
                f"{pack_id}: {summary['bytes']} bytes; SHA-256 and source/slice parity verified",
                flush=True,
            )
        packages.append(entry)

    # Guard against a source edit during extraction, including same-size edits.
    if sha256_file(source) != source_hash:
        raise ValueError("source changed during export; no manifest will be published")
    document = {
        "schema": SCHEMA,
        "created_at_utc": datetime.now(timezone.utc).isoformat(),
        "astronomical_year_numbering": "year 0 = 1 BCE; proleptic Gregorian",
        "requested_boundary_scale": "Julian Date TDB; Jan 1 00:00 TDB",
        "source": {
            "format": "jpl-de441-classic-le",
            "sha256": source_hash,
            "bytes": source.stat().st_size,
            "coverage_start_jd_tdb": source_start,
            "coverage_end_jd_tdb": source_end,
            "record_step_days": source_step,
        },
        "distribution_rights": "review_pending",
        "scientific_validation": "real_kernel_and_independent_reference_pending",
        "packages": packages,
    }
    return create_manifest_exclusive(destination, document)


def verify(manifest: Path) -> int:
    data = json.loads(manifest.read_text(encoding="utf-8"))
    if data.get("schema") != SCHEMA or not isinstance(data.get("packages"), list):
        raise ValueError("unsupported mobile pack manifest schema")
    if len(data["packages"]) != len(PACKS):
        raise ValueError("mobile manifest pack inventory is incomplete")
    built = 0
    seen = set()
    for entry in data["packages"]:
        pack_id = entry["id"]
        if not isinstance(pack_id, str) or pack_id not in PACK_BY_ID or pack_id in seen:
            raise ValueError(f"duplicate or unknown pack id: {pack_id}")
        seen.add(pack_id)
        _, first, last, required = PACK_BY_ID[pack_id]
        if (entry["first_year"], entry["last_year_inclusive"], entry["required"]) != (
            first, last, required
        ):
            raise ValueError(f"incorrect temporal inventory for {pack_id}")
        if entry["status"] == "built":
            artifact = entry["artifact"]
            verify_artifact(manifest.parent, artifact)
            parity = artifact.get("parity")
            if (
                not isinstance(parity, dict)
                or parity.get("status") != "pass_against_same_source"
                or parity.get("independent_jpl_validation") is not False
                or not isinstance(parity.get("compared_epochs"), int)
                or parity["compared_epochs"] < artifact["coefficient_records"] * 3
                or parity.get("compared_state_pairs") != parity["compared_epochs"] * 14
            ):
                raise ValueError(f"missing/invalid source parity result for {pack_id}")
            built += 1
        elif entry["status"] != "not_built":
            raise ValueError(f"invalid status: {entry['status']}")
    print(f"verified {built} artifacts against {manifest.name}")
    return built


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("plan", help="print the fixed DE441 mobile pack ranges")
    builder = commands.add_parser("build", help="run offline Rust exports and hash the files")
    builder.add_argument("--source", type=Path, required=True)
    builder.add_argument("--output-dir", type=Path, required=True)
    builder.add_argument("--packs", nargs="+", choices=[*PACK_BY_ID, "all"], default=["core"])
    builder.add_argument("--expected-source-sha256")
    builder.add_argument("--cargo", default="cargo")
    verifier = commands.add_parser("verify", help="recheck produced files against manifest")
    verifier.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        if args.command == "plan":
            for pack_id, first, last, required in PACKS:
                print(f"{pack_id:22} {first:4}–{last:4} {'base' if required else 'optional'}")
        elif args.command == "build":
            if "all" in args.packs and len(args.packs) > 1:
                raise ValueError("--packs all cannot be combined with individual IDs")
            selected = set(PACK_BY_ID if "all" in args.packs else args.packs)
            print(build(args.source, args.output_dir, selected,
                        args.expected_source_sha256, args.cargo))
        else:
            verify(args.manifest)
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"DE441 mobile pack tool: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
