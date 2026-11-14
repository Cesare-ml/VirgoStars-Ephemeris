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

"""Read-only integration-order audit for the experimental Rust SPK PR stack.

Modification 2026-11-14: fail-closed dependency review independent of
synthetic CI success. Never merges, approves or downloads ephemeris kernels.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path

REPOSITORY = "Cesare-ml/VirgoStars-Ephemeris"
INTEGRATION_BRANCH = "dev/ephemeris"
# Direct prerequisites (transitive prerequisites are implied by the chain).
SPK_PR_CHAIN = (
    (148, (), "DAF/SPK directory index"),
    (150, (148,), "Type 2/3 direct state interpolation"),
    (157, (150,), "Single-kernel target-center chains"),
    (159, (157,), "Multi-kernel precedence and center chains"),
    (160, (159,), "Independent vector comparison contract"),
    (162, (160,), "Independent NAIF/SPICE vector generation"),
    (164, (162,), "One-command comparison evidence receipt"),
)
# Independent breadth gate, deliberately not a prerequisite for merging #148.
BREADTH_AUDIT = (165, (), "Sun/Moon/Earth/planet reference matrix audit")
MANIFEST = SPK_PR_CHAIN + (BREADTH_AUDIT,)
PR_NUMBERS = tuple(item[0] for item in MANIFEST)


class AuditInputError(ValueError):
    """The input is incomplete, malformed, or not a trustworthy PR status."""


def validate_snapshot(snapshot: object) -> dict[int, dict]:
    """Validate actual GitHub pull metadata; reject missing/partial state."""
    if not isinstance(snapshot, dict):
        raise AuditInputError("expected an object keyed by PR number")
    result: dict[int, dict] = {}
    for number in PR_NUMBERS:
        raw = snapshot.get(str(number))
        if not isinstance(raw, dict) or raw.get("number") != number:
            raise AuditInputError(f"missing or mismatched PR #{number}")
        state = raw.get("state")
        merged = raw.get("merged")
        draft = raw.get("draft")
        base = raw.get("base")
        if state not in ("open", "closed") or type(merged) is not bool or type(draft) is not bool:
            raise AuditInputError(f"PR #{number}: invalid state/merged/draft")
        if not isinstance(base, dict) or not isinstance(base.get("ref"), str):
            raise AuditInputError(f"PR #{number}: missing base branch")
        if merged and state != "closed":
            raise AuditInputError(f"PR #{number}: merged PR is not closed")
        result[number] = raw
    return result


def audit(snapshot: object) -> list[dict]:
    """Report dependency order only: NEVER claim CI or scientific accuracy."""
    records = validate_snapshot(snapshot)
    output = []
    for number, dependencies, description in MANIFEST:
        record = records[number]
        unmerged = [parent for parent in dependencies if not records[parent]["merged"]]
        if record["merged"] and unmerged:
            status = "ORDER_CONFLICT"
        elif record["merged"]:
            status = "MERGED"
        elif record["state"] == "closed":
            status = "CLOSED_UNMERGED"
        elif record["base"]["ref"] != INTEGRATION_BRANCH:
            status = "BASE_REVIEW"
        elif unmerged:
            status = "BLOCKED"
        elif record["draft"]:
            status = "DRAFT_REVIEW"
        else:
            status = "HUMAN_REVIEW_REQUIRED"
        output.append({
            "number": number,
            "description": description,
            "status": status,
            "unmerged_prerequisites": unmerged,
            "draft": record["draft"],
            "base": record["base"]["ref"],
            "url": f"https://github.com/{REPOSITORY}/pull/{number}",
        })
    return output


def fetch_live_snapshot() -> dict:
    """Read public GitHub metadata; use optional token only for rate limits."""
    result = {}
    for number in PR_NUMBERS:
        url = f"https://api.github.com/repos/{REPOSITORY}/pulls/{number}"
        headers = {
            "Accept": "application/vnd.github+json",
            "User-Agent": "VirgoStars-Ephemeris-read-only-PR-audit",
            "X-GitHub-Api-Version": "2022-11-28",
        }
        token = os.environ.get("GITHUB_TOKEN")
        if token:
            headers["Authorization"] = f"Bearer {token}"
        request = urllib.request.Request(url, headers=headers)
        try:
            with urllib.request.urlopen(request, timeout=15) as response:
                data = json.load(response)
        except (OSError, ValueError, urllib.error.HTTPError) as error:
            raise AuditInputError(f"cannot inspect GitHub PR #{number}: {error}") from error
        result[str(number)] = data
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--live", action="store_true", help="read-only GitHub REST lookup")
    source.add_argument("--snapshot", type=Path, help="offline PR-number-keyed GitHub response JSON")
    parser.add_argument("--json", action="store_true", help="emit machine-readable report")
    parser.add_argument(
        "--require-integrated", action="store_true",
        help="exit 1 until all listed PRs are merged in dependency order",
    )
    args = parser.parse_args(argv)
    try:
        snapshot = fetch_live_snapshot() if args.live else json.loads(args.snapshot.read_text())
        report = audit(snapshot)
    except (AuditInputError, OSError, ValueError) as error:
        print(f"PR dependency audit input error: {error}", file=sys.stderr)
        return 2
    if args.json:
        print(json.dumps({
            "repository": REPOSITORY,
            "integration_branch": INTEGRATION_BRANCH,
            "scientific_accuracy_certified": False,
            "merge_authorized": False,
            "pull_requests": report,
        }, indent=2))
    else:
        for item in report:
            missing = ",".join(f"#{n}" for n in item["unmerged_prerequisites"])
            print(f'#{item["number"]}: {item["status"]}; prerequisites={missing or "-"}; '
                  f'{item["description"]}')
        print("NOTE: dependency audit only; CI, human review and real JPL/NAIF validation are separate.")
    if any(item["status"] == "ORDER_CONFLICT" for item in report):
        return 1
    if args.require_integrated and any(item["status"] != "MERGED" for item in report):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
