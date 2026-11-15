# Original VirgoStars test source, CPAL-1.0; see LICENSE and NOTICE.
# Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
# Python standard-library tests; no real catalog or PyERFA installation needed.
import importlib.util
import math
from pathlib import Path
import unittest

MODULE = Path(__file__).resolve().parents[1] / 'scripts' / 'generate-hipparcos-erfa-reference.py'
spec = importlib.util.spec_from_file_location('virgostars_erfa_reference', MODULE)
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)


class SpyErfa:
    def __init__(self):
        self.calls = []

    def pmsafe(self, ra, dec, pmra, pmdec, plx, rv, ep1a, ep1b, ep2a, ep2b):
        self.calls.append((ra, dec, pmra, pmdec, plx, rv, ep1a, ep1b, ep2a, ep2b))
        return (ra, dec, pmra, pmdec, plx, rv)


def fixture():
    catalog = {
        'id': 'hipparcos-2007-curated-' + 'a' * 12 + '-' + 'b' * 12 + '-standard',
        'records': [{
            'name': 'Synthetic', 'epochTdbJulianDay': mod.HIP_EPOCH,
            'rightAscensionDeg': 123., 'declinationDeg': 60.,
            'properMotionRaCosDecMasPerYear': 10.,
            'properMotionDecMasPerYear': -20.,
            'parallaxMas': 10., 'radialVelocityKmPerSecond': 20.,
            'visualMagnitude': 3.
        }]
    }
    audit = {
        'hipparcos': {'catalog': 'CDS I/311/hip2', 'rows': 117955, 'sha256': 'a' * 64},
        'supplements': {'sha256': 'b' * 64},
        'complexSolutionsAllowed': False,
        'qualityPolicy': {'requireAllFiveFormalUncertainties': True, 'minParallaxSignalToNoise': 5},
        'stars': [{
            'name': 'Synthetic', 'hip': 123, 'astrometryReviewStatus': 'STANDARD_5_PARAM',
            'formalUncertainties': {
                'raMas': 1., 'decMas': 1., 'parallaxMas': 1.,
                'properMotionRaMasPerYear': 1., 'properMotionDecMasPerYear': 1.
            },
            'parallaxSignalToNoise': 10., 'radialVelocityReferenceFrame': 'BARYCENTRIC',
            'visualMagnitudeSource': 'https://example.test/V',
            'radialVelocitySource': 'https://example.test/RV',
            'nameSource': 'https://example.test/name'
        }]
    }
    return catalog, audit


class ErfaReferenceTests(unittest.TestCase):
    def test_cos_declination_and_arcsec_units_are_converted(self):
        catalog, audit = fixture()
        spy = SpyErfa()
        report = mod.generate(catalog, audit, spy, mod.DEFAULT_EPOCHS)
        self.assertEqual(report['coordinateFrame'], 'ICRS_ASTROMETRIC')
        self.assertFalse(report['scientificallyValidated'])
        self.assertEqual(len(spy.calls), 3)
        for call in spy.calls:
            self.assertAlmostEqual(call[2], 10 * mod.MAS_TO_RAD / math.cos(math.radians(60)), places=19)
            self.assertAlmostEqual(call[3], -20 * mod.MAS_TO_RAD, places=19)
            self.assertEqual(call[4], 0.01)
            self.assertEqual(call[5], 20.)
            self.assertAlmostEqual(call[6] + call[7], mod.HIP_EPOCH, places=7)

    def test_unsafe_astrometry_or_reference_frame_fails(self):
        catalog, audit = fixture()
        audit['stars'][0]['radialVelocityReferenceFrame'] = 'HELIOCENTRIC'
        with self.assertRaisesRegex(ValueError, 'nonbarycentric'):
            mod.generate(catalog, audit, SpyErfa(), mod.DEFAULT_EPOCHS)
        audit['stars'][0]['radialVelocityReferenceFrame'] = 'BARYCENTRIC'
        audit['qualityPolicy']['minParallaxSignalToNoise'] = 2
        with self.assertRaisesRegex(ValueError, 'SNR gates'):
            mod.generate(catalog, audit, SpyErfa(), mod.DEFAULT_EPOCHS)
        audit['qualityPolicy']['minParallaxSignalToNoise'] = 5
        audit['stars'][0]['formalUncertainties'].pop('raMas')
        with self.assertRaisesRegex(ValueError, 'Incomplete formal-error'):
            mod.generate(catalog, audit, SpyErfa(), mod.DEFAULT_EPOCHS)

    def test_three_epochs_and_long_baseline_are_required(self):
        catalog, audit = fixture()
        with self.assertRaisesRegex(ValueError, '10\+ year propagation'):
            mod.generate(catalog, audit, SpyErfa(), [mod.HIP_EPOCH, 2451545, 2452000])
        with self.assertRaisesRegex(ValueError, 'three distinct'):
            mod.generate(catalog, audit, SpyErfa(), [mod.HIP_EPOCH, 2451545])


if __name__ == '__main__':
    unittest.main()
