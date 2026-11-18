# SPDX-License-Identifier: CPAL-1.0
"""Replay saved JPL samples with finite differences at requested TT epochs.

Usage: python scripts/audit_station_tt_replay.py KERNEL SHA256 RUST_EXE INPUT_JSON INPUT_SHA256 NEW_JSON
This is an offline reanalysis of identical observations, not a new holdout.
"""
import csv
import io
import json
import math
import pathlib
import re
import sys
import urllib.parse

from audit_horizons_ecliptic_radial_rates import AU_KM, TARGETS, ecliptic, four_point, sha
from audit_station_boundary_rates import rust, sign


def main():
    if len(sys.argv) != 7:
        raise SystemExit(__doc__)
    kernel, expected, exe, input_file, input_hash, out = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3]), pathlib.Path(sys.argv[4]), sys.argv[5], pathlib.Path(sys.argv[6])
    if not all(re.fullmatch('[a-f0-9]{64}', h) for h in (expected, input_hash)) or out.exists() or len({p.resolve() for p in (kernel, exe, input_file, out)}) != 4:
        raise SystemExit('invalid inputs')
    if sha(kernel) != expected or sha(input_file) != input_hash:
        raise SystemExit('input hash mismatch')
    exe_hash = sha(exe)
    data = json.loads(input_file.read_text(encoding='utf8'))
    if data.get('schema') != 'de441-station-boundary-rates/1' or data.get('kernelSha256') != expected:
        raise SystemExit('invalid source report')
    names = {entry[2]: i for i, entry in enumerate(TARGETS)}
    reference = {}
    for source in data['sources']:
        body = names[source['target']]
        params = urllib.parse.parse_qs(urllib.parse.urlparse(source['url']).query)
        tts = [float(x) for x in params['TLIST'][0].strip("'").split()]
        raw = source['raw']
        header = next(line for line in raw.splitlines() if 'RA_(ICRF-a-app)' in line)
        cols = [x.strip() for x in header.split(',')]
        indexes = [cols.index(x) for x in ('delta', 'deldot', 'RA_(ICRF-a-app)', 'DEC_(ICRF-a-app)')]
        a, b = raw.find('$$SOE'), raw.find('$$EOE')
        rows = list(csv.reader(io.StringIO(raw[a + 5:b].strip())))
        if a < 0 or b <= a or len(rows) != len(tts):
            raise SystemExit('bad retained response')
        for fields, tt in zip(rows, tts):
            if not fields[0].strip() or any(not fields[i].strip() for i in indexes) or abs(float(fields[0]) - tt) > 1e-8:
                raise SystemExit('bad retained TT or field')
            v = [float(fields[i]) for i in indexes]
            if not all(math.isfinite(x) for x in v):
                raise SystemExit('nonfinite retained field')
            lon, lat = ecliptic(v[2], v[3], tt)
            key = (body, f'{tt:.9f}')
            if key in reference:
                raise SystemExit('ambiguous reference TT')
            reference[key] = {'lon': lon, 'lat': lat, 'range': v[0], 'q20Rate': v[1]}
    cases = data['cases']
    time_inputs = [('tt', case['jdTdb'] + offset) for case in cases for offset in case['offsets']]
    converted = rust(exe, kernel, time_inputs)
    replayed = []
    for k, original in enumerate(cases):
        case = dict(original)
        tt_values = [entry['jdTt'] for entry in converted[k * 7:k * 7 + 7]]
        group = [reference[(case['id'], f'{tt:.9f}')] for tt in tt_values]
        lon0 = group[3]['lon']
        values = [[lon0 + ((row['lon'] - lon0 + 180.0) % 360.0 - 180.0) for row in group],
                  [row['lat'] for row in group], [row['range'] for row in group]]
        got = case['rust']['fields'][3:6]
        rates = []
        for points in ([1, 2, 4, 5], [0, 1, 5, 6]):
            d = [four_point(series, tt_values, points, tt_values[3]) for series in values]
            rates.append({'h': abs(case['offsets'][points[1]]), 'referencePerTtDay': d,
                          'longitudeErrorArcsecDay': (got[0] - d[0]) * 3600,
                          'latitudeErrorArcsecDay': (got[1] - d[1]) * 3600,
                          'radialErrorAuDay': got[2] - d[2],
                          'q20MinusDerivativeKmS': group[3]['q20Rate'] - d[2] * AU_KM / 86400.0,
                          'rustSign': sign(got[0] * 3600), 'referenceSign': sign(d[0] * 3600)})
        case['requestedTt'] = tt_values
        case['rates'] = rates
        replayed.append(case)
    summary = {}
    for kind in ('station', 'boundary'):
        cases_kind = [c for c in replayed if c['kind'] == kind]
        entries = [(c, s) for c in cases_kind for s in c['rates']]
        def metric(key, limit):
            worst = max(entries, key=lambda pair: abs(pair[1][key]))
            return {'limit': limit, 'checks': len(entries), 'failures': sum(abs(s[key]) > limit for _, s in entries),
                    'maxAbs': abs(worst[1][key]), 'id': worst[0]['id'], 'jdTdb': worst[0]['jdTdb'], 'h': worst[1]['h']}
        summary[kind] = {'cases': len(cases_kind), 'longitudeArcsecDay': metric('longitudeErrorArcsecDay', 0.05),
                         'latitudeArcsecDay': metric('latitudeErrorArcsecDay', 0.05),
                         'radialAuDay': metric('radialErrorAuDay', 5e-10),
                         'q20DirectKmS': metric('q20MinusDerivativeKmS', 0.001),
                         'signMismatches': sum(s['rustSign'] * s['referenceSign'] == -1 for _, s in entries),
                         'signIndeterminate': sum(not s['rustSign'] or not s['referenceSign'] for _, s in entries)}
    windows = {}
    for hours in (1, 6):
        for stencil in (0, 1):
            counts = {'confirmed': 0, 'indeterminate': 0, 'contradicted': 0}
            for index, event in enumerate(data['events']):
                pair = [next(c for c in replayed if c['kind'] == 'station' and c['event'] == index and c['hours'] == side * hours) for side in (-1, 1)]
                expected_signs = (1, -1) if event['kind'] == 'direct_to_retrograde' else (-1, 1)
                signs = [(case['rates'][stencil]['rustSign'], case['rates'][stencil]['referenceSign']) for case in pair]
                if any(v and v != expected_sign for values, expected_sign in zip(signs, expected_signs) for v in values):
                    counts['contradicted'] += 1
                elif all(v for values in signs for v in values):
                    counts['confirmed'] += 1
                else:
                    counts['indeterminate'] += 1
            windows[f'{hours}h_stencil{stencil}'] = counts
    summary['windows'] = windows
    if sha(kernel) != expected or sha(input_file) != input_hash or sha(exe) != exe_hash:
        raise SystemExit('input changed')
    result = {'schema': 'de441-station-boundary-tt-replay/1', 'status': 'EXPLORATORY', 'kernelSha256': expected,
              'inputSha256': input_hash, 'binarySha256': exe_hash, 'summary': summary, 'cases': replayed}
    with out.open('x', encoding='utf8', newline='\n') as destination:
        json.dump(result, destination, indent=2, allow_nan=False)
        destination.write('\n')
    print(json.dumps(summary))
    if any(summary[k][m]['failures'] for k in ('station', 'boundary') for m in ('longitudeArcsecDay', 'latitudeArcsecDay', 'radialAuDay', 'q20DirectKmS')) or any(v['contradicted'] for v in windows.values()):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
