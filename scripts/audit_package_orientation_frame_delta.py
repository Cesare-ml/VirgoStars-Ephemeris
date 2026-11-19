# SPDX-License-Identifier: CPAL-1.0
"""Post-hoc model rotation diagnostic at the saved control TT epochs.

Usage: python scripts/audit_package_orientation_frame_delta.py INPUT INPUT_SHA256 NEW_JSON
Uses two ERFA theoretical frame models, not observed historical orientation.
"""
import json
import math
import pathlib
import re
import sys

import erfa
import numpy as np

from audit_horizons_ecliptic_radial_rates import sha


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    source, expected, out = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3])
    if not re.fullmatch('[a-f0-9]{64}', expected) or out.exists() or source.resolve() == out.resolve() or sha(source) != expected:
        raise SystemExit('invalid source or output')
    data = json.loads(source.read_text(encoding='utf8'))
    if data.get('schema') != 'package-orientation-horizons/1':
        raise SystemExit('invalid report')
    labels = ('lower_outside', 'lower_inside', 'upper_inside', 'upper_outside', 'j2000', 'modern_control')
    rows = []
    for label in labels:
        items = [s for s in data['samples'] if s['label'] == label]
        if len(items) != 1:
            raise SystemExit('missing control')
        sample = items[0]
        tt = sample['full'][0]['jdTt']
        epj = 2000.0 + (tt - 2451545.0) / 365.25
        eq, pole = erfa.ltp(epj), erfa.ltpecl(epj)
        ecl = np.array([eq[0], np.cross(pole, eq[0]), pole])
        dpsi, deps = erfa.nut00a(2451545.0, tt - 2451545.0)
        bias = erfa.pmat06(2451545.0, 0.0)
        long_matrix = erfa.rz(-dpsi, ecl @ bias)
        eps = erfa.obl06(2451545.0, tt - 2451545.0)
        standard = erfa.rx(eps + deps, erfa.numat(eps, dpsi, deps) @ erfa.pmat06(2451545.0, tt - 2451545.0))
        relative = long_matrix @ standard.T
        skew = np.array([relative[2, 1] - relative[1, 2], relative[0, 2] - relative[2, 0], relative[1, 0] - relative[0, 1]]) / 2
        angle = math.atan2(float(np.linalg.norm(skew)), float((np.trace(relative) - 1) / 2)) * 180.0 / math.pi * 3600.0
        rows.append({'label': label, 'jdTdb': sample['jdTdb'], 'jdTt': tt, 'rotationArcsec': angle})
    if sha(source) != expected:
        raise SystemExit('source changed')
    result = {'schema': 'package-orientation-frame-delta/1', 'status': 'POST_HOC_DIAGNOSTIC', 'sourceSha256': expected,
              'erfaVersion': erfa.__version__, 'note': 'Both matrices use ERFA nut00a; difference is between models, not a physical accuracy or automatic-switch threshold.', 'rows': rows}
    with out.open('x', encoding='utf8', newline='\n') as destination:
        json.dump(result, destination, indent=2, allow_nan=False)
        destination.write('\n')
    print(json.dumps(rows))


if __name__ == '__main__':
    main()
