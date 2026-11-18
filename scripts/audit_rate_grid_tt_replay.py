# SPDX-License-Identifier: CPAL-1.0
"""Offline replay of the prior 90-case JPL grid using requested TT epochs.

Usage: python scripts/audit_rate_grid_tt_replay.py OLD_JSON OLD_SHA256 NEW_JSON
The replay uses the same observations; it is not a new independent holdout.
"""
import csv
import io
import json
import math
import pathlib
import re
import sys
import urllib.parse

from audit_horizons_ecliptic_radial_rates import AU_KM, ecliptic, four_point, sha


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    old, expected, out = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3])
    if not re.fullmatch('[a-f0-9]{64}', expected) or out.exists() or old.resolve() == out.resolve() or sha(old) != expected:
        raise SystemExit('invalid source, hash or output')
    data = json.loads(old.read_text(encoding='utf8'))
    if data.get('schema') != 'horizons-erfa-ecliptic-radial-rates/1':
        raise SystemExit('wrong source schema')
    by_target = {}
    for source in data['sources']:
        params = urllib.parse.parse_qs(urllib.parse.urlparse(source['url']).query)
        tt = [float(x) for x in params['TLIST'][0].strip("'").split()]
        raw = source['raw']
        header = next(line for line in raw.splitlines() if 'RA_(ICRF-a-app)' in line)
        cols = [v.strip() for v in header.split(',')]
        indexes = [cols.index(v) for v in ('delta', 'deldot', 'RA_(ICRF-a-app)', 'DEC_(ICRF-a-app)')]
        a, b = raw.find('$$SOE'), raw.find('$$EOE')
        rows = list(csv.reader(io.StringIO(raw[a + 5:b].strip())))
        if a < 0 or b <= a or len(rows) != 63 or len(tt) != 63:
            raise SystemExit('unexpected source rows')
        ref = []
        for row, epoch in zip(rows, tt):
            if not row[0].strip() or any(not row[i].strip() for i in indexes) or abs(float(row[0]) - epoch) > 1e-8:
                raise SystemExit('invalid reference epoch')
            v = [float(row[i]) for i in indexes]
            if not all(math.isfinite(x) for x in v):
                raise SystemExit('nonfinite value')
            lon, lat = ecliptic(v[2], v[3], epoch)
            ref.append({'lon': lon, 'lat': lat, 'range': v[0], 'q20Rate': v[1]})
        by_target[source['target']] = (tt, ref)
    records = []
    for original in data['records']:
        prior = [row for row in data['records'] if row['target'] == original['target']]
        position = next(i for i, row in enumerate(prior) if row['year'] == original['year'])
        tt_all, ref_all = by_target[original['target']]
        times, ref = tt_all[position * 7:position * 7 + 7], ref_all[position * 7:position * 7 + 7]
        lon0 = ref[3]['lon']
        series = [[lon0 + ((row['lon'] - lon0 + 180) % 360 - 180) for row in ref],
                  [row['lat'] for row in ref], [row['range'] for row in ref]]
        case = dict(original)
        case['stencils'] = []
        for h, points in ((0.02, [1, 2, 4, 5]), (0.04, [0, 1, 5, 6])):
            values = [four_point(x, times, points, times[3]) for x in series]
            got = original['rustRates']
            case['stencils'].append({'h': h, 'referencePerTtDay': values,
                                     'longitudeErrorArcsecDay': (got[0] - values[0]) * 3600,
                                     'latitudeErrorArcsecDay': (got[1] - values[1]) * 3600,
                                     'radialErrorAuDay': got[2] - values[2],
                                     'q20MinusRangeDerivativeKmS': ref[3]['q20Rate'] - values[2] * AU_KM / 86400})
        records.append(case)
    entries = [(r, s) for r in records for s in r['stencils']]
    def metric(key, limit):
        worst = max(entries, key=lambda x: abs(x[1][key]))
        return {'checks': len(entries), 'limit': limit, 'failures': sum(abs(s[key]) > limit for _, s in entries),
                'maxAbs': abs(worst[1][key]), 'target': worst[0]['target'], 'year': worst[0]['year'], 'h': worst[1]['h']}
    summary = {'longitudeArcsecDay': metric('longitudeErrorArcsecDay', 0.05), 'latitudeArcsecDay': metric('latitudeErrorArcsecDay', 0.05),
               'radialAuDay': metric('radialErrorAuDay', 5e-10), 'q20DirectKmS': metric('q20MinusRangeDerivativeKmS', 0.001)}
    if sha(old) != expected:
        raise SystemExit('source changed')
    result = {'schema': 'horizons-erfa-grid-tt-replay/1', 'sourceSha256': expected, 'status': 'EXPLORATORY', 'summary': summary, 'records': records}
    with out.open('x', encoding='utf8', newline='\n') as destination:
        json.dump(result, destination, indent=2, allow_nan=False)
        destination.write('\n')
    print(json.dumps(summary))
    if any(metric['failures'] for metric in summary.values()):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
