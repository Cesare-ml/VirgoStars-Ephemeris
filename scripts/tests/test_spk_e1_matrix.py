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
