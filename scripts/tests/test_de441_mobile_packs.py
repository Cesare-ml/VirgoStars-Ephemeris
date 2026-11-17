# EXHIBIT A. Common Public Attribution License Version 1.0.
# The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
# Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
# The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
# The Original Developer is the Initial Developer.
# The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
# Contributors retain applicable rights in their own contributions under the terms of this License.
# [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
# SPDX-License-Identifier: CPAL-1.0
# Modification (2026-11-18): synthetic pack-manifest regression tests.

"""Synthetic-only regression tests for offline DE441 mobile packaging.

Modification 2026-11-18: original VirgoStars tests under CPAL-1.0.
The separately licensed JPL DE441 file is not used or redistributed here.
"""

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
import build_de441_mobile_packs as mobile  # noqa: E402


def synthetic_classic(path: Path, start: float, end: float, records: int) -> None:
    record_size = 3000
    data = bytearray((records + 2) * record_size)
    title = b"JPL Planetary Ephemeris DE441"
    data[:len(title)] = title
    struct.pack_into("<dddi", data, 2652, start, end, 32.0, 0)
    struct.pack_into("<i", data, 2840, 441)
    for index in range(records):
        record_start = start + index * (end - start) / records
        record_end = start + (index + 1) * (end - start) / records
        struct.pack_into("<dd", data, (index + 2) * record_size,
                         record_start, record_end)
    path.write_bytes(data)


class MobilePackTests(unittest.TestCase):
    def test_fixed_ranges_are_complete_without_gaps(self):
        self.assertEqual(len(mobile.PACKS), 10)
        self.assertEqual(mobile.PACK_BY_ID["core"], ("core", 1600, 2100, True))
        self.assertEqual(mobile.PACK_BY_ID["future-2101-2300"][-3:], (2101, 2300, False))
        historical = [p for p in mobile.PACKS if p[0].startswith("history")]
        self.assertEqual([(p[1], p[2]) for p in historical],
                         [(y, y + 199) for y in range(0, 1600, 200)])

    def test_summary_requires_expected_destination(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "core.eph"
            output = (
                "requested_jd_tdb=[2305447.5, 2488434.5) "
                "stored_jd_tdb=[2305440, 2488440] "
                f"records=1 bytes=9000 output={path}"
            )
            self.assertEqual(mobile.parse_export_summary(output, path)["bytes"], 9000)
            with self.assertRaisesRegex(ValueError, "different output path"):
                mobile.parse_export_summary(output, Path(directory) / "wrong.eph")
            with self.assertRaisesRegex(ValueError, "no recognized summary"):
                mobile.parse_export_summary("arbitrary output", path)

    def test_artifact_hash_and_records_are_verified(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            output = folder / "de441-core.eph"
            synthetic_classic(output, 2305440.0, 2488440.0, 1)
            artifact = {
                "file": output.name,
                "sha256": mobile.sha256_file(output),
                "bytes": output.stat().st_size,
                "stored_start_jd_tdb": 2305440.0,
                "stored_end_jd_tdb": 2488440.0,
                "requested_start_jd_tdb": 2305447.5,
                "requested_end_exclusive_jd_tdb": 2488434.5,
                "coefficient_records": 1,
            }
            mobile.verify_artifact(folder, artifact)
            output.write_bytes(output.read_bytes()[:-1] + b"x")
            with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
                mobile.verify_artifact(folder, artifact)
            artifact["sha256"] = mobile.sha256_file(output)
            mobile.verify_artifact(folder, artifact)
            artifact["coefficient_records"] = 2
            with self.assertRaisesRegex(ValueError, "coefficient record"):
                mobile.verify_artifact(folder, artifact)

    def test_rejects_path_escape_and_manifest_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            with self.assertRaisesRegex(ValueError, "simple basename"):
                mobile.verify_artifact(folder, {"file": "../other.eph"})
            document = {"schema": mobile.SCHEMA, "packages": []}
            first = mobile.create_manifest_exclusive(folder, document)
            original = first.read_bytes()
            with self.assertRaises(FileExistsError):
                mobile.create_manifest_exclusive(folder, {"different": True})
            self.assertEqual(first.read_bytes(), original)

    def test_build_core_with_mocked_rust_export(self):
        # This mocks the exporter; it does not establish DE441 numerical parity.
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            source = folder / "de441-source.eph"
            destination = folder / "out"
            synthetic_classic(source, 1_000_000.0, 3_000_000.0, 1)

            def fake_run(command, **kwargs):
                self.assertEqual(command[1:4], ["run", "--offline", "--locked"])
                output = Path(command[command.index("--") + 2])
                synthetic_classic(output, 2305440.0, 2488440.0, 1)
                text = (
                    "requested_jd_tdb=[2305447.5, 2488434.5) "
                    "stored_jd_tdb=[2305440, 2488440] "
                    f"records=1 bytes={output.stat().st_size} output={output}"
                )
                return type("Completed", (), {"returncode": 0, "stdout": text, "stderr": ""})()

            with patch.object(mobile.subprocess, "run", side_effect=fake_run) as invoked:
                manifest = mobile.build(source, destination, {"core"},
                                        hashlib.sha256(source.read_bytes()).hexdigest(), "cargo")
            self.assertEqual(invoked.call_count, 1)
            document = json.loads(manifest.read_text(encoding="utf-8"))
            self.assertEqual(document["source"]["sha256"], mobile.sha256_file(source))
            self.assertEqual(document["distribution_rights"], "review_pending")
            self.assertEqual([p["status"] for p in document["packages"]].count("built"), 1)
            self.assertEqual(mobile.verify(manifest), 1)
            with self.assertRaises(FileExistsError):
                mobile.build(source, destination, {"core"}, None, "cargo")


if __name__ == "__main__":
    unittest.main()
