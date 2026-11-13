# EXHIBIT A. Common Public Attribution License Version 1.0.
# The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
# Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
# The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
# The Original Developer is the Initial Developer.
# The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
# Contributors retain applicable rights in their own contributions under the terms of this License.
# [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code for Your Modifications.]
# SPDX-License-Identifier: CPAL-1.0

"""Synthetic input tests for E1 matrix breadth; NO astronomical accuracy claims."""

import importlib.util
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest


MODULE_PATH = Path(__file__).resolve().parents[1] / "audit_spk_e1_matrix.py"
SPEC = importlib.util.spec_from_file_location("audit_spk_e1_matrix", MODULE_PATH)
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)

METADATA = (
    "# provider=NAIF_SPICE\n"
    "# method=spkezr\n"
    "# time_scale=TDB\n"
    "# frame=J2000\n"
    "# corrections=NONE\n"
    "# units=KM-S\n"
    "# toolkit_version=synthetic-not-real-toolkit\n"
    "# generated_utc=2026-11-13T00:00:00Z\n"
    "# reference_source=synthetic-unit-test-not-spice\n"
    "# ordered_kernel_sha256=" + "a" * 64 + "\n"
)


def synthetic_document(pairs, times=(0.0, 86400.0, 172800.0)):
    rows = [f"{et},{target},{center},1,2,3,4,5,6"
            for target, center in pairs for et in times]
    return METADATA + AUDIT.COLUMNS + "\n" + "\n".join(rows) + "\n"


CORE = [(10, 399), (301, 399), (399, 0)]
ALL_PAIRS = CORE + [(4, 399), (5, 399)]


def matched_synthetic_receipt():
    """Fabricated metadata for consistency tests, never NAIF evidence."""
    import hashlib
    document = synthetic_document(ALL_PAIRS).replace(
        AUDIT.COLUMNS,
        "# generator_sha256=" + "b" * 64 + "\n"
        "# ordered_kernel_names=synthetic.bsp\n"
        "# ordered_kernel_sizes_bytes=42\n"
        + AUDIT.COLUMNS,
    )
    receipt = {
        "status": "COMPARATOR_PASS_PROVENANCE_REVIEW_PENDING",
        "scientific_accuracy_certified": False,
        "provenance_review_required": True,
        "reference_sha256": hashlib.sha256(document.encode("utf-8")).hexdigest(),
        "case_count": 15,
        "provider": "NAIF_SPICE",
        "method": "spkezr",
        "frame": "J2000",
        "corrections": "NONE",
        "time_scale": "TDB_SECONDS_PAST_J2000",
        "position_units": "km",
        "velocity_units": "km/s",
        "max_position_component_km": 0.001,
        "max_velocity_component_km_s": 0.000001,
        "toolkit_version": "synthetic-not-real-toolkit",
        "generator_sha256": "b" * 64,
        "ordered_kernels": [
            {"name": "synthetic.bsp", "size_bytes": 42, "sha256": "a" * 64}
        ],
        "rust_command": [
            "cargo", "run", "--offline", "--locked", "--example",
            "validate_spk_reference", "--",
        ],
    }
    return document, receipt


class E1MatrixAuditTests(unittest.TestCase):
    def test_complete_synthetic_five_pair_three_epoch_matrix_passes_breadth(self):
        rows = AUDIT.parse_reference(synthetic_document(ALL_PAIRS))
        self.assertEqual(len(rows), 15)
        issues, planets, shared = AUDIT.audit_e1_matrix(rows)
        self.assertEqual(issues, [])
        self.assertEqual(planets, [(4, 399), (5, 399)])
        self.assertEqual(shared, [0, 86400, 172800])

    def test_missing_moon_pair_is_incomplete(self):
        rows = AUDIT.parse_reference(synthetic_document([CORE[0], CORE[2], *ALL_PAIRS[3:]]))
        issues, _, _ = AUDIT.audit_e1_matrix(rows)
        self.assertTrue(any("Moon relative to Earth" in item for item in issues))

    def test_one_planet_even_with_two_observers_is_incomplete(self):
        rows = AUDIT.parse_reference(synthetic_document(CORE + [(4, 399), (4, 0)]))
        issues, _, _ = AUDIT.audit_e1_matrix(rows)
        self.assertTrue(any("two distinct planetary" in item for item in issues))

    def test_planet_center_and_barycenter_are_one_physical_system(self):
        # 4 = Mars system barycenter; 499 = Mars body center.
        # Different NAIF IDs must not masquerade as two planets.
        rows = AUDIT.parse_reference(synthetic_document(CORE + [(4, 399), (499, 399)]))
        issues, planets, shared = AUDIT.audit_e1_matrix(rows)
        self.assertTrue(any("two distinct planetary systems" in issue for issue in issues))
        self.assertEqual(planets, [])
        self.assertEqual(shared, [])

    def test_mixed_planet_center_and_barycenter_plus_another_planet_passes(self):
        rows = AUDIT.parse_reference(
            synthetic_document(CORE + [(4, 399), (499, 399), (5, 399)])
        )
        issues, planets, shared = AUDIT.audit_e1_matrix(rows)
        self.assertEqual(issues, [])
        self.assertEqual(planets, [(4, 399), (5, 399)])
        self.assertEqual(len(shared), 3)

    def test_all_supported_naif_centers_map_to_correct_barycenter_system(self):
        for system in (1, 2, 4, 5, 6, 7, 8, 9):
            with self.subTest(system=system):
                self.assertEqual(AUDIT.planet_system(system), system)
                self.assertEqual(AUDIT.planet_system(system * 100 + 99), system)
        for unrelated in (0, 3, 10, 301, 399, 500, 100000):
            with self.subTest(unrelated=unrelated):
                self.assertIsNone(AUDIT.planet_system(unrelated))

    def test_disjoint_epoch_sets_do_not_satisfy_shared_epoch_requirement(self):
        base = AUDIT.parse_reference(synthetic_document(CORE))
        planets = AUDIT.parse_reference(synthetic_document([(4, 399), (5, 399)], (100, 200, 300)))
        issues, _, _ = AUDIT.audit_e1_matrix(base + planets)
        self.assertTrue(any("share three ET epochs" in item for item in issues))

    def test_only_two_epochs_for_core_pair_fail_even_if_other_rows_pass(self):
        rows = AUDIT.parse_reference(synthetic_document(ALL_PAIRS))
        rows = [row for row in rows if row != (301, 399, 172800.0)]
        issues, _, _ = AUDIT.audit_e1_matrix(rows)
        self.assertTrue(any("Moon relative to Earth" in item for item in issues))

    def test_wrong_sun_center_does_not_count_as_required_pair(self):
        rows = AUDIT.parse_reference(synthetic_document([(10, 0), *ALL_PAIRS[1:]]))
        issues, _, _ = AUDIT.audit_e1_matrix(rows)
        self.assertTrue(any("Sun relative to Earth" in item for item in issues))

    def test_duplicate_rows_and_signed_zero_rejected(self):
        doc = synthetic_document(ALL_PAIRS)
        self.assertRaisesRegex(ValueError, "duplicate case",
                               AUDIT.parse_reference, doc + "0,10,399,1,2,3,4,5,6\n")
        self.assertRaisesRegex(ValueError, "duplicate case",
                               AUDIT.parse_reference, doc + "-0,10,399,1,2,3,4,5,6\n")

    def test_rejects_invalid_time_vector_ids_and_same_target(self):
        doc = synthetic_document(ALL_PAIRS)
        source = "0.0,10,399,1,2,3,4,5,6"
        for mutated in (
            "nan,10,399,1,2,3,4,5,6",
            "0.0,10,399,1,2,3,4,5,inf",
            "0.0,10.0,399,1,2,3,4,5,6",
            "0.0,10,10,1,2,3,4,5,6",
            "0.0,99999999999999,399,1,2,3,4,5,6",
            "0.0,10,399,1,2,3,4,5",
        ):
            with self.subTest(mutated=mutated), self.assertRaises(ValueError):
                AUDIT.parse_reference(doc.replace(source, mutated, 1))

    def test_rejects_malformed_contract_and_late_metadata(self):
        doc = synthetic_document(ALL_PAIRS)
        for altered in (
            doc.replace("frame=J2000", "frame=ECLIPDATE"),
            doc.replace("time_scale=TDB", "time_scale=UTC"),
            doc.replace("units=KM-S", "units=KM-D"),
            doc.replace("corrections=NONE", "corrections=LT"),
            doc.replace("ordered_kernel_sha256=" + "a"*64,
                        "ordered_kernel_sha256=bad"),
            doc + "# frame=J2000\n",
            "# provider=NAIF_SPICE\n" + doc,
            doc.replace(AUDIT.COLUMNS, "unexpected,columns"),
        ):
            with self.subTest(altered=altered[:60]), self.assertRaises(ValueError):
                AUDIT.parse_reference(altered)

    def test_plan_explicit_e1_matrix_passes_same_audit_as_csv(self):
        cases = AUDIT.plan_e1_cases([4, 5], [0.0, 86400.0, 172800.0])
        self.assertEqual(len(cases), 15)
        self.assertEqual(cases[:3], [(10, 399, 0.0), (10, 399, 86400.0),
                                     (10, 399, 172800.0)])
        self.assertEqual(cases[-3:], [(5, 399, 0.0), (5, 399, 86400.0),
                                       (5, 399, 172800.0)])
        self.assertEqual(AUDIT.audit_e1_matrix(cases)[0], [])

    def test_plan_no_default_planets_or_epochs(self):
        for planets, times in (
            ([], [0.0, 86400.0, 172800.0]),
            ([4], [0.0, 86400.0, 172800.0]),
            ([4, 5], []),
            ([4, 5], [0.0, 86400.0]),
        ):
            with self.subTest(planets=planets, times=times), self.assertRaises(ValueError):
                AUDIT.plan_e1_cases(planets, times)

    def test_plan_rejects_same_physical_planet_and_unsupported_ids(self):
        for planets in ([4, 499], [5, 599], [4, 4], [3, 4],
                        [301, 4], [399, 5]):
            with self.subTest(planets=planets), self.assertRaises(ValueError):
                AUDIT.plan_e1_cases(planets, [0.0, 86400.0, 172800.0])
        self.assertEqual(
            len(AUDIT.plan_e1_cases([499, 5], [0.0, 86400.0, 172800.0])),
            15,
        )

    def test_plan_rejects_duplicate_and_nonfinite_epochs(self):
        for epochs in ([0.0, -0.0, 86400.0], [0.0, float("nan"), 1.0],
                       [0.0, float("inf"), 1.0], [0.0, 1.0, float("-inf")]):
            with self.subTest(epochs=epochs), self.assertRaises(ValueError):
                AUDIT.plan_e1_cases([4, 5], epochs)

    def test_plan_allows_four_user_supplied_epochs_without_implicit_values(self):
        epochs = [-86400.0, 0.0, 123.5, 86400.0]
        cases = AUDIT.plan_e1_cases([5, 499], epochs)
        self.assertEqual(len(cases), 20)
        self.assertEqual({et for _, _, et in cases}, set(epochs))
        self.assertEqual(AUDIT.audit_e1_matrix(cases)[0], [])

    def test_plan_cli_prints_only_generator_compatible_case_flags(self):
        from contextlib import redirect_stdout, redirect_stderr
        from io import StringIO

        out, err = StringIO(), StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = AUDIT.main(["--plan", "--planet", "4", "--planet", "5",
                               "--epoch", "0", "--epoch", "86400",
                               "--epoch", "172800"])
        self.assertEqual(code, 0)
        flags = out.getvalue().splitlines()
        self.assertEqual(len(flags), 15)
        self.assertEqual(flags[0], "--case 10:399:0")
        self.assertEqual(flags[-1], "--case 5:399:172800")
        self.assertTrue(all(flag.startswith("--case ") for flag in flags))
        self.assertIn("NOT CERTIFIED", err.getvalue())

    def test_plan_cli_fails_closed_on_incorrect_usage(self):
        from contextlib import redirect_stderr
        from io import StringIO

        for args in (
            ["--plan", "--planet", "4", "--planet", "499",
             "--epoch", "0", "--epoch", "86400", "--epoch", "172800"],
            ["--plan", "--planet", "4", "--planet", "5",
             "--epoch", "0", "--epoch", "0", "--epoch", "172800"],
            ["--reference", "/unused.csv", "--plan",
             "--planet", "4", "--planet", "5",
             "--epoch", "0", "--epoch", "86400", "--epoch", "172800"],
            ["--epoch", "0", "--reference", "/unused.csv"],
            [],
        ):
            with self.subTest(args=args), redirect_stderr(StringIO()):
                self.assertEqual(AUDIT.main(args), 2)

    def test_json_pass_binds_exact_input_bytes_and_declines_accuracy_claims(self):
        from contextlib import redirect_stdout, redirect_stderr
        from io import StringIO
        import hashlib
        import json

        with TemporaryDirectory() as folder:
            csv = Path(folder) / "reference.csv"
            csv.write_text(synthetic_document(ALL_PAIRS), encoding="utf-8")
            output, errors = StringIO(), StringIO()
            with redirect_stdout(output), redirect_stderr(errors):
                code = AUDIT.main(["--reference", str(csv), "--json"])
            report = json.loads(output.getvalue())
            self.assertEqual(code, 0)
            self.assertEqual(errors.getvalue(), "")
            self.assertEqual(report["schema"], "virgostars-e1-matrix-audit/v1")
            self.assertEqual(report["status"], "pass")
            self.assertEqual(report["reference_sha256"],
                             hashlib.sha256(csv.read_bytes()).hexdigest())
            self.assertEqual(report["case_count"], 15)
            self.assertEqual(report["planet_pairs"], [[4, 399], [5, 399]])
            self.assertEqual(report["shared_et_tdb_seconds"], ["0", "86400", "172800"])
            self.assertEqual(report["issues"], [])
            self.assertFalse(report["scientific_accuracy_certified"])
            self.assertTrue(report["provenance_review_required"])

    def test_json_incomplete_returns_nonzero_and_no_false_pass(self):
        from contextlib import redirect_stdout
        from io import StringIO
        import json

        with TemporaryDirectory() as folder:
            csv = Path(folder) / "reference.csv"
            csv.write_text(synthetic_document(CORE), encoding="utf-8")
            output = StringIO()
            with redirect_stdout(output):
                code = AUDIT.main(["--json", "--reference", str(csv)])
            report = json.loads(output.getvalue())
            self.assertEqual(code, 1)
            self.assertEqual(report["status"], "incomplete")
            self.assertEqual(report["case_count"], 9)
            self.assertTrue(any("two distinct" in x for x in report["issues"]))
            self.assertEqual(report["planet_pairs"], [])
            self.assertFalse(report["scientific_accuracy_certified"])

    def test_json_invalid_input_returns_only_machine_readable_error(self):
        from contextlib import redirect_stdout, redirect_stderr
        from io import StringIO
        import json

        with TemporaryDirectory() as folder:
            csv = Path(folder) / "reference.csv"
            csv.write_bytes(b"not,valid,columns\n")
            output, errors = StringIO(), StringIO()
            with redirect_stdout(output), redirect_stderr(errors):
                code = AUDIT.main(["--json", "--reference", str(csv)])
            report = json.loads(output.getvalue())
            self.assertEqual(code, 2)
            self.assertEqual(errors.getvalue(), "")
            self.assertEqual(report["status"], "invalid")
            self.assertTrue(report["issues"])
            self.assertNotIn("reference_sha256", report)
            self.assertFalse(report["scientific_accuracy_certified"])

    def test_json_reports_reject_invalid_utf8_without_crashing(self):
        from contextlib import redirect_stdout
        from io import StringIO
        import json

        with TemporaryDirectory() as folder:
            csv = Path(folder) / "reference.csv"
            csv.write_bytes(b"\xff\xfe")
            output = StringIO()
            with redirect_stdout(output):
                self.assertEqual(AUDIT.main(["--reference", str(csv), "--json"]), 2)
            self.assertEqual(json.loads(output.getvalue())["status"], "invalid")

    def test_json_fingerprint_changes_on_only_line_endings(self):
        from contextlib import redirect_stdout
        from io import StringIO
        import json

        with TemporaryDirectory() as folder:
            csv = Path(folder) / "reference.csv"
            content = synthetic_document(ALL_PAIRS)
            digests = []
            for raw in (content.encode("utf-8"),
                        content.replace("\n", "\r\n").encode("utf-8")):
                csv.write_bytes(raw)
                output = StringIO()
                with redirect_stdout(output):
                    self.assertEqual(
                        AUDIT.main(["--reference", str(csv), "--json"]), 0
                    )
                digests.append(json.loads(output.getvalue())["reference_sha256"])
            self.assertNotEqual(digests[0], digests[1])

    def test_json_and_plan_cannot_be_combined(self):
        from contextlib import redirect_stdout
        from io import StringIO
        import json

        output = StringIO()
        with redirect_stdout(output):
            code = AUDIT.main(["--plan", "--json", "--planet", "4",
                               "--planet", "5", "--epoch", "0",
                               "--epoch", "86400", "--epoch", "172800"])
        self.assertEqual(code, 2)
        self.assertEqual(json.loads(output.getvalue())["status"], "invalid")

    def test_matching_numeric_receipt_declares_consistency_not_accuracy(self):
        import hashlib

        document, receipt = matched_synthetic_receipt()
        AUDIT.verify_run_receipt(
            document, hashlib.sha256(document.encode("utf-8")).hexdigest(),
            15, receipt,
        )
        self.assertFalse(receipt["scientific_accuracy_certified"])
        self.assertTrue(receipt["provenance_review_required"])

    def test_receipt_rejects_changed_csv_or_false_digest(self):
        import hashlib
        import copy

        document, receipt = matched_synthetic_receipt()
        bad = copy.deepcopy(receipt)
        bad["reference_sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "reference SHA-256 differs"):
            AUDIT.verify_run_receipt(
                document, hashlib.sha256(document.encode()).hexdigest(), 15, bad
            )
        with self.assertRaisesRegex(ValueError, "reference SHA-256 differs"):
            AUDIT.verify_run_receipt(
                document, hashlib.sha256((document + " ").encode()).hexdigest(),
                15, receipt,
            )

    def test_receipt_rejects_wrong_case_count_or_boolean_count(self):
        import hashlib
        import copy

        document, receipt = matched_synthetic_receipt()
        for count in (14, 16, True, "15"):
            bad = copy.deepcopy(receipt)
            bad["case_count"] = count
            with self.subTest(count=count), self.assertRaisesRegex(ValueError, "case count"):
                AUDIT.verify_run_receipt(
                    document, hashlib.sha256(document.encode()).hexdigest(), 15, bad
                )

    def test_receipt_rejects_invalid_status_false_flags_and_contract(self):
        import hashlib
        import copy

        document, receipt = matched_synthetic_receipt()
        mutations = (
            ("status", "FAIL"),
            ("scientific_accuracy_certified", True),
            ("provenance_review_required", False),
            ("frame", "ECLIPDATE"),
            ("corrections", "LT"),
            ("time_scale", "UTC"),
            ("position_units", "m"),
            ("velocity_units", "km/day"),
            ("provider", "JPL_HORIZONS"),
        )
        for key, value in mutations:
            bad = copy.deepcopy(receipt)
            bad[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                AUDIT.verify_run_receipt(
                    document, hashlib.sha256(document.encode()).hexdigest(), 15, bad
                )

    def test_receipt_rejects_generator_toolkit_and_kernel_manifest_mismatches(self):
        import hashlib
        import copy

        document, receipt = matched_synthetic_receipt()
        bad_receipts = []
        for key, value in (
            ("generator_sha256", "0" * 64),
            ("toolkit_version", "WRONG"),
            ("ordered_kernels", []),
        ):
            bad = copy.deepcopy(receipt)
            bad[key] = value
            bad_receipts.append(bad)
        for field, value in (
            ("name", "different.bsp"),
            ("size_bytes", 43),
            ("size_bytes", True),
            ("sha256", "0" * 64),
        ):
            bad = copy.deepcopy(receipt)
            bad["ordered_kernels"][0][field] = value
            bad_receipts.append(bad)
        for i, bad in enumerate(bad_receipts):
            with self.subTest(i=i), self.assertRaises(ValueError):
                AUDIT.verify_run_receipt(
                    document, hashlib.sha256(document.encode()).hexdigest(), 15, bad
                )

    def test_receipt_rejects_unsafe_tolerances_and_noncanonical_command(self):
        import hashlib
        import copy

        document, receipt = matched_synthetic_receipt()
        for field in ("max_position_component_km", "max_velocity_component_km_s"):
            for bad_value in (0, -1, float("inf"), float("nan"), True, "1"):
                bad = copy.deepcopy(receipt)
                bad[field] = bad_value
                with self.subTest(field=field, value=bad_value), self.assertRaises(ValueError):
                    AUDIT.verify_run_receipt(
                        document, hashlib.sha256(document.encode()).hexdigest(), 15, bad
                    )
        bad = copy.deepcopy(receipt)
        bad["rust_command"] = ["python", "pretend-pass"]
        with self.assertRaisesRegex(ValueError, "canonical offline Rust"):
            AUDIT.verify_run_receipt(
                document, hashlib.sha256(document.encode()).hexdigest(), 15, bad
            )

    def test_receipt_cli_json_reports_consistency_on_matched_input(self):
        from contextlib import redirect_stdout, redirect_stderr
        from io import StringIO
        import json

        document, receipt = matched_synthetic_receipt()
        with TemporaryDirectory() as directory:
            csv = Path(directory) / "sample.csv"
            evidence = Path(directory) / "receipt.json"
            csv.write_bytes(document.encode())
            evidence.write_text(json.dumps(receipt), encoding="utf-8")
            out, err = StringIO(), StringIO()
            with redirect_stdout(out), redirect_stderr(err):
                result = AUDIT.main([
                    "--reference", str(csv), "--receipt", str(evidence), "--json"
                ])
            report = json.loads(out.getvalue())
            self.assertEqual(result, 0)
            self.assertEqual(err.getvalue(), "")
            self.assertEqual(report["status"], "pass")
            self.assertTrue(report["receipt_consistency_checked"])
            self.assertFalse(report["scientific_accuracy_certified"])

    def test_receipt_cli_fail_closed_on_mismatched_or_malformed_receipt(self):
        from contextlib import redirect_stdout
        from io import StringIO
        import json

        document, receipt = matched_synthetic_receipt()
        with TemporaryDirectory() as directory:
            csv = Path(directory) / "sample.csv"
            evidence = Path(directory) / "receipt.json"
            csv.write_bytes(document.encode())
            for content in (json.dumps({**receipt, "case_count": 99}),
                            "{not-json", "null"):
                evidence.write_text(content, encoding="utf-8")
                out = StringIO()
                with redirect_stdout(out):
                    result = AUDIT.main([
                        "--reference", str(csv), "--receipt", str(evidence), "--json"
                    ])
                report = json.loads(out.getvalue())
                self.assertEqual(result, 2)
                self.assertEqual(report["status"], "invalid")
                self.assertNotIn("reference_sha256", report)

    def test_receipt_cannot_be_used_with_plan(self):
        from contextlib import redirect_stderr
        from io import StringIO

        with redirect_stderr(StringIO()):
            self.assertEqual(AUDIT.main([
                "--plan", "--receipt", "/unused.json",
                "--planet", "4", "--planet", "5", "--epoch", "0",
                "--epoch", "86400", "--epoch", "172800",
            ]), 2)

    def test_cli_exit_status_distinguishes_pass_incomplete_and_invalid(self):
        with TemporaryDirectory() as folder:
            csv = Path(folder) / "reference.csv"
            csv.write_text(synthetic_document(ALL_PAIRS), encoding="utf-8")
            self.assertEqual(AUDIT.main(["--reference", str(csv)]), 0)
            csv.write_text(synthetic_document(CORE), encoding="utf-8")
            self.assertEqual(AUDIT.main(["--reference", str(csv)]), 1)
            csv.write_text("invalid", encoding="utf-8")
            self.assertEqual(AUDIT.main(["--reference", str(csv)]), 2)


if __name__ == "__main__":
    unittest.main()
