# EXHIBIT A. Common Public Attribution License Version 1.0.
# 
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

"""Offline, synthetic tests for the SPK reference producer, never JPL parity."""

import importlib.util
import hashlib
import json
import subprocess
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest


MODULE_PATH = Path(__file__).resolve().parents[1] / "spk_naif_reference.py"
SPEC = importlib.util.spec_from_file_location("spk_naif_reference", MODULE_PATH)
REFERENCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(REFERENCE)


class FakeSpice:
    def __init__(self, fail=False, bad_state=None):
        self.loaded = []
        self.clear_count = 0
        self.fail = fail
        self.bad_state = bad_state
        self.queries = []

    def kclear(self):
        self.clear_count += 1
        self.loaded.clear()

    def furnsh(self, path):
        self.loaded.append(path)

    def tkvrsn(self, item):
        if item != "TOOLKIT":
            raise AssertionError("unexpected toolkit query")
        return "CSPICE_N0067"

    def spkezr(self, target, et, frame, abcorr, observer):
        self.queries.append((target, et, frame, abcorr, observer, tuple(self.loaded)))
        if self.fail:
            raise RuntimeError("synthetic SPICE failure")
        if self.bad_state is not None:
            return self.bad_state, 0.0
        return [1.25, -2.5, 3.75, 0.1, -0.2, 0.3], 0.0


class SpkReferenceTests(unittest.TestCase):
    def setUp(self):
        self.folder = TemporaryDirectory()
        self.addCleanup(self.folder.cleanup)
        root = Path(self.folder.name)
        self.first = root / "first.bsp"
        self.second = root / "second.bsp"
        self.first.write_bytes(b"fake little-endian SPK payload, not a real kernel")
        self.second.write_bytes(b"fake big-endian SPK payload, not a real kernel")
        self.paths = [self.first, self.second]

    def reference(self, spice=None):
        return REFERENCE.generate_reference(
            self.paths,
            [(301, 399, 0.0), (399, 0, 123.25)],
            spice if spice is not None else FakeSpice(),
            generated_utc="2026-11-13T12:00:00Z",
            generator_digest=hashlib.sha256(MODULE_PATH.read_bytes()).hexdigest(),
        )

    def test_parses_explicit_et_without_converting_time_or_body_ids(self):
        self.assertEqual(REFERENCE.case_spec("301:399:0"), (301, 399, 0.0))
        self.assertEqual(REFERENCE.case_spec("-301:0:123.25"), (-301, 0, 123.25))
        self.assertEqual(REFERENCE.parse_cases(["301:399:0", "301:399:1"])[1][2], 1.0)

    def test_rejects_duplicate_same_body_invalid_epoch_and_ids(self):
        for value in (
            "301:301:0",
            "301:399:nan",
            "301:399:inf",
            "301.0:399:1",
            "301:999999999999:1",
            "301:399",
            "301:399:1:2",
        ):
            with self.subTest(value=value), self.assertRaises(ValueError):
                REFERENCE.case_spec(value)
        with self.assertRaises(ValueError):
            REFERENCE.parse_cases(["301:399:0", "301:399:-0"])
        with self.assertRaises(ValueError):
            REFERENCE.parse_cases([])

    def test_independent_generator_emits_required_provenance_and_exact_columns(self):
        spice = FakeSpice()
        document = self.reference(spice)
        self.assertIn("# provider=NAIF_SPICE\n", document)
        self.assertIn("# toolkit_version=CSPICE_N0067\n", document)
        self.assertIn("# reference_source=scripts/spk_naif_reference.py\n", document)
        self.assertIn("# generator_sha256=" + hashlib.sha256(MODULE_PATH.read_bytes()).hexdigest(), document)
        self.assertIn(REFERENCE.COLUMNS + "\n", document)
        self.assertIn("0,301,399,1.25,-2.5,3.75,0.10000000000000001", document)
        self.assertEqual(spice.clear_count, 2)
        self.assertEqual(len(spice.queries), 2)
        self.assertEqual(spice.queries[0][:5], ("301", 0.0, "J2000", "NONE", "399"))
        self.assertEqual(spice.queries[0][5], tuple(map(str, self.paths)))
        REFERENCE.verify_reference_hashes(document, self.paths)

    def test_hash_verification_rejects_modified_kernel_and_swapped_order(self):
        document = self.reference()
        REFERENCE.verify_reference_hashes(document, self.paths)
        with self.assertRaisesRegex(ValueError, "filename/order"):
            REFERENCE.verify_reference_hashes(document, list(reversed(self.paths)))
        self.second.write_bytes(self.second.read_bytes() + b"!")
        with self.assertRaisesRegex(ValueError, "byte size"):
            REFERENCE.verify_reference_hashes(document, self.paths)

    def test_hash_verification_rejects_same_size_tampering(self):
        document = self.reference()
        data = bytearray(self.first.read_bytes())
        data[0] ^= 1
        self.first.write_bytes(data)
        with self.assertRaisesRegex(ValueError, "SHA256 mismatch"):
            REFERENCE.verify_reference_hashes(document, self.paths)

    def test_missing_duplicate_or_invalid_metadata_and_wrong_kernel_count(self):
        document = self.reference()
        for changed in (
            document.replace("# generator_sha256=", "# invalid_generator_sha256="),
            document.replace("# ordered_kernel_sha256=", "# invalid_hashes="),
            document.replace(REFERENCE.COLUMNS, "unexpected,columns"),
            "# provider=NAIF_SPICE\n" + document,
        ):
            with self.assertRaises(ValueError):
                REFERENCE.verify_reference_hashes(changed, self.paths)
        with self.assertRaisesRegex(ValueError, "count mismatch"):
            REFERENCE.verify_reference_hashes(document, self.paths[:1])

    def test_generator_source_digest_is_verified_not_only_declared(self):
        document = self.reference()
        digest = hashlib.sha256(MODULE_PATH.read_bytes()).hexdigest()
        bad = document.replace(
            "# generator_sha256=" + digest,
            "# generator_sha256=" + "b" * 64,
        )
        with self.assertRaisesRegex(ValueError, "generator SHA256 mismatch"):
            REFERENCE.verify_reference_hashes(bad, self.paths)

    def test_naif_error_clears_pool_and_outputs_no_csv(self):
        spice = FakeSpice(fail=True)
        with self.assertRaisesRegex(RuntimeError, "synthetic SPICE failure"):
            self.reference(spice)
        self.assertEqual(spice.clear_count, 2)
        self.assertEqual(spice.loaded, [])

    def test_invalid_or_nonfinite_spice_vectors_are_rejected(self):
        for state in ([1.0] * 5, [1.0] * 5 + [float("nan")], [float("inf")] * 6):
            spice = FakeSpice(bad_state=state)
            with self.assertRaises(ValueError):
                self.reference(spice)
            self.assertEqual(spice.clear_count, 2)

    def test_generate_detects_changed_bytes_during_spice_call(self):
        file_to_mutate = self.first

        class MutatingSpice(FakeSpice):
            def spkezr(self, target, et, frame, abcorr, observer):
                file_to_mutate.write_bytes(file_to_mutate.read_bytes() + b"!")
                return super().spkezr(target, et, frame, abcorr, observer)

        with self.assertRaisesRegex(ValueError, "changed during NAIF generation"):
            self.reference(MutatingSpice())


    def test_one_command_audit_emits_replayable_receipt_only_after_rust_pass(self):
        reference = Path(self.folder.name) / "reference.csv"
        receipt = Path(self.folder.name) / "receipt.json"
        calls = []

        def passing_rust(command, *, cwd, check, capture_output, text):
            calls.append((command, cwd, check, capture_output, text))
            self.assertEqual(command[:6], [
                "cargo", "run", "--offline", "--locked", "--example", "validate_spk_reference"
            ])
            self.assertEqual(command[-2:], list(map(lambda path: str(path.resolve()), self.paths)))
            return subprocess.CompletedProcess(command, 0, "2 cases: failures=0", "")

        result = REFERENCE.run_validation(
            self.paths,
            ["301:399:0", "399:0:123.25"],
            reference,
            receipt,
            0.001,
            1e-7,
            FakeSpice(),
            runner=passing_rust,
            repo_root=Path(self.folder.name),
        )
        self.assertEqual(len(calls), 1)
        self.assertEqual(result["status"], "COMPARATOR_PASS_PROVENANCE_REVIEW_PENDING")
        self.assertFalse(result["scientific_accuracy_certified"])
        self.assertTrue(result["provenance_review_required"])
        self.assertEqual(result["case_count"], 2)
        self.assertEqual(result["rust_stdout"], "2 cases: failures=0")
        self.assertEqual(json.loads(receipt.read_text(encoding="utf-8")), result)
        self.assertEqual(
            result["reference_sha256"], hashlib.sha256(reference.read_bytes()).hexdigest()
        )
        self.assertEqual(result["ordered_kernels"][0]["name"], "first.bsp")
        REFERENCE.verify_reference_hashes(reference.read_text(encoding="utf-8"), self.paths)
        with self.assertRaisesRegex(ValueError, "refuse to overwrite"):
            REFERENCE.run_validation(
                self.paths, ["301:399:0"], reference, receipt, 0.001, 1e-7,
                FakeSpice(), runner=passing_rust,
            )

    def test_failed_rust_comparison_keeps_reference_but_never_success_receipt(self):
        reference = Path(self.folder.name) / "reference.csv"
        receipt = Path(self.folder.name) / "receipt.json"

        def failing_rust(command, **options):
            return subprocess.CompletedProcess(command, 1, "1 cases: failures=1", "exceeds limit")

        with self.assertRaisesRegex(ValueError, "Rust SPK comparator exited 1"):
            REFERENCE.run_validation(
                self.paths, ["301:399:0"], reference, receipt, 0.001, 1e-7,
                FakeSpice(), runner=failing_rust,
            )
        self.assertTrue(reference.exists())
        self.assertFalse(receipt.exists())

    def test_changed_kernel_while_rust_runs_invalidates_result(self):
        reference = Path(self.folder.name) / "reference.csv"
        receipt = Path(self.folder.name) / "receipt.json"

        def corrupt_kernel(command, **options):
            contents = bytearray(self.first.read_bytes())
            contents[0] ^= 1
            self.first.write_bytes(contents)
            return subprocess.CompletedProcess(command, 0, "1 cases: failures=0", "")

        with self.assertRaisesRegex(ValueError, "SHA256 mismatch"):
            REFERENCE.run_validation(
                self.paths, ["301:399:0"], reference, receipt, 0.001, 1e-7,
                FakeSpice(), runner=corrupt_kernel,
            )
        self.assertFalse(receipt.exists())

    def test_modified_reference_while_rust_runs_invalidates_result(self):
        reference = Path(self.folder.name) / "reference.csv"
        receipt = Path(self.folder.name) / "receipt.json"

        def corrupt_reference(command, **options):
            reference.write_text(reference.read_text(encoding="utf-8") + "# extra\n", encoding="utf-8")
            return subprocess.CompletedProcess(command, 0, "1 cases: failures=0", "")

        with self.assertRaisesRegex(ValueError, "CSV bytes changed"):
            REFERENCE.run_validation(
                self.paths, ["301:399:0"], reference, receipt, 0.001, 1e-7,
                FakeSpice(), runner=corrupt_reference,
            )
        self.assertFalse(receipt.exists())

    def test_one_command_requires_explicit_finite_positive_limits_and_new_outputs(self):
        for bad in ("0", "-1", "inf", "nan", "abc"):
            with self.assertRaises(ValueError):
                REFERENCE.approved_limit(bad, "position")
        self.assertEqual(REFERENCE.approved_limit("0.001", "position"), 0.001)
        reference = Path(self.folder.name) / "reference.csv"
        receipt = Path(self.folder.name) / "receipt.json"
        with self.assertRaisesRegex(ValueError, "max position"):
            REFERENCE.run_validation(
                self.paths, ["301:399:0"], reference, receipt, float("nan"), 1e-7,
                FakeSpice(),
            )
        with self.assertRaisesRegex(ValueError, "different paths"):
            REFERENCE.run_validation(
                self.paths, ["301:399:0"], reference, reference, 0.01, 1e-7,
                FakeSpice(),
            )

    def test_verify_command_works_without_spiceypy_and_fail_closed(self):
        document = self.reference()
        csv = Path(self.folder.name) / "naif.csv"
        csv.write_text(document, encoding="utf-8")
        command = [
            "verify", "--reference", str(csv),
            "--kernel", str(self.first), "--kernel", str(self.second)
        ]
        self.assertEqual(REFERENCE.main(command), 0)
        self.second.unlink()
        self.assertEqual(REFERENCE.main(command), 1)


if __name__ == "__main__":
    unittest.main()
