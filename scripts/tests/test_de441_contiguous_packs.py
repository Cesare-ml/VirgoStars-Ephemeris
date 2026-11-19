# EXHIBIT A. Common Public Attribution License Version 1.0.
# The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
# Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
# The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
# The Original Developer is the Initial Developer.
# The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
# Contributors retain applicable rights in their own contributions under the terms of this License.
# [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
# SPDX-License-Identifier: CPAL-1.0
# Modification (2026-11-19): synthetic-only tests for the contiguous DE441
# export inventory; no third-party kernel material is used.

"""Offline catalog/manifest regressions. No real JPL comparison is performed."""

import hashlib
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import build_de441_contiguous_packs as catalog  # noqa: E402


def fake_classic(path: Path, start: float, end: float) -> None:
    record_bytes = 3000
    data = bytearray(record_bytes * 3)
    title = b"JPL Planetary Ephemeris DE441"
    data[:len(title)] = title
    struct.pack_into("<ddd", data, 2652, start, end, 32.0)
    struct.pack_into("<i", data, 2840, 441)
    struct.pack_into("<dd", data, record_bytes * 2, start, end)
    path.write_bytes(data)


class ContiguousCatalogTests(unittest.TestCase):
    def test_default_inventory_covers_desktop_and_all_optional_years(self):
        plan = catalog.build_plan(catalog.DEFAULT_MIN_YEAR, catalog.DEFAULT_MAX_YEAR)
        self.assertLessEqual(len(plan), 256)
        self.assertEqual(plan[0][1], -12999)
        self.assertEqual(plan[-1][2], 16999)
        self.assertEqual([row for row in plan if row[3]], [
            ("desktop-core-1000-2200", 1000, 2200, True)
        ])
        self.assertIn(("history-00800-00999", 800, 999, False), plan)
        self.assertIn(("future-02201-02400", 2201, 2400, False), plan)
        self.assertEqual(len({row[0] for row in plan}), len(plan))
        self.assertTrue(all(a[2] + 1 == b[1] for a, b in zip(plan, plan[1:])))

    def test_explicit_short_range_and_invalid_catalog_are_rejected(self):
        plan = catalog.build_plan(600, 2600)
        self.assertEqual(
            [(row[1], row[2]) for row in plan],
            [(600, 799), (800, 999), (1000, 2200), (2201, 2400), (2401, 2600)],
        )
        for bad in [(1001, 2300), (0, 2199), (-20001, 2200), (1000, 20001)]:
            with self.assertRaises(ValueError):
                catalog.build_plan(*bad)
        self.assertEqual(catalog.year_label(-1), "m00001")
        self.assertEqual(catalog.year_label(0), "00000")

    def test_build_base_reuses_rust_export_and_preserves_release_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            source = folder / "source.eph"
            destination = folder / "out"
            fake_classic(source, 1_000_000.0, 3_000_000.0)
            plan = catalog.build_plan(1000, 2200)
            pack_id = "desktop-core-1000-2200"

            def fake_run(command, **kwargs):
                self.assertEqual(command[1:4], ["run", "--offline", "--locked"])
                if "verify_de441_classic_slice" in command:
                    return type("Result", (), {
                        "returncode": 0,
                        "stdout": "parity_ok records=1 epochs=3 state_pairs=42 "
                                  "jd_tdb=[2305440, 2488440]",
                        "stderr": "",
                    })()
                artifact = Path(command[command.index("--") + 2])
                self.assertEqual(artifact.name, catalog.filename(pack_id))
                fake_classic(artifact, 2305440.0, 2488440.0)
                return type("Result", (), {
                    "returncode": 0,
                    "stdout": (
                        "requested_jd_tdb=[2305447.5, 2488434.5) "
                        "stored_jd_tdb=[2305440, 2488440] "
                        f"records=1 bytes={artifact.stat().st_size} output={artifact}"
                    ),
                    "stderr": "",
                })()

            with patch.object(catalog.subprocess, "run", side_effect=fake_run) as runs:
                path = catalog.build(
                    source, destination, plan, {pack_id},
                    hashlib.sha256(source.read_bytes()).hexdigest()
                )
            self.assertEqual(runs.call_count, 2)
            manifest = json.loads(path.read_text(encoding="utf-8"))
            self.assertEqual(manifest["schema"], catalog.SCHEMA)
            self.assertEqual(manifest["distribution_rights"], "review_pending")
            self.assertEqual(manifest["publication_status"],
                             "local_unsigned_not_authorized")
            self.assertEqual(manifest["packages"][0]["artifact"]["file"],
                             catalog.filename(pack_id))
            self.assertEqual(catalog.verify(path), 1)
            with self.assertRaises(FileExistsError):
                catalog.build(source, destination, plan, {pack_id}, None)

    def test_unbuilt_inventory_is_not_claimed_to_be_downloadable(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            plan = catalog.build_plan(800, 2400)
            manifest = {
                "schema": catalog.SCHEMA,
                "desktop_base_years_inclusive": [1000, 2200],
                "optional_block_years": 200,
                "catalog_first_year": 800,
                "catalog_last_year_inclusive": 2400,
                "packages": [
                    dict(id=pack_id, first_year=first, last_year_inclusive=last,
                         required=required, status="not_built")
                    for pack_id, first, last, required in plan
                ],
            }
            path = folder / catalog.MANIFEST_NAME
            path.write_text(json.dumps(manifest), encoding="utf-8")
            self.assertEqual(catalog.verify(path), 0)
            manifest["packages"][0]["status"] = "built"
            path.write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "artifact name"):
                catalog.verify(path)
            manifest["packages"][0]["status"] = "not_built"
            manifest["packages"][0]["artifact"] = {}
            path.write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "unbuilt package"):
                catalog.verify(path)

    def test_tampering_with_catalog_boundaries_is_detected(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            plan = catalog.build_plan(800, 2200)
            manifest = {
                "schema": catalog.SCHEMA,
                "desktop_base_years_inclusive": [1000, 2200],
                "optional_block_years": 200,
                "catalog_first_year": 800,
                "catalog_last_year_inclusive": 2200,
                "packages": [
                    dict(id=pack_id, first_year=first, last_year_inclusive=last,
                         required=required, status="not_built")
                    for pack_id, first, last, required in plan
                ],
            }
            manifest["packages"][1]["first_year"] = 1200
            path = folder / catalog.MANIFEST_NAME
            path.write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "out-of-order, altered"):
                catalog.verify(path)


if __name__ == "__main__":
    unittest.main()
