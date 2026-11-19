# SPDX-License-Identifier: CPAL-1.0
"""Tests the numerical stencil independently of ephemeris data or Horizons."""
import unittest
from audit_horizons_ecliptic_radial_rates import four_point


class StencilTests(unittest.TestCase):
    def test_cubic_on_uneven_representable_jd_offsets(self):
        center = 2451545.0
        offsets = [-0.081, -0.041, -0.019, 0.0, 0.021, 0.039, 0.079]
        jds = [center + x for x in offsets]
        values = [4.0 + 7.0 * (jd - center) - 2.0 * (jd - center)**2 + 3.0 * (jd - center)**3 for jd in jds]
        for points in ([1, 2, 4, 5], [0, 1, 5, 6]):
            self.assertAlmostEqual(four_point(values, jds, points, center), 7.0, places=11)


if __name__ == '__main__':
    unittest.main()
