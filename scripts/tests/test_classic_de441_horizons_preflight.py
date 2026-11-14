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

"""Synthetic unit tests for classic DE441/Horizons SHA-256 preflight.

The fake 2844-byte header is NOT an official DE441 kernel; all Cargo
invocations here are mocks. No scientific results are asserted.
"""

import hashlib
import importlib.util
from pathlib import Path
from types import SimpleNamespace
from unittest import mock
import json
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "validate_classic_de441_horizons.py"
SPEC = importlib.util.spec_from_file_location("validate_classic_de441_horizons", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class ClassicDe441HorizonsPreflightTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.kernel = root / "fictional-de441.eph"
        self.reference = root / "fictional-reference.csv"
        header = bytearray(2844)
        magic = b"JPL Planetary Ephemeris DE441"
        header[:len(magic)] = magic
        header[2840:2844] = (441).to_bytes(4, "little", signed=True)
        self.kernel.write_bytes(header)
        self.reference.write_text("fake independently pinned reference\n", encoding="utf-8")
        self.expected_kernel = hashlib.sha256(self.kernel.read_bytes()).hexdigest()
        self.expected_reference = hashlib.sha256(self.reference.read_bytes()).hexdigest()

    def inputs(self):
        return (
            self.kernel, self.reference,
            self.expected_kernel, self.expected_reference, "0.01", "0.0001",
        )

    def test_pinned_identical_bytes_and_explicit_positive_tolerances(self):
        with mock.patch.object(validator, "MIN_CLASSIC_BYTES", 0):
            receipt = validator.preflight(*self.inputs())
        self.assertEqual(receipt["kernel_sha256"], self.expected_kernel)
        self.assertEqual(receipt["reference_sha256"], self.expected_reference)
        self.assertEqual(receipt["kernel_bytes"], 2844)
        self.assertEqual(receipt["position_component_tolerance_km"], 0.01)
        self.assertEqual(receipt["velocity_component_tolerance_km_per_day"], 0.0001)

    def test_mismatched_and_invalid_hashes_fail_closed(self):
        for position in (2, 3):
            values = list(self.inputs())
            values[position] = "a" * 64 if values[position] != "a" * 64 else "b" * 64
            with mock.patch.object(validator, "MIN_CLASSIC_BYTES", 0):
                with self.assertRaisesRegex(ValueError, "SHA-256 differs"):
                    validator.preflight(*values)
        for bad in ("x" * 64, "0" * 63, "", "a" * 65):
            values = list(self.inputs())
            values[2] = bad
            with self.assertRaisesRegex(ValueError, "64 hexadecimal"):
                validator.preflight(*values)

    def test_modified_kernel_or_reference_cannot_be_reused(self):
        with mock.patch.object(validator, "MIN_CLASSIC_BYTES", 0):
            self.kernel.write_bytes(self.kernel.read_bytes() + b"!")
            with self.assertRaisesRegex(ValueError, "kernel SHA-256 differs"):
                validator.preflight(*self.inputs())
            # Restore only the kernel and instead modify the reference.
            self.kernel.write_bytes(self.kernel.read_bytes()[:-1])
            self.reference.write_bytes(self.reference.read_bytes() + b"!")
            with self.assertRaisesRegex(ValueError, "reference SHA-256 differs"):
                validator.preflight(*self.inputs())

    def test_rejects_short_file_and_spk_disguised_as_classic(self):
        with self.assertRaisesRegex(ValueError, "too small"):
            validator.preflight(*self.inputs())
        self.kernel.write_bytes(b"DAF/SPK " + b"0" * 3000)
        with mock.patch.object(validator, "MIN_CLASSIC_BYTES", 0):
            with self.assertRaisesRegex(ValueError, "not a classic"):
                validator.preflight(*self.inputs())

    def test_rejects_nonfinite_zero_and_negative_tolerances(self):
        for value in ("0", "-2", "inf", "nan", "oops"):
            with self.subTest(value=value):
                with self.assertRaisesRegex(ValueError, "strictly positive"):
                    validator.positive_tolerance(value, "position")

    def test_preflight_completes_before_mocked_cargo_and_does_not_certify_science(self):
        runner = mock.Mock(return_value=SimpleNamespace(
            returncode=0, stdout="synthetic comparator stdout", stderr="",
        ))
        with mock.patch.object(validator, "MIN_CLASSIC_BYTES", 0):
            result = validator.compare(
                *self.inputs(), runner=runner,
                source_checker=lambda: {
                "source_git_commit_sha": "a" * 40,
                "source_tree_clean": True,
            },
            )
        runner.assert_called_once()
        args, kwargs = runner.call_args
        self.assertEqual(args[0][:8], [
            "cargo", "run", "--locked", "--quiet", "-p",
            "virgostars-ephemeris-core", "--example", "validate_horizons",
        ])
        self.assertEqual(args[0][-4:], [
            str(self.kernel.resolve()), str(self.reference.resolve()), "0.01", "0.0001",
        ])
        self.assertFalse(kwargs["check"])
        self.assertFalse(result["scientific_validated"])
        self.assertEqual(
            result["status"], "COMPARISON_PASSED_PENDING_INDEPENDENT_PROVENANCE_REVIEW",
        )
        with mock.patch.object(validator, "MIN_CLASSIC_BYTES", 0):
            with self.assertRaisesRegex(ValueError, "kernel SHA-256 differs"):
                validator.compare(
                    self.kernel, self.reference, "0" * 64, self.expected_reference,
                    "0.01", "0.0001", runner=runner,
                )
        self.assertEqual(runner.call_count, 1)

    def test_cargo_failure_is_explicit_and_does_not_claim_validation(self):
        runner = mock.Mock(return_value=SimpleNamespace(
            returncode=101, stdout="", stderr="mocked invalid reference",
        ))
        with mock.patch.object(validator, "MIN_CLASSIC_BYTES", 0):
            result = validator.compare(
                *self.inputs(), runner=runner,
                source_checker=lambda: {
                "source_git_commit_sha": "a" * 40,
                "source_tree_clean": True,
            },
            )
        self.assertEqual(result["status"], "COMPARISON_FAILED")
        self.assertEqual(result["cargo_exit_code"], 101)
        self.assertFalse(result["scientific_validated"])

    def test_cli_records_preflight_failure_without_corrupting_input(self):
        receipt = Path(self.temp.name) / "validation-receipt.json"
        with mock.patch.object(validator, "MIN_CLASSIC_BYTES", 0):
            rc = validator.main([
                "--kernel", str(self.kernel),
                "--reference", str(self.reference),
                "--kernel-sha256", "0" * 64,
                "--reference-sha256", self.expected_reference,
                "--position-km", "0.01",
                "--velocity-km-per-day", "0.0001",
                "--receipt", str(receipt),
            ])
        self.assertEqual(rc, 1)
        report = json.loads(receipt.read_text(encoding="utf-8"))
        self.assertEqual(report["status"], "PREFLIGHT_FAILED")
        self.assertFalse(report["scientific_validated"])
        self.assertEqual(hashlib.sha256(self.kernel.read_bytes()).hexdigest(), self.expected_kernel)
        self.assertEqual(hashlib.sha256(self.reference.read_bytes()).hexdigest(), self.expected_reference)

    def test_source_provenance_pins_git_head_and_clean_checkout(self):
        root = SCRIPT.resolve().parents[1]
        outputs = [
            SimpleNamespace(returncode=0, stdout=str(root) + "\\n"),
            SimpleNamespace(returncode=0, stdout="a" * 40 + "\\n"),
            SimpleNamespace(returncode=0, stdout=""),
        ]
        git_runner = mock.Mock(side_effect=outputs)
        evidence = validator.source_provenance(git_runner=git_runner)
        self.assertEqual(evidence["source_git_commit_sha"], "a" * 40)
        self.assertTrue(evidence["source_tree_clean"])
        self.assertEqual([call.args[0] for call in git_runner.call_args_list], [
            ["git", "rev-parse", "--show-toplevel"],
            ["git", "rev-parse", "HEAD"],
            ["git", "status", "--porcelain=v1", "--untracked-files=all"],
        ])
        for call in git_runner.call_args_list:
            self.assertEqual(call.kwargs["cwd"], root)
            self.assertFalse(call.kwargs["check"])

    def test_dirty_checkout_is_rejected_even_with_valid_commit(self):
        root = SCRIPT.resolve().parents[1]
        responses = [
            SimpleNamespace(returncode=0, stdout=str(root) + "\\n"),
            SimpleNamespace(returncode=0, stdout="b" * 40 + "\\n"),
            SimpleNamespace(returncode=0, stdout="?? patched-untracked.rs\\n"),
        ]
        with self.assertRaisesRegex(ValueError, "worktree must be clean"):
            validator.source_provenance(git_runner=mock.Mock(side_effect=responses))

    def test_wrong_repo_invalid_head_and_git_failure_fail_closed(self):
        root = SCRIPT.resolve().parents[1]
        scenarios = [
            ([SimpleNamespace(returncode=0, stdout=str(root / "wrong") + "\\n")], "repository root"),
            ([
                SimpleNamespace(returncode=0, stdout=str(root) + "\\n"),
                SimpleNamespace(returncode=0, stdout="not-a-full-git-sha"),
            ], "40-hex"),
            ([SimpleNamespace(returncode=128, stdout="")], "failed"),
        ]
        for responses, reason in scenarios:
            with self.subTest(reason=reason):
                with self.assertRaisesRegex(ValueError, reason):
                    validator.source_provenance(git_runner=mock.Mock(side_effect=responses))

    def test_unverifiable_source_prevents_cargo_after_valid_hashes(self):
        runner = mock.Mock()
        with mock.patch.object(validator, "MIN_CLASSIC_BYTES", 0):
            with self.assertRaisesRegex(ValueError, "source tree dirty"):
                validator.compare(
                    *self.inputs(), runner=runner,
                    source_checker=mock.Mock(side_effect=ValueError("source tree dirty")),
                )
        runner.assert_not_called()

    def test_receipt_must_not_overwrite_kernel_or_reference(self):
        for receipt in (self.kernel, self.reference):
            with self.subTest(receipt=receipt.name):
                original = receipt.read_bytes()
                with mock.patch.object(validator, "MIN_CLASSIC_BYTES", 0):
                    rc = validator.main([
                        "--kernel", str(self.kernel),
                        "--reference", str(self.reference),
                        "--kernel-sha256", self.expected_kernel,
                        "--reference-sha256", self.expected_reference,
                        "--position-km", "0.01",
                        "--velocity-km-per-day", "0.0001",
                        "--receipt", str(receipt),
                    ])
                self.assertEqual(rc, 1)
                self.assertEqual(receipt.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
