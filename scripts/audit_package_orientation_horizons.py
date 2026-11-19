# SPDX-License-Identifier: CPAL-1.0
"""Assess the existing experimental Rust model with local packs, JPL and ERFA.

Usage: python scripts/audit_package_orientation_horizons.py KERNEL MANIFEST PACK_DIR RUST_EXE NEW_JSON
Network is used only to query Horizons; kernels are read locally and not copied.
Long-term output remains experimental. Input hashes and permissions are unchanged.
"""
import csv
import datetime
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

from audit_horizons_ecliptic_radial_rates import AU_KM, TARGETS, four_point, sha

EXPECTED_SOURCE = '476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da'
OFFSETS = [-0.08, -0.04, -0.02, 0.0, 0.02, 0.04, 0.08]


def engine(exe, kernel, cases):
    text = ''.join(f'{target},{jd:.9f}\n' for target, jd in cases)
    proc = subprocess.run([str(exe), str(kernel)], input=text, text=True, capture_output=True, check=True)
    lines = proc.stdout.splitlines()
    if len(lines) != len(cases):
        raise SystemExit('wrong Rust output count')
    result = []
    for line, (target, requested) in zip(lines, cases):
        fields = line.split(',')
        if len(fields) < 4 or fields[0] != str(target) or abs(float(fields[1]) - requested) > 1e-8:
            raise SystemExit('unexpected Rust output')
        row = {'id': str(target), 'jdTdb': float(fields[1]), 'jdTt': float(fields[2]), 'standardStatus': fields[3]}
        if target == 'tt':
            if fields[3] != 'time':
                raise SystemExit('wrong time output')
        else:
            if len(fields) < 10:
                raise SystemExit('missing candidate fields')
            row['candidate'] = [float(x) for x in fields[4:10]]
            if not all(math.isfinite(x) for x in row['candidate']):
                raise SystemExit('nonfinite candidate')
            if fields[3] == 'ok':
                if len(fields) != 16:
                    raise SystemExit('wrong standard field count')
                row['standard'] = [float(x) for x in fields[10:16]]
            elif fields[3] == 'error':
                row['standardError'] = ','.join(fields[10:]).strip()
            else:
                raise SystemExit('bad standard status')
        result.append(row)
    return result


def erfa_long_ecliptic(ra, dec, jd_tt):
    epj = 2000.0 + (jd_tt - 2451545.0) / 365.25
    equator = erfa.ltp(epj)
    pole = erfa.ltpecl(epj)
    axes = np.array([equator[0], np.cross(pole, equator[0]), pole])
    bias = erfa.pmat06(2451545.0, 0.0)
    dpsi, _ = erfa.nut00a(2451545.0, jd_tt - 2451545.0)
    matrix = erfa.rz(-dpsi, axes @ bias)
    vec = erfa.s2c(math.radians(ra), math.radians(dec))
    lon, lat = erfa.c2s(matrix @ vec)
    return math.degrees(lon) % 360.0, math.degrees(lat)


def wrap(value):
    return (value + 180.0) % 360.0 - 180.0


def main():
    if len(sys.argv) != 6:
        raise SystemExit(__doc__)
    kernel, manifest, packs, exe, out = map(pathlib.Path, sys.argv[1:])
    if out.exists() or len({kernel.resolve(), manifest.resolve(), exe.resolve(), out.resolve()}) != 4:
        raise SystemExit('output exists or aliases input')
    mh, eh = sha(manifest), sha(exe)
    metadata = json.loads(manifest.read_text(encoding='utf8'))
    if metadata.get('schema') != 'virgostars.de441.mobile-packs.v1' or metadata['source']['sha256'] != EXPECTED_SOURCE or len(metadata['packages']) != 10:
        raise SystemExit('unexpected manifest')
    if sha(kernel) != EXPECTED_SOURCE or kernel.stat().st_size != metadata['source']['bytes']:
        raise SystemExit('source mismatch')
    samples, files = [], {}
    for package in metadata['packages']:
        art = package['artifact']; filename = art['file']
        if pathlib.Path(filename).name != filename or package['id'] in files:
            raise SystemExit('invalid package name or ID')
        file = packs / filename
        if file.stat().st_size != art['bytes'] or sha(file) != art['sha256']:
            raise SystemExit('pack hash or size mismatch')
        files[package['id']] = {'path': file, 'sha256': art['sha256']}
        for label, jd in (('start_plus_1d', art['requested_start_jd_tdb'] + 1.0),
                          ('end_minus_1d', art['requested_end_exclusive_jd_tdb'] - 1.0)):
            samples.append({'label': label, 'package': package['id'], 'jdTdb': jd})
    samples += [{'label': name, 'jdTdb': jd} for name, jd in (
        ('lower_outside', 2415020.0 - 0.0101), ('lower_inside', 2415020.0 + 0.0101),
        ('upper_inside', 2488070.0 - 0.0101), ('upper_outside', 2488070.0 + 0.0101),
        ('j2000', 2451545.0), ('modern_control', 2461362.0))]
    if len(samples) != 26 or len({x['jdTdb'] for x in samples}) != 26:
        raise SystemExit('unexpected sample selection')
    cases = [(body, s['jdTdb']) for s in samples for body in range(10)]
    full = engine(exe, kernel, cases)
    for i, sample in enumerate(samples):
        sample['full'] = full[i * 10:i * 10 + 10]
    pack_checks = []
    for package_id, info in files.items():
        selected = [s for s in samples if s.get('package') == package_id]
        pack_results = engine(exe, info['path'], [(body, s['jdTdb']) for s in selected for body in range(10)])
        for i, sample in enumerate(selected):
            for body in range(10):
                a = sample['full'][body]['candidate']; b = pack_results[i * 10 + body]['candidate']
                errors = [abs(wrap(a[0] - b[0]))] + [abs(x - y) for x, y in zip(a[1:], b[1:])]
                limits = [1e-8, 1e-8, 1e-12, 1e-8, 1e-8, 1e-12]
                pack_checks.append({'package': package_id, 'label': sample['label'], 'id': body, 'errors': errors,
                                    'overLimit': any(x > y for x, y in zip(errors, limits))})
    dates = sorted({s['jdTdb'] + offset for s in samples for offset in OFFSETS})
    times = engine(exe, kernel, [('tt', date) for date in dates])
    tt_by_tdb = {date: row['jdTt'] for date, row in zip(dates, times)}
    all_refs, sources = {}, []
    for body, (_, command, name) in enumerate(TARGETS):
        for first in range(0, len(dates), 60):
            group = dates[first:first + 60]
            tts = [tt_by_tdb[t] for t in group]
            params = {'format': 'json', 'COMMAND': f"'{command}'", 'OBJ_DATA': "'NO'", 'MAKE_EPHEM': "'YES'",
                      'EPHEM_TYPE': "'OBSERVER'", 'CENTER': "'500@399'", 'TLIST': "'" + ' '.join(f'{t:.9f}' for t in tts) + "'",
                      'TLIST_TYPE': "'JD'", 'TIME_TYPE': "'TT'", 'CAL_FORMAT': "'JD'", 'QUANTITIES': "'20,23,31,45'",
                      'REF_SYSTEM': "'ICRF'", 'ANG_FORMAT': "'DEG'", 'CSV_FORMAT': "'YES'", 'EXTRA_PREC': "'YES'", 'APPARENT': "'AIRLESS'"}
            url = 'https://ssd.jpl.nasa.gov/api/horizons.api?' + urllib.parse.urlencode(params)
            with urllib.request.urlopen(url, timeout=30) as response:
                raw = json.load(response)['result']
            if not re.search(rf'Target body name: {re.escape(name)} \({command}\).*\{{source: DE441\}}', raw) or not re.search(r'Center body name: Earth \(399\).*\{source: DE441\}', raw):
                raise SystemExit('incorrect Horizons metadata')
            header = next((line for line in raw.splitlines() if 'RA_(ICRF-a-app)' in line), '')
            cols = [x.strip() for x in header.split(',')]
            if 'Date_________JDTT' not in cols or 'Atmos refraction: NO (AIRLESS)' not in raw:
                raise SystemExit('wrong Horizons time scale or atmosphere')
            indexes = [cols.index(x) for x in ('delta', 'deldot', 'S-O-T', 'ObsEcLon', 'ObsEcLat', 'RA_(ICRF-a-app)', 'DEC_(ICRF-a-app)')]
            a, b = raw.find('$$SOE'), raw.find('$$EOE')
            if a < 0 or b <= a:
                raise SystemExit('missing reference table')
            rows = list(csv.reader(io.StringIO(raw[a + 5:b].strip())))
            if len(rows) != len(group):
                raise SystemExit('reference row count')
            for tdb, tt, fields in zip(group, tts, rows):
                if not fields[0].strip() or any(not fields[i].strip() for i in indexes) or abs(float(fields[0]) - tt) > 1e-8:
                    raise SystemExit('reference field or epoch mismatch')
                v = [float(fields[i]) for i in indexes]
                if not all(math.isfinite(x) for x in v):
                    raise SystemExit('nonfinite Horizons row')
                lon, lat = erfa_long_ecliptic(v[5], v[6], tt)
                all_refs[(body, tdb)] = {'lon': lon, 'lat': lat, 'range': v[0], 'q20': v[1],
                                         'elongation': v[2], 'q31lon': v[3], 'q31lat': v[4]}
            sources.append({'target': name, 'url': url, 'raw': raw})
    records = []
    for sample in samples:
        for body in range(10):
            center = sample['jdTdb']
            jd = [center + o for o in OFFSETS]
            tt = [tt_by_tdb[t] for t in jd]
            ref = [all_refs[(body, t)] for t in jd]
            candidate = sample['full'][body]['candidate']
            lon0 = ref[3]['lon']
            lons = [lon0 + wrap(r['lon'] - lon0) for r in ref]
            series = [lons, [r['lat'] for r in ref], [r['range'] for r in ref]]
            checks = []
            for h, idx in ((0.02, [1, 2, 4, 5]), (0.04, [0, 1, 5, 6])):
                d = [four_point(x, tt, idx, tt[3]) for x in series]
                checks.append({'h': h, 'referencePerTtDay': d, 'longitudeRateArcsecDay': (candidate[3] - d[0]) * 3600,
                               'latitudeRateArcsecDay': (candidate[4] - d[1]) * 3600, 'rangeRateAuDay': candidate[5] - d[2],
                               'q20MinusRangeDerivativeKmS': ref[3]['q20'] - d[2] * AU_KM / 86400})
            records.append({'sample': sample['label'], 'package': sample.get('package'), 'jdTdb': center,
                            'target': TARGETS[body][2], 'id': body, 'elongationDeg': ref[3]['elongation'],
                            'positionLonArcsec': wrap(candidate[0] - ref[3]['lon']) * 3600,
                            'positionLatArcsec': (candidate[1] - ref[3]['lat']) * 3600,
                            'rangeAu': candidate[2] - ref[3]['range'],
                            'q31LonArcsec': wrap(candidate[0] - ref[3]['q31lon']) * 3600,
                            'q31LatArcsec': (candidate[1] - ref[3]['q31lat']) * 3600, 'checks': checks})
    guards = []
    for sample in samples:
        if sample['label'] in ('lower_inside', 'lower_outside', 'upper_inside', 'upper_outside', 'j2000', 'modern_control'):
            for body, row in enumerate(sample['full']):
                want_ok = sample['label'] not in ('lower_outside', 'upper_outside')
                guard_ok = (row['standardStatus'] == 'ok') if want_ok else (row['standardStatus'] == 'error' and 'Orientation(UnsupportedEpoch' in row.get('standardError', ''))
                diff = None
                if row['standardStatus'] == 'ok':
                    a, b = row['candidate'], row['standard']
                    diff = [wrap(a[0] - b[0]) * 3600, (a[1] - b[1]) * 3600,
                            a[2] - b[2], (a[3] - b[3]) * 3600, (a[4] - b[4]) * 3600, a[5] - b[5]]
                guards.append({'label': sample['label'], 'id': body, 'guardOk': guard_ok, 'candidateMinusStandard': diff})
    def metric(items, key, limit):
        worst = max(items, key=lambda x: abs(x[key]))
        return {'checks': len(items), 'limit': limit, 'failures': None if limit is None else sum(abs(x[key]) > limit for x in items),
                'maxAbs': abs(worst[key]), 'target': worst.get('target'), 'sample': worst.get('sample'), 'package': worst.get('package')}
    rates = [{'target': r['target'], 'sample': r['sample'], 'package': r['package'], **c} for r in records for c in r['checks']]
    summary = {'cases': len(records), 'packChecks': len(pack_checks), 'packFailures': sum(x['overLimit'] for x in pack_checks),
               'guardChecks': len(guards), 'guardFailures': sum(not x['guardOk'] for x in guards),
               'longitudeArcsec': metric(records, 'positionLonArcsec', 1.0), 'latitudeArcsec': metric(records, 'positionLatArcsec', 1.0),
               'rangeAu': metric(records, 'rangeAu', 1e-9), 'longitudeRateArcsecDay': metric(rates, 'longitudeRateArcsecDay', 0.1),
               'latitudeRateArcsecDay': metric(rates, 'latitudeRateArcsecDay', 0.1), 'rangeRateAuDay': metric(rates, 'rangeRateAuDay', 5e-10),
               'q20DirectKmS': metric(rates, 'q20MinusRangeDerivativeKmS', 0.001),
               'q31LonDifferentModelArcsec': metric(records, 'q31LonArcsec', None), 'q31LatDifferentModelArcsec': metric(records, 'q31LatArcsec', None)}
    if sha(kernel) != EXPECTED_SOURCE or sha(manifest) != mh or sha(exe) != eh or any(sha(info['path']) != info['sha256'] for info in files.values()):
        raise SystemExit('input changed')
    result = {'schema': 'package-orientation-horizons/1', 'status': 'EXPLORATORY', 'sourceSha256': EXPECTED_SOURCE,
              'manifestSha256': mh, 'binarySha256': eh, 'erfaVersion': erfa.__version__, 'summary': summary,
              'samples': samples, 'guards': guards, 'packChecks': pack_checks, 'records': records, 'sources': sources}
    with out.open('x', encoding='utf8', newline='\n') as destination:
        json.dump(result, destination, indent=2, allow_nan=False)
        destination.write('\n')
    print(json.dumps(summary))
    keys = ('longitudeArcsec', 'latitudeArcsec', 'rangeAu', 'longitudeRateArcsecDay', 'latitudeRateArcsecDay', 'rangeRateAuDay', 'q20DirectKmS')
    if summary['packFailures'] or summary['guardFailures'] or any(summary[k]['failures'] for k in keys):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
