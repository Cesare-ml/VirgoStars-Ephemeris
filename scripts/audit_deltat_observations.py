# SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
"""Compare the existing Rust ΔT model with external observations and constraints.

python scripts/audit_deltat_observations.py RUST_EXE USNO_DIR NEW_REPORT.json
Requires pyerfa and a Horizons connection. No astronomical model is duplicated.
"""
import csv
import datetime as dt
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import urllib.parse
import urllib.request

import erfa


AS_OF = dt.date(2026, 11, 18)
SOURCES = {
    'historic_deltat.data': 'https://maia.usno.navy.mil/ser7/historic_deltat.data',
    'deltat.data': 'https://maia.usno.navy.mil/ser7/deltat.data',
    'deltat.preds': 'https://maia.usno.navy.mil/ser7/deltat.preds',
}
EVENTS = [
    (1361, 5, [[-408.0, 601.0], [1865.0, 18532.0]]),
    (1514, 8, [[277.0, 890.0]]),
    (1542, 8, [[-328.0, 332.0]]),
    (1575, 5, [[-1762.0, 1091.0]]),
]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    exe, sources, out = map(lambda x: Path(x).resolve(), sys.argv[1:])
    if out.exists() or out == exe or sources == out or sources in out.parents:
        raise SystemExit('report must be a new file outside the input directory')
    files = [exe] + [sources / n for n in SOURCES]
    before = {str(x): sha(x) for x in files}
    historic = []
    for line in (sources / 'historic_deltat.data').read_text().splitlines():
        parts = line.split()
        if len(parts) != 5:
            continue
        try:
            year, value, error = map(float, parts[:3])
        except ValueError:
            continue
        if 1657 <= year < 1972:
            historic.append({'year': year, 'reference': value, 'referenceError': error})
    monthly = []
    for line in (sources / 'deltat.data').read_text().splitlines():
        parts = line.split()
        if len(parts) != 4:
            raise ValueError('unexpected monthly row')
        year, month, day = map(int, parts[:3])
        date = dt.date(year, month, day)
        if date > AS_OF:
            continue
        begin = dt.date(year, 1, 1)
        end = dt.date(year + 1, 1, 1)
        decimal = year + (date - begin).days / (end - begin).days
        monthly.append({'date': date.isoformat(), 'year': decimal, 'reference': float(parts[3])})
    predictions = []
    for line in (sources / 'deltat.preds').read_text().splitlines()[1:]:
        fields = line.split()
        if len(fields) < 4:
            continue
        mjd, year, value = map(float, fields[:3])
        if 2027.0 <= year <= 2033.75:
            predictions.append({'mjd': mjd, 'year': year, 'reference': value, 'reportedError': float(fields[-1])})
    if len(historic) < 500 or len(monthly) < 600 or not predictions:
        raise ValueError('unexpected source coverage')
    events = []
    for year, month, intervals in EVENTS:
        midpoint = year + (month - 0.5) / 12.0
        events.append({'year': year, 'month': month, 'decimalYear': midpoint,
                       'intervals': intervals,
                       'probeYears': [year + (month - 1) / 12.0, midpoint, year + month / 12.0]})
    years = list(range(0, 1901, 100)) + [1950, 1961]
    epochs = [float(sum(erfa.cal2jd(y, 7, 1)) + 0.5) for y in years]
    # July 1 noon TT; using that date's Gregorian year fraction differs from
    # its actual UT1 by at most several hours for these epochs, not months.
    jpl_decimal = [y + (181 + int(y % 4 == 0 and (y % 100 != 0 or y % 400 == 0)) + 0.5) /
                   (365 + int(y % 4 == 0 and (y % 100 != 0 or y % 400 == 0))) for y in years]
    inputs = [x['year'] for x in historic + monthly + predictions]
    inputs += [y for e in events for y in e['probeYears']] + jpl_decimal
    run = subprocess.run([str(exe)], input=''.join(f'{y:.12f}\n' for y in inputs),
                         capture_output=True, text=True, check=True)
    rows = [line.split(',') for line in run.stdout.splitlines()]
    if len(rows) != len(inputs):
        raise ValueError('wrong Rust output count')
    evaluated = []
    for requested, fields in zip(inputs, rows):
        got, value = map(float, fields)
        if abs(got - requested) > 1e-9:
            raise ValueError('Rust output year mismatch')
        evaluated.append(value)
    values = iter(evaluated)
    for row in historic + monthly + predictions:
        row['model'] = next(values)
        row['residual'] = row['model'] - row['reference']
    for row in events:
        row['probeModels'] = [next(values) for _ in row['probeYears']]
        row['model'] = row['probeModels'][1]
        row['inPublishedInterval'] = any(a <= row['model'] <= b for a, b in row['intervals'])
    params = {'format': 'json', 'COMMAND': "'10'", 'OBJ_DATA': "'NO'", 'MAKE_EPHEM': "'YES'",
              'EPHEM_TYPE': "'OBSERVER'", 'CENTER': "'500@399'", 'TLIST': "'" + ' '.join(f'{x:.9f}' for x in epochs) + "'",
              'TLIST_TYPE': "'JD'", 'TIME_TYPE': "'TT'", 'CAL_FORMAT': "'JD'", 'QUANTITIES': "'30'",
              'CSV_FORMAT': "'YES'", 'EXTRA_PREC': "'YES'"}
    url = 'https://ssd.jpl.nasa.gov/api/horizons.api?' + urllib.parse.urlencode(params)
    with urllib.request.urlopen(url, timeout=30) as response:
        raw = json.load(response)['result']
    if 'Date_________JDTT' not in raw or 'TDB-UT' not in raw or '$$SOE' not in raw or '$$EOE' not in raw:
        raise ValueError('unexpected Horizons metadata')
    header = next(x for x in raw.splitlines() if 'Date_________JDTT' in x and 'TDB-UT' in x)
    names = [x.strip() for x in header.split(',')]
    index = names.index('TDB-UT')
    table = raw.split('$$SOE', 1)[1].split('$$EOE', 1)[0].strip()
    refrows = list(csv.reader(io.StringIO(table)))
    if len(refrows) != len(epochs):
        raise ValueError('wrong Horizons row count')
    jpl = []
    for y, epoch, row in zip(jpl_decimal, epochs, refrows):
        if abs(float(row[0]) - epoch) > 1e-8:
            raise ValueError('Horizons epoch mismatch')
        model, ref = next(values), float(row[index])
        jpl.append({'decimalYear': y, 'jdTt': epoch, 'reference': ref, 'model': model, 'residual': model-ref})
    if before != {str(x): sha(x) for x in files}:
        raise ValueError('source changed during audit')

    def stats(group, limit):
        worst = max(group, key=lambda x: abs(x['residual']))
        return {'count': len(group), 'threshold': limit, 'failures': sum(abs(x['residual']) > limit for x in group),
                'maxAbs': abs(worst['residual']), 'worst': worst}
    old = [x for x in monthly if x['date'] < '2005-01-01']
    future = [x for x in monthly if x['date'] >= '2005-01-01']
    with_error = [x for x in historic if x['referenceError'] > 0]
    summary = {'historic': {'count': len(historic), 'outsideReportedError': sum(abs(x['residual']) > x['referenceError'] for x in with_error),
                            'withNonzeroError': len(with_error), 'maxAbs': max(abs(x['residual']) for x in historic)},
               'monthly1973to2004': stats(old, 0.5), 'monthly2005on': stats(future, 1.0),
               'events': events, 'horizons': {'count': len(jpl), 'maxAbs': max(abs(x['residual']) for x in jpl)}}
    report = {'schema': 'de441-deltat-reference/1', 'status': 'EXPERIMENTAL', 'asOf': AS_OF.isoformat(),
              'sourceHashes': before, 'sourceUrls': SOURCES, 'erfaVersion': erfa.__version__,
              'summary': summary, 'historic': historic, 'monthly': monthly, 'predictions': predictions,
              'events': events, 'horizonsUrl': url, 'horizonsRaw': raw, 'horizons': jpl}
    with out.open('x', encoding='utf8', newline='\n') as f:
        json.dump(report, f, indent=2, allow_nan=False)
        f.write('\n')
    print(json.dumps(summary))
    if summary['monthly1973to2004']['failures'] or summary['monthly2005on']['failures'] or not all(x['inPublishedInterval'] for x in events):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
