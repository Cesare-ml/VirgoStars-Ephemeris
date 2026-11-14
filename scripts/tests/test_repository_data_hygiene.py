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

"""Offline test cases for the tracked kernel-data distribution preflight."""

import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "check_repository_data_hygiene.py"
SPEC = importlib.util.spec_from_file_location("check_repository_data_hygiene", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class RepositoryDataHygieneTests(unittest.TestCase):
    def test_blocked_extensions_are_case_insensitive_and_match_archives(self):
        for name in [
            "data/de441.eph", "kernels/DE440S.BSP", "x/foo.se1",
            "pck/site.BPC", "frames/model.tf", "leap.tls.gz",
            "nested/planet.BSP.XZ", "earth.tpc.zst", "sclk.tsc.zip",
        ]:
            with self.subTest(name=name):
                self.assertTrue(checker.is_kernel_path(name))
        for name in ["src/engine.rs", "data/reference.csv", "docs/de441.eph.md", "astrometry.bsp.txt"]:
            with self.subTest(name=name):
                self.assertFalse(checker.is_kernel_path(name))

    def test_detects_disguised_kernel_payloads_and_allows_text(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = root / "opaque.dat"
            data.write_bytes(b"DAF/SPK " + b"\0" * 32)
            self.assertIn("DAF/SPK", checker.inspect_path(root, data.name))
            data.write_bytes(b"JPL Planetary Ephemeris DE441" + b"\0" * 32)
            self.assertIn("JPL", checker.inspect_path(root, data.name))
            data.write_text("synthetic regression data, no external ephemeris")
            self.assertIsNone(checker.inspect_path(root, data.name))

    def test_git_index_detects_forced_add_of_ignored_kernel(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            (root / ".gitignore").write_text("*.bsp\n")
            (root / "readme.txt").write_text("original research\n")
            (root / "venus.bsp").write_bytes(b"DAF/SPK " + b"\0" * 10)
            subprocess.run(["git", "add", ".gitignore", "readme.txt"], cwd=root, check=True)
            self.assertEqual(checker.check_repository(root), [])
            subprocess.run(["git", "add", "-f", "venus.bsp"], cwd=root, check=True)
            self.assertEqual(
                checker.check_repository(root),
                [("venus.bsp", "external kernel/ephemeris filename")],
            )

    def test_untracked_external_file_does_not_fail_distribution_check(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            (root / "de441.eph").write_bytes(b"JPL Planetary Ephemeris DE441")
            self.assertEqual(checker.check_repository(root), [])


    def test_indexed_kernel_magic_cannot_be_hidden_by_worktree_replacement(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            disguised = root / "opaque.dat"
            disguised.write_bytes(b"DAF/SPK " + b"\0" * 80)
            subprocess.run(["git", "add", "opaque.dat"], cwd=root, check=True)
            # Git will commit the staged kernel, not these later worktree bytes.
            disguised.write_text("harmless working-tree replacement")
            self.assertEqual(
                checker.check_repository(root),
                [("opaque.dat", "NAIF DAF/SPK magic header")],
            )
            disguised.unlink()
            self.assertEqual(
                checker.check_repository(root),
                [("opaque.dat", "NAIF DAF/SPK magic header")],
            )

    def test_unstaged_kernel_magic_cannot_mask_clean_index(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            path = root / "benign.dat"
            path.write_text("staged project documentation")
            subprocess.run(["git", "add", "benign.dat"], cwd=root, check=True)
            path.write_bytes(b"JPL Planetary Ephemeris DE441" + b"\0" * 60)
            self.assertEqual(checker.check_repository(root), [])
            # Once staged, the byte-level guard must reject it.
            subprocess.run(["git", "add", "benign.dat"], cwd=root, check=True)
            self.assertEqual(
                checker.check_repository(root),
                [("benign.dat", "classic JPL planetary ephemeris magic header")],
            )

    def test_unmerged_index_fails_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            oid = subprocess.run(
                ["git", "hash-object", "-w", "--stdin"],
                cwd=root, input=b"synthetic conflict", stdout=subprocess.PIPE, check=True,
            ).stdout.decode("ascii").strip()
            subprocess.run(
                ["git", "update-index", "--index-info"],
                cwd=root,
                input=f"100644 {oid} 1\tconflicted.dat\n".encode(),
                check=True,
            )
            with self.assertRaisesRegex(ValueError, "unmerged Git index"):
                checker.check_repository(root)


if __name__ == "__main__":
    unittest.main()
