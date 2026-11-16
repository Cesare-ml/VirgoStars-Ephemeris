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
"""Optional independent CSPICE-generated SPK priority regression, no real data.

Run with independently installed spiceypy and offline Rust dependencies.
Fictional type-3 states deliberately have independently supplied velocities.
"""
from __future__ import annotations

import math
from pathlib import Path
import sys
import tempfile
import unittest

try:
    import spiceypy as spice
except ImportError:
    spice = None

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from spk_naif_reference import run_validation  # noqa: E402


def write_segment(handle, body, center, start, end, value, label):
    # One record, degree 1, six independent coefficient pairs (X,Y,Z,VX,VY,VZ).
    coefficients = [value, 1.0, value + 1, 2.0, value + 2, 3.0,
                    value / 10, 0.1, value / 10 + 1, 0.2, value / 10 + 2, 0.3]
    spice.spkw03(handle, body, center, 'J2000', start, end, label,
                 end - start, 1, 1, coefficients, start)


@unittest.skipIf(spice is None, 'independently installed spiceypy is required')
class SpiceSyntheticPriority(unittest.TestCase):
    def test_same_file_and_kernel_order_precedence(self):
        with tempfile.TemporaryDirectory(prefix='virgostars-spk-priority-') as temporary:
            folder = Path(temporary)
            base, overlay = folder / 'base.bsp', folder / 'overlay.bsp'
            handle = spice.spkopn(str(base), 'fictional base', 0)
            try:
                write_segment(handle, 1000001, 0, -10, 10, 100, 'base')
                write_segment(handle, 1000001, 0, -5, 5, 200, 'same-file overlay')
                write_segment(handle, 1000002, 1000001, -10, 10, 10, 'chain')
            finally:
                spice.spkcls(handle)
            handle = spice.spkopn(str(overlay), 'fictional overlay', 0)
            try:
                write_segment(handle, 1000001, 0, -3, 3, 300, 'later file overlay')
            finally:
                spice.spkcls(handle)

            epochs = [-10.0, math.nextafter(-5.0, -math.inf), -5.0,
                      math.nextafter(-3.0, -math.inf), -3.0, 0.0, 3.0,
                      math.nextafter(3.0, math.inf), 5.0,
                      math.nextafter(5.0, math.inf), 10.0]
            cases = [f'{target}:0:{epoch:.17g}' for target in (1000001, 1000002) for epoch in epochs]
            for order, expected, name in (([base, overlay], 300, 'overlay-last'),
                                          ([overlay, base], 200, 'base-last')):
                try:
                    spice.kclear()
                    for path in order:
                        spice.furnsh(str(path))
                    state, _ = spice.spkezr('1000001', 0, 'J2000', 'NONE', '0')
                    self.assertAlmostEqual(state[0], expected)
                finally:
                    spice.kclear()
                receipt = run_validation(order, cases, folder / f'{name}.csv',
                                         folder / f'{name}.json', 1e-9, 1e-11, spice)
                self.assertEqual(receipt['case_count'], 22)
                self.assertFalse(receipt['scientific_accuracy_certified'])


if __name__ == '__main__':
    unittest.main()
