#!/usr/bin/env python3
"""Compare two independently supplied geometric state CSVs; no network or kernels."""
import argparse
import csv
import math
from pathlib import Path

FIELDS = ("x_km", "y_km", "z_km", "vx_km_day", "vy_km_day", "vz_km_day")
META = ("jd_tdb", "target", "center", "frame", "corrections")


def read_state(path):
    with Path(path).open(newline="", encoding="utf-8-sig") as handle:
        rows = list(csv.DictReader(handle))
    if len(rows) != 1:
        raise ValueError(f"{path}: expected exactly one data row")
    row = rows[0]
    if any(name not in row for name in FIELDS + META):
        raise ValueError(f"{path}: missing required columns")
    if any(not row[name] for name in FIELDS + META):
        raise ValueError(f"{path}: empty required value")
    values = [float(row[name]) for name in FIELDS]
    if not all(math.isfinite(value) for value in values):
        raise ValueError(f"{path}: non-finite state component")
    epoch = float(row["jd_tdb"])
    if not math.isfinite(epoch):
        raise ValueError(f"{path}: non-finite epoch")
    row["jd_tdb"] = epoch
    return row, values


def compare(ours_path, reference_path):
    ours, a = read_state(ours_path)
    reference, b = read_state(reference_path)
    for field in META:
        if ours[field] != reference[field]:
            raise ValueError(f"metadata mismatch for {field}: {ours[field]!r} != {reference[field]!r}")
    if ours["frame"] != "ICRF" or ours["corrections"] != "GEOMETRIC":
        raise ValueError("only ICRF geometric vectors are supported")
    residual = [x - y for x, y in zip(a, b)]
    if not all(math.isfinite(x) for x in residual):
        raise ValueError("non-finite residual (overflow)")
    return {
        "position_norm_km": math.hypot(*residual[:3]),
        "velocity_norm_km_per_day": math.hypot(*residual[3:]),
        "max_position_component_km": max(map(abs, residual[:3])),
        "max_velocity_component_km_per_day": max(map(abs, residual[3:])),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ours", help="Rust export CSV")
    parser.add_argument("reference", help="independently prepared CSV in the same schema and units")
    args = parser.parse_args()
    try:
        metrics = compare(args.ours, args.reference)
    except (ValueError, OSError) as exc:
        parser.error(str(exc))
    for name, value in metrics.items():
        print(f"{name}={value:.16e}")


if __name__ == "__main__":
    main()
