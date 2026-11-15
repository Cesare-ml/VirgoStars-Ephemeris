#!/usr/bin/env python3
# Original VirgoStars source, CPAL-1.0; see repository LICENSE and NOTICE.
# Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
"""Operator-only ERFA pmsafe reference for ICRS astrometric star motion.

Needs the genuine, vetted Hipparcos catalog + per-star provenance audit from
the separate importer, plus independently installed pyerfa. Does not download
data or certify measurement rights, source authenticity, or Swiss API parity.
"""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import sys
import warnings

HIP_EPOCH = 2448349.0625
J2000 = 2451545.0
MAS_TO_RAD = math.pi / (180.0 * 3600.0 * 1000.0)
DEFAULT_EPOCHS = [HIP_EPOCH, J2000, 2469807.5]


def read_pinned_json(path, expected_sha256, label):
    if not Path(path).is_absolute():
        raise ValueError(f'{label} path must be absolute')
    if not isinstance(expected_sha256, str) or len(expected_sha256) != 64 or any(
        c not in '0123456789abcdef' for c in expected_sha256
    ):
        raise ValueError(f'{label} needs a trusted lowercase SHA-256')
    size = Path(path).stat().st_size
    if not 0 < size <= 64 * 1024 * 1024:
        raise ValueError(f'{label} file must be nonempty and at most 64 MiB')
    raw = Path(path).read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != expected_sha256:
        raise ValueError(f'{label}: SHA-256 mismatch')
    return json.loads(raw), digest


def ensure_science_inputs(catalog, audit):
    if not isinstance(catalog, dict) or not isinstance(audit, dict):
        raise ValueError('Expected catalog and matching audit JSON objects')
    if not isinstance(catalog.get('records'), list) or not catalog['records']:
        raise ValueError('Catalog has no records')
    if not isinstance(audit.get('stars'), list) or len(audit['stars']) != len(catalog['records']):
        raise ValueError('Catalog and audit record counts do not match')
    source = audit.get('hipparcos')
    if not isinstance(source, dict) or source.get('catalog') != 'CDS I/311/hip2' or source.get('rows') != 117955:
        raise ValueError('Reference needs a full locally checked Hipparcos I/311 source')
    if audit.get('complexSolutionsAllowed') is not False:
        raise ValueError('Complex astrometric solutions cannot be benchmark references')
    policy = audit.get('qualityPolicy', {})
    if policy.get('requireAllFiveFormalUncertainties') is not True or policy.get('minParallaxSignalToNoise', 0) < 5:
        raise ValueError('Required formal-error and parallax SNR gates not recorded')
    h1 = source.get('sha256')
    h2 = audit.get('supplements', {}).get('sha256')
    if any(not isinstance(h, str) or len(h) != 64 or any(c not in '0123456789abcdef' for c in h) for h in (h1, h2)):
        raise ValueError('Source SHA-256 pins are missing')
    if catalog.get('id') != 'hipparcos-2007-curated-' + h1[:12] + '-' + h2[:12] + '-standard':
        raise ValueError('Catalog ID does not match audit source hashes')
    index = {a.get('name'): a for a in audit['stars']}
    if len(index) != len(audit['stars']):
        raise ValueError('Duplicate audit names')
    required_errors = {'raMas', 'decMas', 'parallaxMas', 'properMotionRaMasPerYear', 'properMotionDecMasPerYear'}
    for star in catalog['records']:
        if not isinstance(star, dict):
            raise ValueError('Invalid star record')
        item = index.get(star.get('name'))
        if item is None or item.get('astrometryReviewStatus') != 'STANDARD_5_PARAM':
            raise ValueError('Missing matching standard five-parameter audit')
        errors = item.get('formalUncertainties', {})
        if not isinstance(errors, dict) or set(errors) != required_errors or any(
            not isinstance(v, (float, int)) or not math.isfinite(v) or v < 0 for v in errors.values()
        ):
            raise ValueError('Incomplete formal-error audit')
        if errors['parallaxMas'] <= 0 or item.get('parallaxSignalToNoise', 0) < 5:
            raise ValueError('Unacceptable parallax evidence')
        if item.get('radialVelocityReferenceFrame') != 'BARYCENTRIC':
            raise ValueError('Cannot silently use nonbarycentric radial velocity')
        for key in ('visualMagnitudeSource', 'radialVelocitySource', 'nameSource'):
            if not isinstance(item.get(key), str) or not item[key].startswith('https://'):
                raise ValueError(f'Missing {key} evidence URL')
        fields = ('rightAscensionDeg', 'declinationDeg', 'properMotionRaCosDecMasPerYear',
                  'properMotionDecMasPerYear', 'parallaxMas', 'radialVelocityKmPerSecond',
                  'visualMagnitude', 'epochTdbJulianDay')
        if any(not isinstance(star.get(k), (float, int)) or not math.isfinite(star[k]) for k in fields):
            raise ValueError('Missing or nonfinite catalog data')
        if star['epochTdbJulianDay'] != HIP_EPOCH or not (0 <= star['rightAscensionDeg'] < 360):
            raise ValueError('Unexpected epoch or RA')
        if not (-90 < star['declinationDeg'] < 90) or not (0 < star['parallaxMas'] <= 10000):
            raise ValueError('Invalid Dec or parallax')
    return index


def generate(catalog, audit, erfa_module, epochs):
    index = ensure_science_inputs(catalog, audit)
    if not isinstance(epochs, list) or len(epochs) < 3 or len(set(epochs)) != len(epochs):
        raise ValueError('Require at least three distinct TDB Julian epochs')
    if HIP_EPOCH not in epochs or max(abs(j - HIP_EPOCH) for j in epochs) < 3652.5:
        raise ValueError('Reference needs Hipparcos epoch and 10+ year propagation')
    if any(not math.isfinite(j) or not 2400000 <= j <= 2600000 for j in epochs):
        raise ValueError('Epochs outside supported numerical range')
    stars = []
    for record in catalog['records']:
        ra = math.radians(record['rightAscensionDeg'])
        dec = math.radians(record['declinationDeg'])
        cos_dec = math.cos(dec)
        if abs(cos_dec) <= 1e-6:
            raise ValueError('Near-polar RA motion needs separate review')
        # Hipparcos pmRA = d(RA)/dt * cos(Dec), in mas/Julian yr.
        # ERFA pmsafe expects angular d(RA)/dt, in rad/Julian yr.
        pmra = record['properMotionRaCosDecMasPerYear'] * MAS_TO_RAD / cos_dec
        pmdec = record['properMotionDecMasPerYear'] * MAS_TO_RAD
        px = record['parallaxMas'] / 1000  # mas -> arcsec
        samples = []
        for jd in epochs:
            with warnings.catch_warnings():
                warnings.simplefilter('error')  # reject ERFA distance/velocity warnings
                result = erfa_module.pmsafe(
                    ra, dec, pmra, pmdec, px, record['radialVelocityKmPerSecond'],
                    J2000, HIP_EPOCH - J2000, J2000, jd - J2000
                )
            ra2, dec2 = float(result[0]), float(result[1])
            if not math.isfinite(ra2) or not math.isfinite(dec2):
                raise ValueError('Nonfinite ERFA propagated angle')
            samples.append({'tdbJulianDay': jd, 'rightAscensionDeg': math.degrees(ra2) % 360,
                            'declinationDeg': math.degrees(dec2)})
        stars.append({'name': record['name'], 'hip': index[record['name']]['hip'], 'samples': samples})
    return {
        'schemaVersion': 1, 'status': 'INDEPENDENT_ALGORITHM_REFERENCE_NOT_ENGINE_PARITY',
        'coordinateFrame': 'ICRS_ASTROMETRIC', 'referenceModel': 'ERFA_PMSAFE',
        'catalogId': catalog['id'], 'epochConvention': 'TDB_JULIAN_DAY',
        'epochsTdbJulianDay': epochs, 'stars': stars, 'scientificallyValidated': False,
        'limitations': [
            'Reference algorithm independent; source measurement quality and usage rights are not certified',
            'SSB-centered ICRS astrometric directions, not geocentric apparent ecliptic positions'
        ]
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('catalog', 'catalog-sha256', 'audit', 'audit-sha256', 'out'):
        parser.add_argument('--' + key, required=True)
    args = parser.parse_args()
    output_path = Path(args.out)
    if not output_path.is_absolute():
        raise ValueError('Reference output path must be absolute')
    output_path = output_path.resolve()
    root = Path(__file__).resolve().parent.parent
    if output_path == root or root in output_path.parents:
        raise ValueError('Reference output must remain outside Git')
    catalog, catalog_hash = read_pinned_json(args.catalog, args.catalog_sha256, 'Catalog')
    audit, audit_hash = read_pinned_json(args.audit, args.audit_sha256, 'Audit')
    try:
        import erfa  # optional external package pyerfa; no automatic installation
    except ImportError as exc:
        raise RuntimeError('Install independently verified pyerfa in an external Python environment') from exc
    report = generate(catalog, audit, erfa, DEFAULT_EPOCHS)
    report['inputCatalogSha256'] = catalog_hash
    report['inputAuditSha256'] = audit_hash
    report['referenceLibraryVersion'] = str(erfa.__version__)
    serialized = (json.dumps(report, indent=2, allow_nan=False) + '\n').encode('utf-8')
    fd = os.open(output_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(serialized)
    except BaseException:
        try:
            output_path.unlink()
        except OSError:
            pass
        raise
    print(json.dumps({'output': str(output_path), 'stars': len(report['stars']),
                      'epochsTdbJulianDay': DEFAULT_EPOCHS, 'scientificallyValidated': False}))


if __name__ == '__main__':
    try:
        main()
    except Exception as exc:
        print(str(exc), file=sys.stderr)
        sys.exit(1)
