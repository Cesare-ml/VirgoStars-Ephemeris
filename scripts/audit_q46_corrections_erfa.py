# SPDX-License-Identifier: CPAL-1.0
"""Exploratory comparison; astronomy routines are supplied by Rust and ERFA.

Run: python scripts/audit_q46_corrections_erfa.py KERNEL SHA256 RUST_EXE NEW_JSON
Network: five instants per case, four batched Horizons requests. Raw replies
are saved only to the caller's new report, never committed or overwritten.
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
C_KM_DAY = 299792.458 * 86400.0
ARCSEC_PER_RAD = 180.0 / math.pi * 3600.0
TARGETS = [(1, '301', 'Moon'), (2, '199', 'Mercury'),
           (3, '299', 'Venus'), (5, '5', 'Jupiter Barycenter')]
JDS = [2451580.0, 2451620.0, 2451674.0, 2451740.0, 2451820.0]
OFFSETS = [-0.01, -0.005, 0.0, 0.005, 0.01]


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(4 * 1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def unit(x):
    return x / np.linalg.norm(x)


def direction(ra, dec):
    return erfa.s2c(math.radians(ra), math.radians(dec))


def main():
    if len(sys.argv) != 5:
        raise SystemExit(__doc__)
    kernel, expected, executable, out = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3]), pathlib.Path(sys.argv[4])
    if not re.fullmatch('[a-f0-9]{64}', expected) or out.exists() or len({kernel.resolve(), executable.resolve(), out.resolve()}) != 3:
        raise SystemExit('invalid arguments or existing output')
    if sha(kernel) != expected:
        raise SystemExit('kernel hash mismatch')
    binary_hash = sha(executable)
    cases = [(body, jd + offset) for body, _, _ in TARGETS for jd in JDS for offset in OFFSETS]
    inputs = ''.join(f'{body},{jd:.9f}\n' for body, jd in cases)
    proc = subprocess.run([str(executable), str(kernel)], input=inputs, text=True, capture_output=True, check=True)
    parsed = list(csv.reader(io.StringIO(proc.stdout)))
    if len(parsed) != len(cases):
        raise SystemExit('unexpected Rust row count')
    geometry = {}
    for fields, (body, jd) in zip(parsed, cases):
        if len(fields) != 27 or int(fields[0]) != body or abs(float(fields[1]) - jd) > 1e-8:
            raise SystemExit('unexpected Rust output')
        x = np.array([float(v) for v in fields[3:]], dtype=float).reshape(8, 3)
        if not np.isfinite(x).all():
            raise SystemExit('non-finite Rust state')
        p = unit(x[0]); evec = x[3] - x[5]; e = unit(evec); q = unit(x[6] - x[7])
        sun_au = np.linalg.norm(evec) / AU_KM
        velocity = x[4] / C_KM_DAY
        erfa_def = unit(erfa.ld(1.0, p, q, e, sun_au, 1e-6))
        erfa_app = unit(erfa.ab(erfa_def, velocity, sun_au, math.sqrt(1.0 - np.dot(velocity, velocity))))
        geometry[(body, jd)] = {'tt': float(fields[2]), 'p': p, 'rust_def': unit(x[1]), 'rust_app': unit(x[2]),
                                'erfa_def': erfa_def, 'erfa_app': erfa_app,
                                'elongation': math.degrees(math.acos(float(np.clip(-np.dot(p, e), -1.0, 1.0)))),
                                'denominator': float(np.dot(q, q + e))}
    records, sources = [], []
    for body, command, name in TARGETS:
        epochs = [geometry[(body, jd + offset)]['tt'] for jd in JDS for offset in OFFSETS]
        params = {'format': 'json', 'COMMAND': f"'{command}'", 'OBJ_DATA': "'NO'", 'MAKE_EPHEM': "'YES'",
                  'EPHEM_TYPE': "'OBSERVER'", 'CENTER': "'500@399'", 'TLIST': "'" + ' '.join(f'{x:.9f}' for x in epochs) + "'",
                  'TLIST_TYPE': "'JD'", 'TIME_TYPE': "'TT'", 'CAL_FORMAT': "'JD'", 'QUANTITIES': "'1,45,46'",
                  'REF_SYSTEM': "'ICRF'", 'ANG_FORMAT': "'DEG'", 'CSV_FORMAT': "'YES'", 'EXTRA_PREC': "'YES'", 'APPARENT': "'AIRLESS'"}
        url = 'https://ssd.jpl.nasa.gov/api/horizons.api?' + urllib.parse.urlencode(params)
        with urllib.request.urlopen(url, timeout=30) as response:
            raw = json.load(response)['result']
        if not re.search(rf'Target body name: {re.escape(name)} \({command}\).*\{{source: DE441\}}', raw) or not re.search(r'Center body name: Earth \(399\).*\{source: DE441\}', raw):
            raise SystemExit('unexpected Horizons metadata')
        header = next((line for line in raw.splitlines() if 'RA_(ICRF-a-app)' in line), '')
        cols = [x.strip() for x in header.split(',')]
        if 'Date_________JDTT' not in cols:
            raise SystemExit('unexpected time scale')
        indices = [cols.index(x) for x in ('R.A.___(ICRF)', 'DEC____(ICRF)', 'RA_(ICRF-a-app)', 'DEC_(ICRF-a-app)', 'I_dRA*cosD', 'I_d(DEC)/dt')]
        a, b = raw.find('$$SOE'), raw.find('$$EOE')
        if a < 0 or b <= a:
            raise SystemExit('missing ephemeris data')
        rows = list(csv.reader(io.StringIO(raw[a + 5:b].strip())))
        if len(rows) != len(epochs):
            raise SystemExit('unexpected Horizons row count')
        ref = []
        for row, epoch in zip(rows, epochs):
            if not row[0].strip() or any(not row[i].strip() for i in indices) or abs(float(row[0]) - epoch) > 1e-8:
                raise SystemExit('missing value or wrong epoch')
            v = [float(row[i]) for i in indices]
            if not all(math.isfinite(z) for z in v):
                raise SystemExit('non-finite reference')
            ref.append({'q1': direction(v[0], v[1]), 'q45': direction(v[2], v[3]), 'q46': v[4:6],
                        'q1ra': v[0], 'q1dec': v[1], 'q45ra': v[2], 'q45dec': v[3]})
        sources.append({'target': name, 'url': url, 'raw': raw})
        for k, jd in enumerate(JDS):
            group = ref[k * 5:k * 5 + 5]
            g = [geometry[(body, jd + offset)] for offset in OFFSETS]
            ra, dec = map(math.radians, (group[2]['q1ra'], group[2]['q1dec']))
            basis = np.array([[-math.sin(ra), math.cos(ra), 0.0],
                              [-math.cos(ra) * math.sin(dec), -math.sin(ra) * math.sin(dec), math.cos(dec)]])
            ara, adec = map(math.radians, (group[2]['q45ra'], group[2]['q45dec']))
            apparent_basis = np.array([[-math.sin(ara), math.cos(ara), 0.0],
                                       [-math.cos(ara) * math.sin(adec), -math.sin(ara) * math.sin(adec), math.cos(adec)]])
            rates = []
            for i in (0, 1):
                j = 4 - i
                factor = ARCSEC_PER_RAD / ((jd + OFFSETS[j]) - (jd + OFFSETS[i])) / 24.0
                def projected_difference(plus, minus):
                    return basis @ (plus - minus) * factor
                def correction(field, before):
                    return projected_difference(g[j][field] - g[j][before], g[i][field] - g[i][before])
                erfa_def = correction('erfa_def', 'p'); erfa_ab = correction('erfa_app', 'erfa_def')
                rust_def = correction('rust_def', 'p'); rust_ab = correction('rust_app', 'rust_def')
                horizons_total = projected_difference(group[j]['q45'] - group[j]['q1'], group[i]['q45'] - group[i]['q1'])
                apparent_rate = apparent_basis @ (group[j]['q45'] - group[i]['q45']) * factor
                rates.append({'halfStepDays': abs(OFFSETS[i]), 'erfaDef': erfa_def.tolist(), 'erfaAb': erfa_ab.tolist(),
                              'rustDef': rust_def.tolist(), 'rustAb': rust_ab.tolist(), 'horizonsTotal': horizons_total.tolist(),
                              'apparentRate': apparent_rate.tolist(), 'q46': group[2]['q46']})
            records.append({'target': name, 'jdTdb': jd, 'jdTt': g[2]['tt'], 'elongationDeg': g[2]['elongation'],
                            'denominator': g[2]['denominator'], 'rates': rates})
    if sha(kernel) != expected or sha(executable) != binary_hash:
        raise SystemExit('input changed')
    def difference(a, b):
        return max(abs(x - y) for x, y in zip(a, b))
    rows = [rate for record in records for rate in record['rates']]
    errors = {
        'deflection': [difference(r['erfaDef'], r['rustDef']) for r in rows],
        'aberration': [difference(r['erfaAb'], r['rustAb']) for r in rows],
        'horizonsTotal': [difference(r['horizonsTotal'], [a + b for a, b in zip(r['erfaDef'], r['erfaAb'])]) for r in rows],
        'stepSensitivity': [max(difference(r['rates'][0][key], r['rates'][1][key])
                                for key in ('erfaDef', 'erfaAb', 'horizonsTotal')) for r in records],
    }
    limits = {'deflection': 0.0005, 'aberration': 0.0005, 'horizonsTotal': 0.005, 'stepSensitivity': 0.001}
    summary = {key: {'count': len(values), 'limitArcsecHour': limits[key], 'maxArcsecHour': max(values),
                     'failures': sum(value > limits[key] for value in values)} for key, values in errors.items()}
    doc = {'schema': 'q46-erfa-separated-corrections/1', 'status': 'EXPLORATORY', 'capturedUtc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
           'kernelSha256': expected, 'binarySha256': binary_hash, 'erfaVersion': erfa.__version__, 'summary': summary,
           'records': records, 'sources': sources}
    with out.open('x', encoding='utf8', newline='\n') as f:
        json.dump(doc, f, indent=2, allow_nan=False)
        f.write('\n')
    print(json.dumps({'records': len(records), 'output': str(out), 'erfaVersion': erfa.__version__, 'summary': summary}))
    if any(item['failures'] for item in summary.values()):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
