# SPDX-License-Identifier: CPAL-1.0
"""Compare existing Rust experimental orientation against ERFA model routines.

Usage: python scripts/audit_package_orientation_erfa.py RUST_EXE NEW_JSON
Reference ERFA and Rust are independent implementations of related models;
this is not an independent observation of historical Earth orientation.
"""
import csv
import io
import json
import math
import pathlib
import subprocess
import sys

import erfa
import numpy as np

from audit_horizons_ecliptic_radial_rates import sha


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    exe, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    if exe.resolve() == out.resolve() or out.exists():
        raise SystemExit('output exists or aliases input')
    exe_hash = sha(exe)
    dates = []
    for year in range(0, 2301, 25):
        for month in (1, 4, 7, 10):
            jd0, jd1 = erfa.cal2jd(year, month, 1)
            dates.append((year, month, float(jd0 + jd1 + 0.5)))
    proc = subprocess.run([str(exe)], input=''.join(f'{jd:.9f}\n' for _, _, jd in dates),
                          text=True, capture_output=True, check=True)
    rows = list(csv.reader(io.StringIO(proc.stdout)))
    if len(rows) != len(dates):
        raise SystemExit('wrong Rust output count')
    arcsec = 180.0 / math.pi * 3600.0
    records = []
    for (year, month, jd), fields in zip(dates, rows):
        if len(fields) != 15 or abs(float(fields[0]) - jd) > 1e-8:
            raise SystemExit('wrong Rust epoch or field count')
        values = np.array([float(x) for x in fields[1:]], dtype=float)
        if not np.isfinite(values).all():
            raise SystemExit('nonfinite Rust output')
        epj = 2000.0 + (jd - 2451545.0) / 365.25
        eq_diff = np.max(np.abs(values[2:11].reshape(3, 3) - erfa.ltp(epj)))
        pole_diff = np.max(np.abs(values[11:14] - erfa.ltpecl(epj)))
        nut_a = np.array(erfa.nut00a(2451545.0, jd - 2451545.0))
        nut_b = np.array(erfa.nut00b(2451545.0, jd - 2451545.0))
        records.append({'year': year, 'month': month, 'jdTt': jd, 'equatorMaxElement': float(eq_diff),
                        'eclipticPoleMaxElement': float(pole_diff), 'experimentalMinus00aArcsec': ((values[0:2] - nut_a) * arcsec).tolist(),
                        'standard00bMinus00aArcsec': ((nut_b - nut_a) * arcsec).tolist()})
    if sha(exe) != exe_hash:
        raise SystemExit('executable changed')
    def summarize(subset):
        if not subset:
            return None
        def metric(key, limit):
            def mag(row):
                v = row[key]
                return max(map(abs, v)) if isinstance(v, list) else abs(v)
            worst = max(subset, key=mag)
            return {'limit': limit, 'maxAbs': mag(worst), 'failures': sum(mag(x) > limit for x in subset),
                    'year': worst['year'], 'month': worst['month']}
        return {'count': len(subset), 'equator': metric('equatorMaxElement', 1e-12),
                'pole': metric('eclipticPoleMaxElement', 1e-12),
                'experimentalMinus00aArcsec': metric('experimentalMinus00aArcsec', 0.05),
                'standard00bMinus00aArcsec': metric('standard00bMinus00aArcsec', 5.0)}
    summary = {'all': summarize(records), 'history0To1599': summarize([r for r in records if r['year'] < 1600]),
               'core1600To2100': summarize([r for r in records if 1600 <= r['year'] <= 2100]),
               'future2101To2300': summarize([r for r in records if 2100 < r['year'] <= 2300])}
    result = {'schema': 'package-orientation-erfa/1', 'status': 'EXPLORATORY', 'binarySha256': exe_hash,
              'erfaVersion': erfa.__version__, 'summary': summary, 'records': records}
    with out.open('x', encoding='utf8', newline='\n') as destination:
        json.dump(result, destination, indent=2, allow_nan=False)
        destination.write('\n')
    print(json.dumps(summary))
    if any(summary['all'][k]['failures'] for k in ('equator', 'pole', 'experimentalMinus00aArcsec', 'standard00bMinus00aArcsec')):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
