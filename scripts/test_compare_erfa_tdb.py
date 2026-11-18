"""Local tests for the optional ERFA time-model helper.

SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
"""
import json
import pathlib
import subprocess
import sys
import unittest

import erfa


class TimeModelTests(unittest.TestCase):
    def test_published_erfa_anchor(self):
        # ERFA's t_dtdb example, topocentric; this only checks the reference.
        result = erfa.dtdb(2448939.5, 0.123, 0.76543, 5.0123, 5525.242, 3190.0)
        self.assertAlmostEqual(result, -0.001280368005936999, delta=1e-15)

    def test_helper_geocenter_and_duplicate_rejection(self):
        helper = pathlib.Path(__file__).with_name("compare_erfa_tdb.py")
        def run(payload):
            return subprocess.run([sys.executable, str(helper)], input=json.dumps(payload), text=True, capture_output=True, check=False)
        good = run({"epochs": [2451545.0, -1911000.0]})
        self.assertEqual(good.returncode, 0, good.stderr)
        rows = json.loads(good.stdout)["rows"]
        self.assertEqual([r[0] for r in rows], [2451545.0, -1911000.0])
        self.assertAlmostEqual(rows[0][1], float(erfa.dtdb(2451545.0, 0.0, 0.0, 0.0, 0.0, 0.0)), delta=1e-15)
        self.assertNotEqual(run({"epochs": [2451545.0, 2451545.0]}).returncode, 0)


if __name__ == "__main__":
    unittest.main()
