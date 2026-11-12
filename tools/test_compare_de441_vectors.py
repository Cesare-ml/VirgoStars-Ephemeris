#!/usr/bin/env python3
"""Deterministic standard-library tests for the offline DE441 CSV comparator.

No external kernel or Horizons download is required.
"""
import csv
import importlib.util
import math
import subprocess
import sys
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().with_name("compare_de441_vectors.py")
spec = importlib.util.spec_from_file_location("compare_de441_vectors", SCRIPT)
comparator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(comparator)

HEADER = (*comparator.META, *comparator.FIELDS)
BASE = {
    "jd_tdb": "2451545.0",
    "target": "moon",
    "center": "earth",
    "frame": "ICRF",
    "corrections": "GEOMETRIC",
    "x_km": "100",
    "y_km": "200",
    "z_km": "300",
    "vx_km_day": "1",
    "vy_km_day": "2",
    "vz_km_day": "3",
}


class VectorComparisonTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.ours = Path(self.temp.name) / "ours.csv"
        self.reference = Path(self.temp.name) / "reference.csv"

    def write(self, path, rows, fields=HEADER):
        with path.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.DictWriter(handle, fieldnames=fields)
            writer.writeheader()
            writer.writerows(rows)

    def pair(self, ours=None, reference=None):
        self.write(self.ours, [BASE if ours is None else ours])
        self.write(self.reference, [BASE if reference is None else reference])
        return comparator.compare(self.ours, self.reference)

    def test_identical_vectors_have_zero_residual(self):
        self.assertEqual(
            self.pair(),
            dict.fromkeys(
                ("position_norm_km", "velocity_norm_km_per_day",
                 "max_position_component_km", "max_velocity_component_km_per_day"),
                0.0,
            ),
        )

    def test_known_position_and_velocity_residuals(self):
        other = dict(BASE, x_km="97", y_km="196", z_km="300",
                     vx_km_day="1", vy_km_day="-10", vz_km_day="3")
        metrics = self.pair(reference=other)
        self.assertEqual(metrics["position_norm_km"], 5.0)
        self.assertEqual(metrics["velocity_norm_km_per_day"], 12.0)
        self.assertEqual(metrics["max_position_component_km"], 4.0)
        self.assertEqual(metrics["max_velocity_component_km_per_day"], 12.0)

    def test_mismatched_metadata_rejected(self):
        for key, value in (
            ("jd_tdb", "2451545.00001"), ("target", "sun"),
            ("center", "ssb"), ("frame", "ECLIPTIC"),
            ("corrections", "APPARENT"),
        ):
            with self.subTest(field=key):
                with self.assertRaisesRegex(ValueError, "metadata mismatch"):
                    self.pair(reference=dict(BASE, **{key: value}))

    def test_unsupported_frame_or_correction_rejected_even_if_equal(self):
        for key, value in (("frame", "ECLIPTIC"), ("corrections", "APPARENT")):
            with self.subTest(field=key):
                changed = dict(BASE, **{key: value})
                with self.assertRaisesRegex(ValueError, "only ICRF geometric"):
                    self.pair(changed, changed)

    def test_missing_empty_and_nonfinite_values_rejected(self):
        for change in (
            {"x_km": ""}, {"x_km": "nan"}, {"x_km": "inf"},
            {"jd_tdb": "-inf"}, {"target": ""},
        ):
            with self.subTest(change=change):
                self.write(self.ours, [dict(BASE, **change)])
                with self.assertRaises(ValueError):
                    comparator.read_state(self.ours)
        missing = tuple(name for name in HEADER if name != "z_km")
        self.write(self.ours, [{k: BASE[k] for k in missing}], missing)
        with self.assertRaisesRegex(ValueError, "CSV header"):
            comparator.read_state(self.ours)

    def test_duplicate_and_extra_csv_headers_rejected(self):
        for header in (
            (*HEADER, "x_km"),
            (*HEADER, "unexpected"),
            tuple(name for name in HEADER if name != "x_km"),
        ):
            with self.subTest(header=header):
                with self.ours.open("w", newline="", encoding="utf-8") as handle:
                    writer = csv.writer(handle)
                    writer.writerow(header)
                    writer.writerow([BASE.get(name, "0") for name in header])
                with self.assertRaisesRegex(ValueError, "CSV header"):
                    comparator.read_state(self.ours)

    def test_short_and_long_data_rows_rejected(self):
        for values in (
            [BASE[name] for name in HEADER[:-1]],
            [BASE[name] for name in HEADER] + ["unexpected"],
        ):
            with self.subTest(column_count=len(values)):
                with self.ours.open("w", newline="", encoding="utf-8") as handle:
                    writer = csv.writer(handle)
                    writer.writerow(HEADER)
                    writer.writerow(values)
                with self.assertRaises(ValueError):
                    comparator.read_state(self.ours)

    def test_headerless_csv_rejected(self):
        self.ours.write_text("", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "missing CSV header"):
            comparator.read_state(self.ours)

    def test_reordered_columns_and_utf8_bom_accepted(self):
        # Independent reference converters may emit columns in a different order,
        # while spreadsheet exports commonly include a UTF-8 BOM.
        reordered = tuple(reversed(HEADER))
        self.write(self.ours, [BASE], reordered)
        self.write(self.reference, [BASE])
        self.ours.write_bytes(b"\xef\xbb\xbf" + self.ours.read_bytes())
        metrics = comparator.compare(self.ours, self.reference)
        self.assertTrue(all(value == 0.0 for value in metrics.values()))

    def test_nonfinite_velocity_components_rejected(self):
        for value in ("nan", "inf", "-inf"):
            with self.subTest(value=value):
                self.write(self.ours, [dict(BASE, vz_km_day=value)])
                with self.assertRaisesRegex(ValueError, "non-finite state component"):
                    comparator.read_state(self.ours)

    def test_multiple_or_zero_rows_rejected(self):
        for rows in ([], [BASE, BASE]):
            self.write(self.ours, rows)
            with self.assertRaisesRegex(ValueError, "exactly one data row"):
                comparator.read_state(self.ours)

    def test_finite_components_with_overflowing_norm_rejected(self):
        # math.hypot is overflow-resistant: 1e308 * sqrt(3) remains finite.
        # 1.5e308 * sqrt(3) exceeds the f64 maximum, exercising the guard.
        ours = dict(BASE, x_km=str(1.5e308), y_km=str(1.5e308), z_km=str(1.5e308))
        reference = dict(BASE, x_km="0", y_km="0", z_km="0")
        with self.assertRaisesRegex(ValueError, "non-finite residual metric"):
            self.pair(ours, reference)

    def test_cli_reports_zero_metrics_for_identical_vectors(self):
        self.write(self.ours, [BASE])
        self.write(self.reference, [BASE])
        result = subprocess.run(
            [sys.executable, str(SCRIPT), str(self.ours), str(self.reference)],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        lines = result.stdout.splitlines()
        self.assertEqual(len(lines), 4)
        self.assertEqual(
            {line.split("=", 1)[0] for line in lines},
            {"position_norm_km", "velocity_norm_km_per_day",
             "max_position_component_km", "max_velocity_component_km_per_day"},
        )
        self.assertTrue(all(float(line.split("=", 1)[1]) == 0.0 for line in lines))

    def test_cli_rejects_mismatched_metadata_without_metrics(self):
        self.write(self.ours, [BASE])
        self.write(self.reference, [dict(BASE, center="ssb")])
        result = subprocess.run(
            [sys.executable, str(SCRIPT), str(self.ours), str(self.reference)],
            capture_output=True, text=True, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("metadata mismatch for center", result.stderr)
        self.assertEqual(result.stdout, "")

    def test_overflow_residual_rejected(self):
        ours = dict(BASE, x_km=str(1e308))
        reference = dict(BASE, x_km=str(-1e308))
        with self.assertRaisesRegex(ValueError, "non-finite residual"):
            self.pair(ours, reference)


if __name__ == "__main__":
    unittest.main()
