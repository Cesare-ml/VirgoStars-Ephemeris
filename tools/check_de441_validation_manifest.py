#!/usr/bin/env python3
"""Fail-closed metadata preflight for independent DE441 validation manifests.

This checks provenance and comparability, NOT numerical accuracy.
"""
import argparse
import json
import math
from pathlib import Path

REQUIRED = ("kernel_url", "kernel_sha256", "kernel_bytes", "retrieved_utc",
            "engine_commit", "reference_source", "reference_solution",
            "reference_query", "tolerance_position_km", "tolerance_velocity_km_day",
            "samples")
SAMPLE_FIELDS = ("jd_tdb", "target_id", "center_id", "frame", "correction",
                 "reference_plane", "position_unit", "velocity_unit")


def validate_manifest(data):
    if not isinstance(data, dict):
        raise ValueError("manifest must be an object")
    missing = [field for field in REQUIRED if field not in data]
    if missing:
        raise ValueError("missing fields: " + ", ".join(missing))
    for field in ("kernel_url", "retrieved_utc", "engine_commit", "reference_source", "reference_query"):
        if not isinstance(data[field], str) or not data[field].strip():
            raise ValueError(f"{field} must be nonempty")
    digest = data["kernel_sha256"]
    if not isinstance(digest, str) or len(digest) != 64 or any(c not in "0123456789abcdefABCDEF" for c in digest):
        raise ValueError("kernel_sha256 must be a 64-digit hex digest")
    if type(data["kernel_bytes"]) is not int or data["kernel_bytes"] <= 0:
        raise ValueError("kernel_bytes must be a positive integer")
    if data["reference_solution"] != "DE441":
        raise ValueError("reference solution must explicitly be DE441")
    for field in ("tolerance_position_km", "tolerance_velocity_km_day"):
        value = data[field]
        if type(value) not in (float, int) or not math.isfinite(value) or value <= 0:
            raise ValueError(f"{field} must be finite and positive")
    samples = data["samples"]
    if not isinstance(samples, list) or not samples:
        raise ValueError("samples must be a nonempty array")
    for index, sample in enumerate(samples):
        if not isinstance(sample, dict) or any(key not in sample for key in SAMPLE_FIELDS):
            raise ValueError(f"sample {index}: missing required fields")
        if type(sample["jd_tdb"]) not in (int, float) or not math.isfinite(sample["jd_tdb"]):
            raise ValueError(f"sample {index}: invalid TDB Julian date")
        for key in ("target_id", "center_id"):
            if type(sample[key]) is not int:
                raise ValueError(f"sample {index}: {key} must be an integer")
        expected = {"frame": "ICRF", "correction": "NONE", "reference_plane": "FRAME",
                    "position_unit": "km", "velocity_unit": "km/day"}
        for key, value in expected.items():
            if sample[key] != value:
                raise ValueError(f"sample {index}: {key} must be {value}")
        if sample["target_id"] == sample["center_id"]:
            raise ValueError(f"sample {index}: target and center must differ")
    return len(samples)


def reject_duplicate_keys(pairs):
    """Reject ambiguous JSON objects rather than silently accepting the last key."""
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def reject_nonfinite_json_constant(value):
    """Strict JSON forbids NaN/Infinity, including in otherwise ignored fields."""
    raise ValueError(f"non-standard JSON numeric constant: {value}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    try:
        count = validate_manifest(json.loads(args.manifest.read_text(encoding="utf-8"), object_pairs_hook=reject_duplicate_keys,
                                             parse_constant=reject_nonfinite_json_constant))
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        parser.exit(1, f"Invalid DE441 manifest: {exc}\n")
    print(f"Metadata preflight passed for {count} samples; numerical validation NOT performed.")


if __name__ == "__main__":
    main()
