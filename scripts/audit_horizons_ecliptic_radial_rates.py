# SPDX-License-Identifier: CPAL-1.0
"""Independent Horizons/ERFA reference audit, not a production engine.

Usage: python scripts/audit_horizons_ecliptic_radial_rates.py KERNEL SHA256 RUST_EXE NEW_JSON
Uses existing Rust calculations and ERFA reference routines. Makes ten Horizons
requests. Raw replies stay in the new caller-owned report; never overwrites.
"""
import csv
import datetime
import hashlib
import io
import json
import math
import pathlib
import re
import subprocess
import sys
import urllib.parse
import urllib.request

import erfa
import numpy as np

AU_KM = 149597870.7
YEARS = [1901, 1925, 1950, 1975, 2000, 2025, 2050, 2075, 2099]
OFFSETS = [-0.08, -0.04, -0.02, 0.0, 0.02, 0.04, 0.08]
TARGETS = [('Sun', '10', 'Sun'), ('Moon', '301', 'Moon'), ('Mercury', '199', 'Mercury'),
           ('Venus', '299', 'Venus'), ('MarsBarycenter', '4', 'Mars Barycenter'),
           ('JupiterBarycenter', '5', 'Jupiter Barycenter'), ('SaturnBarycenter', '6', 'Saturn Barycenter'),
           ('UranusBarycenter', '7', 'Uranus Barycenter'), ('NeptuneBarycenter', '8', 'Neptune Barycenter'),
           ('PlutoBarycenter', '9', 'Pluto Barycenter')]


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(4 * 1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def four_point(values, jds, indexes, center):
    """Derivative by interpolating four offsets; actual binary JD spacing."""
    steps = np.array([jds[i] - center for i in indexes], dtype=float)
    scale = max(abs(steps))
    x = steps / scale
    weights = np.linalg.solve(np.vstack([x**k for k in range(4)]), np.array([0., 1., 0., 0.])) / scale
    return float(np.dot(weights, [values[i] - values[3] for i in indexes]))


def ecliptic(ra, dec, jd_tt):
    p = erfa.s2c(math.radians(ra), math.radians(dec))
    dpsi, deps = erfa.nut00b(2451545.0, jd_tt - 2451545.0)
    eps = erfa.obl06(2451545.0, jd_tt - 2451545.0)
    rotation = erfa.rx(eps + deps, erfa.numat(eps, dpsi, deps) @ erfa.pmat06(2451545.0, jd_tt - 2451545.0))
    lon, lat = erfa.c2s(rotation @ p)
    return math.degrees(lon) % 360.0, math.degrees(lat)


def main():
    if len(sys.argv) != 5:
        raise SystemExit(__doc__)
    kernel, expected, executable, out = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3]), pathlib.Path(sys.argv[4])
    if not re.fullmatch('[a-f0-9]{64}', expected) or out.exists() or len({kernel.resolve(), executable.resolve(), out.resolve()}) != 3:
        raise SystemExit('invalid inputs or output exists')
    if sha(kernel) != expected:
        raise SystemExit('kernel hash mismatch')
    executable_hash = sha(executable)
    epochs = []
    for year in YEARS:
        d0, d1 = erfa.cal2jd(year, 7, 1)
        epochs.append(float(d0 + d1 + 0.5))
    jds = [jd + offset for jd in epochs for offset in OFFSETS]
    rust, tts = {}, []
    for jd in jds:
        result = subprocess.run([str(executable), str(kernel), f'{jd:.9f}'], text=True, capture_output=True, check=True)
        rows = list(csv.DictReader(io.StringIO(result.stdout)))
        if len(rows) != len(TARGETS) or [x['target'] for x in rows] != [x[0] for x in TARGETS]:
            raise SystemExit('unexpected Rust targets')
        tt = float(rows[0]['tt_jd'])
        if any(abs(float(x['tdb_jd']) - jd) > 1e-8 or abs(float(x['tt_jd']) - tt) > 1e-10 for x in rows):
            raise SystemExit('Rust epoch mismatch')
        tts.append(tt)
        for row in rows:
            rust[(row['target'], jd)] = row
    records, sources = [], []
    for label, command, name in TARGETS:
        params = {'format': 'json', 'COMMAND': f"'{command}'", 'OBJ_DATA': "'NO'", 'MAKE_EPHEM': "'YES'",
                  'EPHEM_TYPE': "'OBSERVER'", 'CENTER': "'500@399'", 'TLIST': "'" + ' '.join(f'{x:.9f}' for x in tts) + "'",
                  'TLIST_TYPE': "'JD'", 'TIME_TYPE': "'TT'", 'CAL_FORMAT': "'JD'", 'QUANTITIES': "'20,23,45'",
                  'REF_SYSTEM': "'ICRF'", 'ANG_FORMAT': "'DEG'", 'CSV_FORMAT': "'YES'", 'EXTRA_PREC': "'YES'", 'APPARENT': "'AIRLESS'"}
        url = 'https://ssd.jpl.nasa.gov/api/horizons.api?' + urllib.parse.urlencode(params)
        with urllib.request.urlopen(url, timeout=30) as response:
            raw = json.load(response)['result']
        if not re.search(rf'Target body name: {re.escape(name)} \({command}\).*\{{source: DE441\}}', raw) or not re.search(r'Center body name: Earth \(399\).*\{source: DE441\}', raw):
            raise SystemExit('unexpected Horizons target, center, or source')
        header = next((line for line in raw.splitlines() if 'RA_(ICRF-a-app)' in line), '')
        cols = [x.strip() for x in header.split(',')]
        if 'Date_________JDTT' not in cols:
            raise SystemExit('unexpected Horizons timescale')
        indexes = [cols.index(x) for x in ('delta', 'deldot', 'S-O-T', 'RA_(ICRF-a-app)', 'DEC_(ICRF-a-app)')]
        a, b = raw.find('$$SOE'), raw.find('$$EOE')
        if a < 0 or b <= a:
            raise SystemExit('missing Horizons ephemeris')
        lines = list(csv.reader(io.StringIO(raw[a + 5:b].strip())))
        if len(lines) != len(tts):
            raise SystemExit('unexpected reference row count')
        ref = []
        for fields, tt in zip(lines, tts):
            if not fields[0].strip() or any(not fields[i].strip() for i in indexes) or abs(float(fields[0]) - tt) > 1e-8:
                raise SystemExit('missing reference value or wrong TT')
            vals = [float(fields[i]) for i in indexes]
            if not all(math.isfinite(x) for x in vals):
                raise SystemExit('nonfinite reference')
            lon, lat = ecliptic(vals[3], vals[4], tt)
            ref.append({'lon': lon, 'lat': lat, 'rangeAu': vals[0], 'rateKmS': vals[1], 'elongationDeg': vals[2]})
        sources.append({'target': name, 'url': url, 'raw': raw})
        for n, (year, center) in enumerate(zip(YEARS, epochs)):
            group = ref[n * 7:n * 7 + 7]
            # Use the actual nine-decimal TT epochs sent to Horizons for the
            # derivative weights. Unrounded TDB offsets are not identical to
            # these samples, especially at short steps and f64 JD precision.
            local_jds = tts[n * 7:n * 7 + 7]
            lon0 = group[3]['lon']
            lons = [lon0 + ((x['lon'] - lon0 + 180.0) % 360.0 - 180.0) for x in group]
            lat = [x['lat'] for x in group]
            distance = [x['rangeAu'] for x in group]
            native = rust[(label, center)]
            got = [float(native['longitude_speed_deg_per_tdb_day']), float(native['latitude_speed_deg_per_tdb_day']),
                   float(native['distance_speed_au_per_tdb_day'])]
            if not all(math.isfinite(x) for x in got):
                raise SystemExit('nonfinite Rust rates')
            stencils = []
            for h, points in ((0.02, [1, 2, 4, 5]), (0.04, [0, 1, 5, 6])):
                d = [four_point(series, local_jds, points, local_jds[3]) for series in (lons, lat, distance)]
                stencils.append({'halfStepDays': h, 'reference': d, 'longitudeErrorArcsecDay': (got[0] - d[0]) * 3600,
                                 'latitudeErrorArcsecDay': (got[1] - d[1]) * 3600, 'radialErrorAuDay': got[2] - d[2],
                                 'q20MinusRangeDerivativeKmS': group[3]['rateKmS'] - d[2] * AU_KM / 86400.0})
            records.append({'target': name, 'year': year, 'jdTdb': center, 'jdTt': tts[n * 7 + 3],
                            'elongationDeg': group[3]['elongationDeg'], 'rustRates': got,
                            'q20RangeRateKmS': group[3]['rateKmS'], 'stencils': stencils})
    if sha(kernel) != expected or sha(executable) != executable_hash:
        raise SystemExit('input changed')
    all_stencils = [(record, s) for record in records for s in record['stencils']]
    def result(key, limit):
        maximum = max(all_stencils, key=lambda x: abs(x[1][key]))
        return {'limit': limit, 'cases': len(all_stencils), 'failures': sum(abs(s[key]) > limit for _, s in all_stencils),
                'maximum': {'value': maximum[1][key], 'target': maximum[0]['target'], 'year': maximum[0]['year'], 'halfStepDays': maximum[1]['halfStepDays']}}
    summary = {'longitudeArcsecDay': result('longitudeErrorArcsecDay', 0.05), 'latitudeArcsecDay': result('latitudeErrorArcsecDay', 0.05),
               'radialAuDay': result('radialErrorAuDay', 5e-10), 'q20DirectVsRangeDerivativeKmS': result('q20MinusRangeDerivativeKmS', 0.001),
               'longitudeSignMismatches': sum(any(r['rustRates'][0] * s['reference'][0] < 0 for s in r['stencils']) for r in records)}
    doc = {'schema': 'horizons-erfa-ecliptic-radial-rates/1', 'status': 'EXPLORATORY', 'capturedUtc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
           'referenceDerivativeScale': 'TT day (actual requested TT epochs)', 'rustDerivativeScale': 'TDB day',
           'kernelSha256': expected, 'binarySha256': executable_hash, 'erfaVersion': erfa.__version__, 'summary': summary,
           'records': records, 'sources': sources}
    with out.open('x', encoding='utf8', newline='\n') as destination:
        json.dump(doc, destination, indent=2, allow_nan=False)
        destination.write('\n')
    print(json.dumps(summary))
    if any(summary[k]['failures'] for k in ('longitudeArcsecDay', 'latitudeArcsecDay', 'radialAuDay', 'q20DirectVsRangeDerivativeKmS')):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
