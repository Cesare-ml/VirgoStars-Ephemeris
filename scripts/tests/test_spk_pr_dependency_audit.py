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

"""Offline regression tests for the read-only SPK PR dependency audit."""
import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

from scripts import audit_spk_pr_dependencies as tool


def snapshot(merged=(), drafts=(), closed_unmerged=(), base_overrides=None):
    merged = set(merged)
    drafts = set(drafts)
    closed_unmerged = set(closed_unmerged)
    base_overrides = base_overrides or {}
    return {
        str(number): {
            "number": number,
            "state": "closed" if number in merged or number in closed_unmerged else "open",
            "merged": number in merged,
            "draft": number in drafts,
            "base": {"ref": base_overrides.get(number, tool.INTEGRATION_BRANCH)},
        }
        for number in tool.PR_NUMBERS
    }


def statuses(records):
    return {row["number"]: row for row in tool.audit(records)}


class DependencyAuditTests(unittest.TestCase):
    def test_initial_chain_only_exposes_first_pr_for_human_review(self):
        records = statuses(snapshot())
        self.assertEqual(records[148]["status"], "HUMAN_REVIEW_REQUIRED")
        self.assertEqual(records[150]["status"], "BLOCKED")
        self.assertEqual(records[164]["status"], "BLOCKED")
        self.assertEqual(records[165]["status"], "HUMAN_REVIEW_REQUIRED")
        self.assertEqual(records[164]["unmerged_prerequisites"], [162])

    def test_merging_first_pr_unlocks_only_its_direct_successor(self):
        records = statuses(snapshot(merged=(148,)))
        self.assertEqual(records[148]["status"], "MERGED")
        self.assertEqual(records[150]["status"], "HUMAN_REVIEW_REQUIRED")
        self.assertEqual(records[157]["status"], "BLOCKED")

    def test_draft_requires_review_even_with_merged_dependencies(self):
        records = statuses(snapshot(merged=(148,), drafts=(150,)))
        self.assertEqual(records[150]["status"], "DRAFT_REVIEW")

    def test_dependency_order_conflict_does_not_get_merged_label(self):
        records = statuses(snapshot(merged=(150,)))
        self.assertEqual(records[150]["status"], "ORDER_CONFLICT")
        self.assertEqual(records[150]["unmerged_prerequisites"], [148])

    def test_closed_unmerged_pr_is_not_actionable(self):
        records = statuses(snapshot(closed_unmerged=(148,)))
        self.assertEqual(records[148]["status"], "CLOSED_UNMERGED")
        self.assertEqual(records[150]["status"], "BLOCKED")

    def test_wrong_integration_branch_requires_explicit_review(self):
        records = statuses(snapshot(base_overrides={148: "main"}))
        self.assertEqual(records[148]["status"], "BASE_REVIEW")

    def test_missing_or_malformed_metadata_fails_closed(self):
        values = snapshot()
        del values["150"]
        with self.assertRaisesRegex(tool.AuditInputError, "#150"):
            tool.audit(values)
        values = snapshot()
        values["150"]["merged"] = "false"
        with self.assertRaisesRegex(tool.AuditInputError, "#150"):
            tool.audit(values)
        values = snapshot()
        values["150"]["merged"] = True
        with self.assertRaisesRegex(tool.AuditInputError, "#150"):
            tool.audit(values)
        values = snapshot()
        values["150"]["base"] = {}
        with self.assertRaisesRegex(tool.AuditInputError, "#150"):
            tool.audit(values)

    def test_integrated_mode_requires_the_independent_breadth_audit(self):
        all_except_breadth = tuple(n for n in tool.PR_NUMBERS if n != 165)
        with tempfile.TemporaryDirectory() as directory:
            filename = Path(directory) / "prs.json"
            filename.write_text(json.dumps(snapshot(merged=all_except_breadth)), encoding="utf-8")
            with contextlib.redirect_stdout(io.StringIO()):
                code = tool.main(["--snapshot", str(filename), "--require-integrated"])
            self.assertEqual(code, 1)
            filename.write_text(json.dumps(snapshot(merged=tool.PR_NUMBERS)), encoding="utf-8")
            with contextlib.redirect_stdout(io.StringIO()):
                code = tool.main(["--snapshot", str(filename), "--require-integrated"])
            self.assertEqual(code, 0)

    def test_json_output_never_claims_scientific_accuracy_or_merge_permission(self):
        with tempfile.TemporaryDirectory() as directory:
            filename = Path(directory) / "prs.json"
            filename.write_text(json.dumps(snapshot()), encoding="utf-8")
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                code = tool.main(["--snapshot", str(filename), "--json"])
            self.assertEqual(code, 0)
            report = json.loads(output.getvalue())
            self.assertIs(report["scientific_accuracy_certified"], False)
            self.assertIs(report["merge_authorized"], False)
            self.assertEqual(len(report["pull_requests"]), len(tool.PR_NUMBERS))

    def test_corrupted_offline_snapshot_returns_input_error(self):
        with tempfile.TemporaryDirectory() as directory:
            filename = Path(directory) / "prs.json"
            filename.write_text("{invalid", encoding="utf-8")
            with contextlib.redirect_stderr(io.StringIO()):
                code = tool.main(["--snapshot", str(filename)])
            self.assertEqual(code, 2)

    def test_out_of_order_merge_sets_nonzero_exit_even_without_strict_flag(self):
        with tempfile.TemporaryDirectory() as directory:
            filename = Path(directory) / "prs.json"
            filename.write_text(json.dumps(snapshot(merged=(150,))), encoding="utf-8")
            with contextlib.redirect_stdout(io.StringIO()):
                code = tool.main(["--snapshot", str(filename)])
            self.assertEqual(code, 1)


    def test_out_of_order_merged_parent_blocks_unmerged_descendants(self):
        records = statuses(snapshot(merged=(150,)))
        self.assertEqual(records[150]["status"], "ORDER_CONFLICT")
        # Prior implementation checked only parent raw["merged"] and would
        # incorrectly announce that #157 was ready for human review.
        self.assertEqual(records[157]["status"], "BLOCKED")
        self.assertEqual(records[157]["blocking_prerequisites"], [150])
        self.assertEqual(records[157]["unmerged_prerequisites"], [])
        self.assertEqual(records[159]["status"], "BLOCKED")

    def test_bad_base_on_merged_parent_blocks_downstream_and_exits_nonzero(self):
        bad = snapshot(
            merged=(148, 150),
            base_overrides={150: "main"},
        )
        records = statuses(bad)
        self.assertEqual(records[150]["status"], "BASE_REVIEW")
        self.assertEqual(records[157]["status"], "BLOCKED")
        self.assertEqual(records[157]["blocking_prerequisites"], [150])
        with tempfile.TemporaryDirectory() as directory:
            filename = Path(directory) / "prs.json"
            filename.write_text(json.dumps(bad), encoding="utf-8")
            with contextlib.redirect_stdout(io.StringIO()):
                exit_code = tool.main(["--snapshot", str(filename)])
            self.assertEqual(exit_code, 1)

    def test_transitively_invalid_merged_descendant_is_order_conflict(self):
        records = statuses(snapshot(merged=(150, 157)))
        self.assertEqual(records[150]["status"], "ORDER_CONFLICT")
        self.assertEqual(records[157]["status"], "ORDER_CONFLICT")
        self.assertEqual(records[159]["status"], "BLOCKED")

    def test_first_review_stage_follows_actual_integrated_chain(self):
        first = tool.next_review_stage(tool.audit(snapshot()))
        self.assertEqual(first["number"], 148)
        second = tool.next_review_stage(tool.audit(snapshot(merged=(148,))))
        self.assertEqual(second["number"], 150)
        # The four canonical Rust implementation PRs are already integrated;
        # comparator #160 becomes the first pending stage, not #165.
        first_validator = tool.next_review_stage(
            tool.audit(snapshot(merged=(148, 150, 157, 159), drafts=(160,)))
        )
        self.assertEqual(first_validator["number"], 160)
        self.assertEqual(first_validator["status"], "DRAFT_REVIEW")
        all_spk = tuple(n for n in tool.PR_NUMBERS if n != 165)
        breadth = tool.next_review_stage(tool.audit(snapshot(merged=all_spk)))
        self.assertEqual(breadth["number"], 165)

    def test_next_review_stage_none_when_all_integrated(self):
        report = tool.audit(snapshot(merged=tool.PR_NUMBERS))
        self.assertIsNone(tool.next_review_stage(report))

    def test_json_next_stage_preserves_no_merge_permission(self):
        with tempfile.TemporaryDirectory() as directory:
            filename = Path(directory) / "prs.json"
            filename.write_text(
                json.dumps(snapshot(merged=(148, 150, 157, 159), drafts=(160,))),
                encoding="utf-8",
            )
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                exit_code = tool.main(["--snapshot", str(filename), "--json"])
            self.assertEqual(exit_code, 0)
            report = json.loads(output.getvalue())
            self.assertEqual(report["next_review_stage"]["number"], 160)
            self.assertEqual(report["next_review_stage"]["status"], "DRAFT_REVIEW")
            self.assertIs(report["merge_authorized"], False)
            self.assertIs(report["scientific_accuracy_certified"], False)


if __name__ == "__main__":
    unittest.main()
