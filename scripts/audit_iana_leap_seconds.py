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

"""Offline integrity and history audit for official IANA/NIST leap-seconds.list.

This is a stdlib-only *data-maintenance* tool, not an astronomical engine.
It never fetches a URL, changes repository files or changes Rust time scales.
Future updates require independent source review and a new reviewed PR.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from datetime import date, datetime, timedelta, timezone
import hashlib
from pathlib import Path
import re
import sys


NTP_EPOCH = datetime(1900, 1, 1, tzinfo=timezone.utc)
BASE_NTP = 2_272_060_800  # 1972-01-01 UTC
BASE_OFFSET = 10
CURRENT_PIN = (
    Path(__file__).resolve().parents[1]
    / "crates"
    / "ephemeris-core"
    / "data"
    / "iana-tzdb-2026b-leap-seconds.list"
)
MAX_BYTES = 1024 * 1024
SHA1_GROUP = re.compile(r"^[0-9a-fA-F]{1,8}$")
DECIMAL = re.compile(r"^[0-9]+$")


class LeapAuditError(ValueError):
    """An input fails the provenance, integrity or historical-data gate."""


@dataclass(frozen=True)
class LeapTable:
    updated_ntp: int | None
    expires_ntp: int
    entries: tuple[tuple[int, int], ...]
    sha1: str | None

    @property
    def expiry_utc(self) -> datetime:
        return from_ntp(self.expires_ntp)


def from_ntp(seconds: int) -> datetime:
    try:
        return NTP_EPOCH + timedelta(seconds=seconds)
    except (OverflowError, ValueError) as error:
        raise LeapAuditError("NTP timestamp outside supported civil date range") from error


def _decimal(text: str, description: str) -> int:
    if not DECIMAL.fullmatch(text):
        raise LeapAuditError(f"{description} is not a nonnegative decimal integer")
    return int(text)


def parse_table(data: str, *, official: bool) -> LeapTable:
    """Parse IANA/NIST NTP leaps; require #$/#@/#h and verify official SHA-1.

    The upstream #h convention hashes the concatenated *decimal tokens*
    from #$ and #@ and each numeric offset record, in file order, ignoring
    whitespace and comments. Its five hex words are padded to eight digits.
    An SHA-1 match is an integrity check, NOT authentication of the source.
    """
    updated = expiry = digest = None
    records: list[tuple[int, int]] = []
    hashed_parts: list[str] = []
    for line_number, raw in enumerate(data.splitlines(), 1):
        line = raw.strip()
        if not line:
            continue
        marker = line[:2]
        if marker in ("#$", "#@"):
            tokens = line[2:].split("#", 1)[0].split()
            if len(tokens) != 1:
                raise LeapAuditError(f"line {line_number}: malformed {marker} timestamp")
            stamp = _decimal(tokens[0], marker)
            if marker == "#$":
                if updated is not None:
                    raise LeapAuditError("duplicate #$ update metadata")
                updated = stamp
            else:
                if expiry is not None:
                    raise LeapAuditError("duplicate #@ expiry metadata")
                expiry = stamp
            hashed_parts.append(tokens[0])
        elif marker == "#h":
            if digest is not None:
                raise LeapAuditError("duplicate #h checksum")
            words = line[2:].split("#", 1)[0].split()
            if len(words) != 5 or any(not SHA1_GROUP.fullmatch(w) for w in words):
                raise LeapAuditError("invalid five-word #h SHA-1 checksum")
            digest = "".join(f"{int(w, 16):08x}" for w in words)
        elif line.startswith("#"):
            continue
        else:
            fields = line.split("#", 1)[0].split()
            if len(fields) != 2:
                raise LeapAuditError(f"line {line_number}: expected NTP seconds and TAI-UTC")
            stamp = _decimal(fields[0], "NTP seconds")
            offset = _decimal(fields[1], "TAI-UTC seconds")
            if offset > 2**31 - 1:
                raise LeapAuditError(f"line {line_number}: offset outside i32 range")
            records.append((stamp, offset))
            hashed_parts.extend(fields)

    if expiry is None or not records:
        raise LeapAuditError("missing #@ expiration or numeric transition records")
    if official and (updated is None or digest is None):
        raise LeapAuditError("official source must contain #$, #@ and #h")
    if updated is not None and updated >= expiry:
        raise LeapAuditError("update timestamp is not before expiration")
    if records[0] != (BASE_NTP, BASE_OFFSET):
        raise LeapAuditError("baseline must be 1972-01-01 TAI-UTC=10 seconds")
    if expiry <= records[-1][0]:
        raise LeapAuditError("expiry is not later than the final transition")
    if expiry % 86400:
        raise LeapAuditError("expiry is not at UTC midnight")
    from_ntp(expiry)

    previous_stamp, previous_offset = records[0]
    for index, (stamp, offset) in enumerate(records):
        stamp_date = from_ntp(stamp)
        if stamp % 86400 or stamp_date.day != 1:
            raise LeapAuditError("transition is not at UTC midnight on month start")
        if index > 0:
            if stamp <= previous_stamp:
                raise LeapAuditError("duplicate or out-of-order leap transition")
            if abs(offset - previous_offset) != 1:
                raise LeapAuditError("leap offset changed by more than one second")
        previous_stamp, previous_offset = stamp, offset

    computed = hashlib.sha1("".join(hashed_parts).encode("ascii")).hexdigest()
    if official and digest != computed:
        raise LeapAuditError(
            f"source #h checksum mismatch (published {digest}, calculated {computed})"
        )
    return LeapTable(updated, expiry, tuple(records), digest)


def compare_to_pin(candidate: LeapTable, pinned: LeapTable) -> bool:
    """Return True only if versioned numeric history and expiry are unchanged.

    Historical differences or shortened history/expiry are errors. Extra
    transitions or a later expiry require review, never automatic replacement.
    """
    old = pinned.entries
    if candidate.entries[: len(old)] != old:
        raise LeapAuditError("existing pinned historical leap entries differ")
    if candidate.expires_ntp < pinned.expires_ntp:
        raise LeapAuditError("candidate expiration is older than pinned expiration")
    if candidate.entries != old and candidate.entries[len(old)][0] >= candidate.expires_ntp:
        raise LeapAuditError("candidate adds leap entries after its own expiry")
    return candidate.entries == old and candidate.expires_ntp == pinned.expires_ntp


def render_proposal(table: LeapTable, version: str) -> str:
    """Render *review-only* candidate text on stdout; never edit a repository file."""
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,79}", version):
        raise LeapAuditError("candidate version must be a safe simple label")
    result = [
        "# REQUIRES MANUAL SCIENTIFIC REVIEW before replacing a pinned leap table.",
        f"# Candidate release: {version}",
        "# Upstream source: https://data.iana.org/time-zones/",
        "# Source is public domain; numeric extract, not upstream byte-identical.",
        f"# Upstream #h SHA-1 checked: {table.sha1}",
        "# Unit: NTP seconds from 1900-01-01 UTC, TAI-UTC integral seconds.",
    ]
    if table.updated_ntp is not None:
        result.append(f"# Upstream update NTP: {table.updated_ntp}")
    result.extend(
        [f"#@ {table.expires_ntp}"]
        + [f"{stamp} {offset}" for stamp, offset in table.entries]
    )
    return "\n".join(result) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Offline integrity + historical compatibility audit of IANA/NIST leaps"
    )
    parser.add_argument("source", type=Path, help="locally obtained official leap-seconds.list")
    parser.add_argument("--pinned", type=Path, default=CURRENT_PIN)
    parser.add_argument(
        "--as-of",
        dest="as_of",
        type=date.fromisoformat,
        help="require source validity at this UTC day (YYYY-MM-DD); no system clock used",
    )
    parser.add_argument(
        "--propose", metavar="VERSION",
        help="print review-only candidate snapshot to stdout; never write files",
    )
    args = parser.parse_args(argv)
    try:
        if args.source.stat().st_size > MAX_BYTES or args.pinned.stat().st_size > MAX_BYTES:
            raise LeapAuditError("input exceeds 1 MiB safety limit")
        upstream = parse_table(args.source.read_text(encoding="utf-8"), official=True)
        pinned = parse_table(args.pinned.read_text(encoding="utf-8"), official=False)
        if args.as_of is not None and args.as_of >= upstream.expiry_utc.date():
            raise LeapAuditError("official source expired at the requested UTC date")
        matched = compare_to_pin(upstream, pinned)
        if args.propose is not None:
            sys.stdout.write(render_proposal(upstream, args.propose))
        else:
            print(
                f"SHA-1: verified; upstream rows={len(upstream.entries)}; "
                f"last offset={upstream.entries[-1][1]} seconds; "
                f"updated={upstream.updated_ntp}; expires={upstream.expiry_utc.isoformat()}; "
                f"pinned rows={len(pinned.entries)}; "
                f"status={'UNCHANGED' if matched else 'REVIEW REQUIRED'}"
            )
        return 0 if matched else 2
    except (OSError, UnicodeError, LeapAuditError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
