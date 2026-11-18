"""Local tests for the optional ERFA matrix audit.

SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
"""
import unittest

import numpy as np

from audit_erfa_long_term_axes import ARCSEC_PER_RAD, FIXED_FLAGS, check_fixed_flags, compare


class MatrixComparisonTests(unittest.TestCase):
    def test_fixed_direction_flags(self):
        self.assertTrue(check_fixed_flags(FIXED_FLAGS, FIXED_FLAGS | 64))
        self.assertFalse(check_fixed_flags(0, 0))
        self.assertFalse(check_fixed_flags(FIXED_FLAGS, FIXED_FLAGS | 16))
        self.assertFalse(check_fixed_flags(FIXED_FLAGS, FIXED_FLAGS & ~1))
        self.assertFalse(check_fixed_flags(FIXED_FLAGS, True))

    def test_identical_and_known_rotation(self):
        identity = np.eye(3)[None, :, :]
        components, angles = compare(identity, identity)
        self.assertEqual(components[0], 0.0)
        self.assertEqual(angles[0], 0.0)
        angle = 0.001
        c, s = np.cos(angle), np.sin(angle)
        rotated = np.array([[[c, -s, 0], [s, c, 0], [0, 0, 1]]])
        _, angles = compare(identity, rotated)
        self.assertAlmostEqual(angles[0], angle * ARCSEC_PER_RAD, places=10)

    def test_rejects_invalid_and_reflected_matrices(self):
        identity = np.eye(3)[None, :, :]
        with self.assertRaises(ValueError):
            compare(identity, np.eye(2)[None, :, :])
        with self.assertRaises(ValueError):
            compare(identity, identity * 2)
        with self.assertRaises(ValueError):
            compare(identity, np.diag([1, 1, -1])[None, :, :])
        invalid = identity.copy()
        invalid[0, 0, 0] = np.nan
        with self.assertRaises(ValueError):
            compare(identity, invalid)


if __name__ == "__main__":
    unittest.main()
