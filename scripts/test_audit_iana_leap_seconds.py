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

"""Deterministic offline tests of the IANA leap-second maintenance audit."""

import contextlib
from datetime import datetime, timedelta, timezone
import hashlib
import io
from pathlib import Path
import tempfile
import unittest

import audit_iana_leap_seconds as audit


OFFICIAL_UPDATED_NTP = 3976686858
OFFICIAL_EXPIRY_NTP = 4007404800
OFFICIAL_2026B_SHA1 = "2e1012704e6749f82f1792b714a0c18836bb19d6"


def pinned_rows():
    return audit.parse_table(audit.CURRENT_PIN.read_text(encoding="utf-8"), official=False).entries


def original_source(rows=None, update=OFFICIAL_UPDATED_NTP, expiry=OFFICIAL_EXPIRY_NTP):
    """Generate NIST/IANA-format text with a correct #h from input tokens."""
    if rows is None:
        rows = pinned_rows()
    canonical = [str(update), str(expiry)]
    lines = ["# Public domain NTP leap seconds", f"#$ {update}", f"#@ {expiry}"]
    for stamp, offset in rows:
        canonical.extend([str(stamp), str(offset)])
        lines.append(f"{stamp} {offset} # UTC midnight")
    sha1 = hashlib.sha1("".join(canonical).encode("ascii")).hexdigest()
    words = [f"{int(sha1[i:i + 8], 16):x}" for i in range(0, 40, 8)]
    lines.append("#h " + " ".join(words))
    return "\n".join(lines) + "\n"


class LeapAuditTests(unittest.TestCase):
    def test_official_2026b_hash_known_independently_and_pinned_equal(self):
        source = original_source()
        calculated = audit.parse_table(source, official=True)
        self.assertEqual(calculated.sha1, OFFICIAL_2026B_SHA1)
        self.assertEqual(calculated.updated_ntp, OFFICIAL_UPDATED_NTP)
        self.assertEqual(calculated.expiry_utc.date().isoformat(), "2026-12-28")
        self.assertTrue(
            audit.compare_to_pin(
                calculated,
                audit.parse_table(audit.CURRENT_PIN.read_text(encoding="utf-8"), official=False),
            )
        )

    def test_tampered_single_offset_rejected_by_upstream_integrity_hash(self):
        source = original_source().replace("3692217600 37", "3692217600 36")
        with self.assertRaisesRegex(audit.LeapAuditError, "checksum mismatch"):
            audit.parse_table(source, official=True)

    def test_edited_history_even_with_consistent_new_hash_is_rejected(self):
        rows = list(pinned_rows())
        rows[1] = (rows[1][0], 12)
        # The parser itself also rejects invalid nonunit change.
        with self.assertRaises(audit.LeapAuditError):
            audit.parse_table(original_source(rows), official=True)

        # An internally consistent alternate historical sequence must
        # still not be allowed to revise previously approved entries.
        rows[1] = (rows[1][0], 11)
        rows[-1] = (rows[-1][0], 36)
        rows = rows[:-1]
        new_expiry = OFFICIAL_EXPIRY_NTP + 86400 * 180
        candidate = audit.parse_table(original_source(rows, expiry=new_expiry), official=True)
        pinned = audit.parse_table(audit.CURRENT_PIN.read_text(encoding="utf-8"), official=False)
        with self.assertRaisesRegex(audit.LeapAuditError, "historical"):
            audit.compare_to_pin(candidate, pinned)

    def test_extension_without_new_leap_needs_review_and_never_overwrites(self):
        source = original_source(expiry=OFFICIAL_EXPIRY_NTP + 86400 * 180)
        candidate = audit.parse_table(source, official=True)
        pinned = audit.parse_table(audit.CURRENT_PIN.read_text(encoding="utf-8"), official=False)
        self.assertFalse(audit.compare_to_pin(candidate, pinned))
        proposal = audit.render_proposal(candidate, "tzdb-2027a")
        self.assertIn("# REQUIRES MANUAL SCIENTIFIC REVIEW", proposal)
        self.assertIn(f"#@ {OFFICIAL_EXPIRY_NTP + 86400 * 180}", proposal)
        self.assertIn("3692217600 37", proposal)

    def test_future_transition_requires_review_not_silent_approval(self):
        rows = list(pinned_rows())
        future = datetime(2027, 1, 1, tzinfo=timezone.utc)
        future_ntp = int((future - audit.NTP_EPOCH).total_seconds())
        rows.append((future_ntp, 38))  # hypothetical, not an IERS announcement
        expiry_ntp = future_ntp + 86400 * 365
        candidate = audit.parse_table(original_source(rows, expiry=expiry_ntp), official=True)
        pinned = audit.parse_table(audit.CURRENT_PIN.read_text(encoding="utf-8"), official=False)
        self.assertFalse(audit.compare_to_pin(candidate, pinned))
        self.assertIn(f"{future_ntp} 38", audit.render_proposal(candidate, "candidate"))
        self.assertEqual(len(candidate.entries), 29)

    def test_malformed_sources_fail_closed(self):
        source = original_source()
        for wrong in [
            source.replace("#h ", "# invalid-h ", 1),
            source.replace("#@ ", "# invalid-expiry ", 1),
            source.replace("2287785600 11", "2287785601 11", 1),
            source.replace("2287785600 11", "2287785600 13", 1),
            source.replace("2287785600 11", "2272060800 10", 1),
            source.replace("2287785600 11", "2287785600 11\n2287785600 11", 1),
            source.replace(f"#$ {OFFICIAL_UPDATED_NTP}", "#$ not_a_number", 1),
            source.replace("#h ", "#h abcdeffed abc 1 bad ", 1),
        ]:
            with self.subTest(fault=wrong[:90]):
                with self.assertRaises(audit.LeapAuditError):
                    audit.parse_table(wrong, official=True)

    def test_as_of_expiry_is_explicit_and_cli_does_not_write(self):
        source = original_source()
        before = audit.CURRENT_PIN.read_bytes()
        with tempfile.TemporaryDirectory() as tempdir:
            path = Path(tempdir) / "official.list"
            path.write_text(source, encoding="utf-8")
            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                valid = audit.main([str(path), "--as-of", "2026-12-27"])
            self.assertEqual(valid, 0, stderr.getvalue())
            self.assertIn("UNCHANGED", stdout.getvalue())

            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(stderr):
                expired = audit.main([str(path), "--as-of", "2026-12-28"])
            self.assertEqual(expired, 1)
            self.assertIn("expired", stderr.getvalue())
        self.assertEqual(audit.CURRENT_PIN.read_bytes(), before)

    def test_cli_review_required_exit_code_and_nonmutating_proposal(self):
        source = original_source(expiry=OFFICIAL_EXPIRY_NTP + 86400 * 180)
        before = audit.CURRENT_PIN.read_bytes()
        with tempfile.TemporaryDirectory() as tempdir:
            path = Path(tempdir) / "updated.list"
            path.write_text(source, encoding="utf-8")
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                rc = audit.main([str(path), "--propose", "tzdb-2027a"])
            self.assertEqual(rc, 2)
            self.assertIn("tzdb-2027a", output.getvalue())
        self.assertEqual(audit.CURRENT_PIN.read_bytes(), before)

    def test_unsafe_version_and_downgrade_rejected(self):
        pinned = audit.parse_table(audit.CURRENT_PIN.read_text(encoding="utf-8"), official=False)
        candidate = audit.parse_table(original_source(), official=True)
        with self.assertRaisesRegex(audit.LeapAuditError, "safe simple label"):
            audit.render_proposal(candidate, "../../etc/passwd")
        old = audit.parse_table(
            original_source(expiry=OFFICIAL_EXPIRY_NTP - 86400), official=True
        )
        with self.assertRaisesRegex(audit.LeapAuditError, "older"):
            audit.compare_to_pin(old, pinned)


if __name__ == "__main__":
    unittest.main()
