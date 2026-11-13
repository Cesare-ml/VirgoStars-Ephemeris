# EXHIBIT A. Common Public Attribution License Version 1.0.
# The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
# Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
# The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
# The Original Developer is the Initial Developer.
# The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
# Contributors retain applicable rights in their own contributions under the terms of this License.
# [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
# SPDX-License-Identifier: CPAL-1.0

"""Synthetic byte-level regression tests; NO NASA/JPL SPK data is bundled."""

import hashlib
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from verify_spk_kernel_hashes import CSV_HEADER, main, read_declared_hashes, verify


class SpkSha256PreflightTests(unittest.TestCase):
    def setUp(self):
        self.temp = TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.first = self.root / "first.bsp"
        self.second = self.root / "second.bsp"
        self.first.write_bytes(b"synthetic-kernel-A\x00")
        self.second.write_bytes(b"synthetic-kernel-B\x01")
        self.reference = self.root / "reference.csv"
        self.write_reference([
            hashlib.sha256(self.first.read_bytes()).hexdigest(),
            hashlib.sha256(self.second.read_bytes()).hexdigest(),
        ])

    def write_reference(self, digests):
        self.reference.write_text(
            "# provider=NAIF_SPICE\n"
            "# ordered_kernel_sha256=" + ";".join(digests) + "\n"
            + CSV_HEADER + "\n0,301,399,1,2,3,4,5,6\n",
            encoding="utf-8",
        )

    def test_success_preserves_load_order(self):
        self.assertEqual(len(verify(self.reference, [self.first, self.second])), 2)
        self.assertEqual(main([str(self.reference), str(self.first), str(self.second)]), 0)

    def test_swapped_kernel_order_fails(self):
        with self.assertRaisesRegex(ValueError, "kernel 1.*mismatch"):
            verify(self.reference, [self.second, self.first])

    def test_tampered_kernel_fails(self):
        self.first.write_bytes(self.first.read_bytes() + b"tampered")
        with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
            verify(self.reference, [self.first, self.second])

    def test_missing_kernel_and_manifest_count_mismatch_fail(self):
        with self.assertRaisesRegex(ValueError, "2 kernels but 1 paths"):
            verify(self.reference, [self.first])
        with self.assertRaises(FileNotFoundError):
            verify(self.reference, [self.first, self.root / "missing.bsp"])

    def test_invalid_hash_and_duplicate_declaration_fail(self):
        self.write_reference(["not-a-real-digest"])
        with self.assertRaisesRegex(ValueError, "64 hexadecimal"):
            read_declared_hashes(self.reference)
        self.write_reference(["a" * 64])
        with self.reference.open("a", encoding="utf-8") as stream:
            stream.write("# ordered_kernel_sha256=" + "b" * 64 + "\n")
        with self.assertRaisesRegex(ValueError, "metadata follows CSV header"):
            read_declared_hashes(self.reference)
        self.write_reference(["a" * 64])
        text = self.reference.read_text(encoding="utf-8")
        self.reference.write_text(text.replace(CSV_HEADER, "# ordered_kernel_sha256=" + "b" * 64 + "\n" + CSV_HEADER), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "duplicate ordered_kernel_sha256"):
            read_declared_hashes(self.reference)

    def test_missing_metadata_and_wrong_header_fail(self):
        self.reference.write_text(CSV_HEADER + "\n0,301,399,1,2,3,4,5,6\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "missing"):
            read_declared_hashes(self.reference)
        self.reference.write_text("# ordered_kernel_sha256=" + "a" * 64 + "\nwrong\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "unexpected CSV header"):
            read_declared_hashes(self.reference)

    def test_uppercase_hashes_are_accepted(self):
        self.write_reference([hashlib.sha256(self.first.read_bytes()).hexdigest().upper()])
        self.assertEqual(len(verify(self.reference, [self.first])), 1)


if __name__ == "__main__":
    unittest.main()
