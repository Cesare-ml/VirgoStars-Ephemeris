# EXHIBIT A. Common Public Attribution License Version 1.0.
# The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
# Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
# The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
# The Original Developer is the Initial Developer.
# The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
# Contributors retain applicable rights in their own contributions under the terms of this License.
# [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
# SPDX-License-Identifier: CPAL-1.0
# Modification (2026-11-14): test ordered names/sizes and hash-only compatibility.
# Modification (2026-11-14): regressions for kernel swaps/edits during SHA-256.

"""Synthetic byte-level regression tests; NO NASA/JPL SPK data is bundled."""

import hashlib
import os
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
from tempfile import TemporaryDirectory
import unittest

from verify_spk_kernel_hashes import CSV_HEADER, main, read_declared_hashes, read_declared_manifest, sha256_file, verify


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

    def add_manifest(self, names=None, sizes=None):
        """Insert optional ordered kernel declarations before the CSV header."""
        content = self.reference.read_text(encoding="utf-8")
        fields = []
        if names is not None:
            fields.append("# ordered_kernel_names=" + ";".join(names))
        if sizes is not None:
            fields.append("# ordered_kernel_sizes_bytes=" + ";".join(map(str, sizes)))
        self.reference.write_text(
            content.replace(CSV_HEADER, "\n".join(fields) + "\n" + CSV_HEADER),
            encoding="utf-8",
        )

    def test_full_manifest_validates_names_sizes_and_hashes(self):
        self.add_manifest(
            names=["first.bsp", "second.bsp"],
            sizes=[self.first.stat().st_size, self.second.stat().st_size],
        )
        hashes, names, sizes = read_declared_manifest(self.reference)
        self.assertEqual(len(hashes), 2)
        self.assertEqual(names, ["first.bsp", "second.bsp"])
        self.assertEqual(sizes, [self.first.stat().st_size, self.second.stat().st_size])
        self.assertEqual(len(verify(self.reference, [self.first, self.second])), 2)
        self.assertEqual(main([str(self.reference), str(self.first), str(self.second)]), 0)

    def test_identical_bytes_swapped_files_rejected_by_names(self):
        # Hash checks alone cannot detect reversing two same-byte kernel files.
        self.first.write_bytes(b"identical synthetic bytes")
        self.second.write_bytes(b"identical synthetic bytes")
        digest = hashlib.sha256(self.first.read_bytes()).hexdigest()
        self.write_reference([digest, digest])
        self.assertEqual(len(verify(self.reference, [self.second, self.first])), 2)
        self.add_manifest(
            names=["first.bsp", "second.bsp"],
            sizes=[self.first.stat().st_size, self.second.stat().st_size],
        )
        with self.assertRaisesRegex(ValueError, "filename/load-order mismatch"):
            verify(self.reference, [self.second, self.first])
        self.assertEqual(main([str(self.reference), str(self.second), str(self.first)]), 1)

    def test_full_manifest_rejects_wrong_size_even_if_sha_is_correct(self):
        self.add_manifest(
            names=["first.bsp", "second.bsp"],
            sizes=[self.first.stat().st_size + 1, self.second.stat().st_size],
        )
        with self.assertRaisesRegex(ValueError, "byte size mismatch"):
            verify(self.reference, [self.first, self.second])

    def test_partial_duplicate_or_invalid_manifests_fail_closed(self):
        actual_names = ["first.bsp", "second.bsp"]
        actual_sizes = [self.first.stat().st_size, self.second.stat().st_size]
        self.add_manifest(names=actual_names)
        with self.assertRaisesRegex(ValueError, "declared together"):
            read_declared_manifest(self.reference)
        self.write_reference([
            hashlib.sha256(self.first.read_bytes()).hexdigest(),
            hashlib.sha256(self.second.read_bytes()).hexdigest(),
        ])
        self.add_manifest(sizes=actual_sizes)
        with self.assertRaisesRegex(ValueError, "declared together"):
            read_declared_manifest(self.reference)
        for names, sizes, expected in (
            (["first.bsp"], actual_sizes, "counts differ"),
            (actual_names, [actual_sizes[0]], "counts differ"),
            (["../first.bsp", "second.bsp"], actual_sizes, "plain, nonempty basenames"),
            (["first\\hidden.bsp", "second.bsp"], actual_sizes, "plain, nonempty basenames"),
            (["", "second.bsp"], actual_sizes, "plain, nonempty basenames"),
            (actual_names, ["01", str(actual_sizes[1])], "canonical decimal"),
            (actual_names, ["-1", str(actual_sizes[1])], "canonical decimal"),
            (actual_names, ["1.0", str(actual_sizes[1])], "canonical decimal"),
            (actual_names, ["9" * 100, str(actual_sizes[1])], "canonical decimal"),
        ):
            with self.subTest(names=names, sizes=sizes):
                self.write_reference([
                    hashlib.sha256(self.first.read_bytes()).hexdigest(),
                    hashlib.sha256(self.second.read_bytes()).hexdigest(),
                ])
                self.add_manifest(names=names, sizes=sizes)
                with self.assertRaisesRegex(ValueError, expected):
                    read_declared_manifest(self.reference)
        self.write_reference([
            hashlib.sha256(self.first.read_bytes()).hexdigest(),
            hashlib.sha256(self.second.read_bytes()).hexdigest(),
        ])
        self.add_manifest(names=actual_names, sizes=actual_sizes)
        content = self.reference.read_text(encoding="utf-8")
        self.reference.write_text(
            content.replace(CSV_HEADER, "# ordered_kernel_names=first.bsp;second.bsp\n" + CSV_HEADER),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ValueError, "duplicate ordered_kernel_names"):
            read_declared_manifest(self.reference)

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

    def test_rejects_same_size_in_flight_file_edit_without_a_false_sha_pass(self):
        """Simulate a metadata change between two descriptor observations."""
        real_fstat = os.fstat
        calls = 0

        def changed_second_fstat(fd):
            nonlocal calls
            calls += 1
            result = real_fstat(fd)
            if calls != 2:
                return result
            return SimpleNamespace(
                st_dev=result.st_dev,
                st_ino=result.st_ino,
                st_size=result.st_size,  # Even unchanged byte length is unsafe.
                st_mtime_ns=result.st_mtime_ns + 1,
                st_ctime_ns=result.st_ctime_ns,
            )

        with patch("verify_spk_kernel_hashes.os.fstat", side_effect=changed_second_fstat):
            with self.assertRaisesRegex(ValueError, "changed during SHA-256 read"):
                verify(self.reference, [self.first, self.second])
        self.assertEqual(calls, 2)

    def test_ctime_disagreement_alone_does_not_reject_unchanged_kernel(self):
        """Windows can report different creation-time fields for path and FD."""
        actual_fstat = os.fstat

        def different_ctime(fd):
            result = actual_fstat(fd)
            return SimpleNamespace(
                st_dev=result.st_dev,
                st_ino=result.st_ino,
                st_size=result.st_size,
                st_mtime_ns=result.st_mtime_ns,
                st_ctime_ns=result.st_ctime_ns + 1_000_000_000,
            )

        with patch("verify_spk_kernel_hashes.os.fstat", side_effect=different_ctime):
            self.assertEqual(
                sha256_file(self.first),
                hashlib.sha256(self.first.read_bytes()).hexdigest(),
            )

    def test_rejects_path_identity_change_between_open_and_hash(self):
        """The opened inode must still be the one inspected by path."""
        original_fstat = os.fstat

        def changed_inode(fd):
            result = original_fstat(fd)
            return SimpleNamespace(
                st_dev=result.st_dev,
                st_ino=result.st_ino + 1,
                st_size=result.st_size,
                st_mtime_ns=result.st_mtime_ns,
                st_ctime_ns=result.st_ctime_ns,
            )

        with patch("verify_spk_kernel_hashes.os.fstat", side_effect=changed_inode):
            with self.assertRaisesRegex(ValueError, "path changed before SHA-256 read"):
                sha256_file(self.first)

    def test_rejects_path_swap_after_stream_before_returning_hash(self):
        """The name must still reference the stream that supplied its bytes."""
        actual_stat = Path.stat
        calls = 0

        def changed_final_path_stat(path, *args, **kwargs):
            nonlocal calls
            result = actual_stat(path, *args, **kwargs)
            if path != self.first:
                return result
            calls += 1
            if calls != 2:
                return result
            return SimpleNamespace(
                st_dev=result.st_dev,
                st_ino=result.st_ino + 1,
                st_size=result.st_size,
                st_mtime_ns=result.st_mtime_ns,
                st_ctime_ns=result.st_ctime_ns,
            )

        with patch.object(Path, "stat", changed_final_path_stat):
            with self.assertRaisesRegex(ValueError, "path changed during SHA-256 read"):
                sha256_file(self.first)
        self.assertEqual(calls, 2)

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
