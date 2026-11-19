# SPDX-License-Identifier: CPAL-1.0
"""Protect the reference-sample timing and indeterminate-sign conventions."""
import unittest

from audit_horizons_ecliptic_radial_rates import four_point
from audit_station_boundary_rates import sign


class ReferenceNumericsTests(unittest.TestCase):
    def test_correct_stencil_uses_actual_sample_times(self):
        center = 2415020.03
        ideal = [center + x for x in (-0.01, -0.005, -0.0025, 0, 0.0025, 0.005, 0.01)]
        actual = list(ideal)
        actual[4] += 2e-8
        values = [0.7 + 0.02 * (t - center) + 0.1 * (t - center)**2 for t in actual]
        points = [1, 2, 4, 5]
        correct = four_point(values, actual, points, actual[3])
        incorrect = four_point(values, ideal, points, ideal[3])
        self.assertAlmostEqual(correct, 0.02, places=11)
        self.assertGreater(abs(incorrect - 0.02), 1e-9)

    def test_sign_deadband_includes_boundaries(self):
        self.assertEqual(sign(-0.0005), 0)
        self.assertEqual(sign(0.0), 0)
        self.assertEqual(sign(0.0005), 0)
        self.assertEqual(sign(-0.0005001), -1)
        self.assertEqual(sign(0.0005001), 1)


if __name__ == '__main__':
    unittest.main()
