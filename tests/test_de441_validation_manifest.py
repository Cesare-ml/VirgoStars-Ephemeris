"""Offline unit tests for DE441 validation metadata preflight."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

script = Path(__file__).resolve().parents[1] / "tools" / "check_de441_validation_manifest.py"
spec = importlib.util.spec_from_file_location("de441_preflight", script)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def valid():
    return {
        "kernel_url": "https://example.invalid/de441.eph",
        "kernel_sha256": "a" * 64,
        "kernel_bytes": 123,
        "retrieved_utc": "2026-11-12",
        "engine_commit": "example-commit",
        "reference_source": "JPL Horizons",
        "reference_solution": "DE441",
        "reference_query": "explicit saved Horizons VECTORS query",
        "tolerance_position_km": 0.001,
        "tolerance_velocity_km_day": 0.001,
        "samples": [{
            "jd_tdb": 2451545.0, "target_id": 301, "center_id": 399,
            "frame": "ICRF", "correction": "NONE", "reference_plane": "FRAME",
            "position_unit": "km", "velocity_unit": "km/day",
        }],
    }


class ManifestPreflightTests(unittest.TestCase):
    def test_reject_duplicate_top_level_json_key(self):
        raw = '{"reference_solution":"DE440","reference_solution":"DE441"}'
        with self.assertRaisesRegex(ValueError, "duplicate JSON key: reference_solution"):
            json.loads(raw, object_pairs_hook=module.reject_duplicate_keys)

    def test_reject_duplicate_nested_sample_key(self):
        raw = '{"samples":[{"target_id":499,"target_id":4}]}'
        with self.assertRaisesRegex(ValueError, "duplicate JSON key: target_id"):
            json.loads(raw, object_pairs_hook=module.reject_duplicate_keys)

    def test_valid_json_object_pairs(self):
        raw = '{"samples":[{"target_id":301,"center_id":399}]}'
        self.assertEqual(
            json.loads(raw, object_pairs_hook=module.reject_duplicate_keys),
            {"samples": [{"target_id": 301, "center_id": 399}]},
        )

    def test_reject_nonstandard_json_constants_even_in_extra_fields(self):
        for constant in ("NaN", "Infinity", "-Infinity"):
            with self.subTest(constant=constant):
                raw = '{"extra": ' + constant + '}'
                with self.assertRaisesRegex(ValueError, "non-standard JSON"):
                    json.loads(raw, object_pairs_hook=module.reject_duplicate_keys,
                               parse_constant=module.reject_nonfinite_json_constant)

    def test_strict_json_accepts_ordinary_finite_numbers(self):
        self.assertEqual(
            json.loads('{"extra": 1.25}', parse_constant=module.reject_nonfinite_json_constant),
            {"extra": 1.25},
        )

    def test_reject_overflowing_json_float_even_in_extra_fields(self):
        for literal in ("1e999", "-1e999"):
            with self.subTest(literal=literal):
                raw = '{"extra": ' + literal + '}'
                with self.assertRaisesRegex(ValueError, "outside finite float range"):
                    json.loads(raw, parse_float=module.reject_nonfinite_json_float)

    def test_accept_finite_json_float_boundaries(self):
        for literal in ("1e308", "-1e308", "1e-999"):
            with self.subTest(literal=literal):
                result = json.loads('{"extra": ' + literal + '}',
                                    parse_float=module.reject_nonfinite_json_float)
                self.assertEqual(result["extra"], float(literal))

    def test_valid_metadata(self):
        self.assertEqual(module.validate_manifest(valid()), 1)

    def test_reject_missing_provenance(self):
        value = valid()
        del value["kernel_sha256"]
        with self.assertRaisesRegex(ValueError, "missing fields"):
            module.validate_manifest(value)

    def test_reject_different_ephemeris(self):
        value = valid()
        value["reference_solution"] = "DE440"
        with self.assertRaisesRegex(ValueError, "DE441"):
            module.validate_manifest(value)

    def test_reject_incompatible_corrections(self):
        value = valid()
        value["samples"][0]["correction"] = "LT+S"
        with self.assertRaisesRegex(ValueError, "correction"):
            module.validate_manifest(value)

    def test_reject_nonfinite_tolerance(self):
        value = valid()
        value["tolerance_position_km"] = float("nan")
        with self.assertRaisesRegex(ValueError, "finite"):
            module.validate_manifest(value)

    def test_reject_bool_epoch(self):
        value = valid()
        value["samples"][0]["jd_tdb"] = True
        with self.assertRaisesRegex(ValueError, "Julian date"):
            module.validate_manifest(value)

    def test_reject_same_target_center(self):
        value = valid()
        value["samples"][0]["center_id"] = 301
        with self.assertRaisesRegex(ValueError, "differ"):
            module.validate_manifest(value)


if __name__ == "__main__":
    unittest.main()
